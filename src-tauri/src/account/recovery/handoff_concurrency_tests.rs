//! A controlled vault read exposes the race between recovery and a new grant.
use super::*;
use crate::account::pending_login::PendingLoginGrants;
use crate::auth::microsoft::MicrosoftDeviceCode;
use std::sync::{mpsc, TryLockError};

struct PausedGrantRead {
    inner: Store,
    key: String,
    reached: Mutex<Option<mpsc::Sender<()>>>,
    release: Mutex<mpsc::Receiver<()>>,
}

impl CredentialStore for PausedGrantRead {
    fn read(&self, id: &str) -> Result<Option<String>, CredentialError> {
        let value = self.inner.read(id)?;
        if id == self.key {
            let reached = self.reached.lock().unwrap().take();
            if let Some(reached) = reached {
                reached.send(()).unwrap();
                self.release.lock().unwrap().recv().unwrap();
            }
        }
        Ok(value)
    }

    fn write(&self, id: &str, value: &str) -> Result<(), CredentialError> {
        self.inner.write(id, value)
    }

    fn delete(&self, id: &str) -> Result<(), CredentialError> {
        self.inner.delete(id)
    }
}

#[test]
fn concurrent_new_grant_waits_for_recovery_acknowledgement_and_survives_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    let repository = AccountRepository::new(path.clone());
    let store = store(path.clone());
    let grants = PendingLoginGrants::new(&path);
    let code = MicrosoftDeviceCode {
        user_code: "fixture-user".into(),
        verification_uri: "https://fixture.invalid/link".into(),
        device_code: "private-device".into(),
        interval: 1,
        expires_in: 900,
    };
    let receipt = grants
        .checkpoint(&store, &code, "previous-refresh", 1000)
        .unwrap();
    let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
    let intent = Intent {
        version: 2,
        change: ProfileChange::Activate {
            profile: profile("microsoft:test"),
        },
        secrets: Some(secrets()),
        grant: Some(receipt),
    };
    recovery.create_marker().unwrap();
    store
        .write(&recovery.key, &serde_json::to_string(&[intent]).unwrap())
        .unwrap();
    let scope = uuid::Uuid::new_v3(
        &uuid::Uuid::NAMESPACE_URL,
        path.to_string_lossy().as_bytes(),
    );
    let key = format!("ingot:microsoft-pending:v1:{scope}");
    let (reached, wait_for_read) = mpsc::channel();
    let (release, wait_for_release) = mpsc::channel();
    let store = PausedGrantRead {
        inner: store,
        key,
        reached: Mutex::new(Some(reached)),
        release: Mutex::new(wait_for_release),
    };
    std::thread::scope(|threads| {
        let replay =
            threads.spawn(|| recovery.with_recovered(&store, &repository, || repository.load()));
        wait_for_read.recv().unwrap();
        // Recovery has read the old ticket but not acknowledged it yet. Grant
        // mutation must remain excluded throughout that read/write interval.
        let held = matches!(
            recovery.pending_login_lock.try_lock(),
            Err(TryLockError::WouldBlock)
        );
        let new_sign_in = threads.spawn(|| {
            recovery.with_pending_login(|| {
                grants.discard(&store)?;
                grants.checkpoint(&store, &code, "next-refresh", 1001)?;
                Ok::<(), RecoveryError>(())
            })
        });
        release.send(()).unwrap();
        assert_eq!(replay.join().unwrap().unwrap()[0].id, "microsoft:test");
        new_sign_in.join().unwrap().unwrap();
        assert!(
            held,
            "a new sign-in must not interleave the handoff read/write"
        );
    });
    let next = store.read(&store.key).unwrap().unwrap();
    assert!(next.contains("next-refresh"));
    assert!(!next.contains("previous-refresh"));
    assert_eq!(repository.load().unwrap().len(), 1);
}
