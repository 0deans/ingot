use super::*;
use crate::account::credentials::{tests::FakeCredentialStore, VaultFailure};
use std::sync::atomic::{AtomicBool, Ordering};

fn code() -> MicrosoftDeviceCode {
    MicrosoftDeviceCode {
        user_code: "user-code".into(),
        verification_uri: "https://fixture.invalid/link".into(),
        device_code: "secret-device-code".into(),
        interval: 1,
        expires_in: 900,
    }
}

#[test]
fn checkpoint_survives_reconstruction_without_creating_files_or_crossing_scopes() {
    let directory = tempfile::tempdir().unwrap();
    let scope = directory.path().join("accounts.json");
    let store = FakeCredentialStore::default();
    PendingLoginGrants::new(&scope)
        .checkpoint(&store, &code(), "secret-refresh", 1000)
        .unwrap();
    let recovered = PendingLoginGrants::new(&scope)
        .read(&store)
        .unwrap()
        .unwrap();
    assert_eq!(recovered.refresh_token, "secret-refresh");
    assert_eq!(recovered.code.device_code, "secret-device-code");
    assert_eq!(recovered.created_at, 1000);
    assert!(!recovered.ticket.is_nil());
    assert!(
        PendingLoginGrants::new(&directory.path().join("other.json"))
            .read(&store)
            .unwrap()
            .is_none()
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn malformed_or_unknown_records_are_preserved_and_errors_exclude_secrets() {
    let store = FakeCredentialStore::default();
    let grants = PendingLoginGrants::new(Path::new("fixture/accounts.json"));
    grants
        .checkpoint(&store, &code(), "secret-refresh", 1000)
        .unwrap();
    let valid = store.read(&grants.key).unwrap().unwrap();
    for value in [
        "secret-refresh invalid-json".to_owned(),
        valid.replace("\"version\":2", "\"version\":9"),
        valid.replace("AwaitingExchange", "Legacy"),
        valid.replace("AwaitingExchange", "secret-refresh"),
        valid.replace("\"version\":2", "\"version\":1"),
        valid.replace("secret-refresh", ""),
        valid.replace("\"created_at\":1000", "\"unexpected\":1000"),
    ] {
        store.put(&grants.key, &value);
        let error = grants.discard(&store).unwrap_err();
        assert!(matches!(error, PendingGrantError::Invalid));
        assert!(!format!("{error:?}: {error}").contains("secret-refresh"));
        assert_eq!(store.read(&grants.key).unwrap().unwrap(), value);
    }
}

#[test]
fn legacy_checkpoint_is_preserved_until_explicit_disposal() {
    let store = FakeCredentialStore::default();
    let grants = PendingLoginGrants::new(Path::new("fixture/accounts.json"));
    let receipt = grants
        .checkpoint(&store, &code(), "secret-refresh", 1000)
        .unwrap();
    let value = store
        .read(&grants.key)
        .unwrap()
        .unwrap()
        .replace("\"version\":2", "\"version\":1")
        .replace("\"state\":\"AwaitingExchange\",", "");
    store.put(&grants.key, &value);
    let record = grants.read(&store).unwrap().unwrap();
    assert!(record.state == GrantState::Legacy);
    assert!(matches!(
        PendingLoginGrants::acknowledge_transfer(&store, receipt),
        Err(PendingGrantError::Invalid)
    ));
    assert_eq!(store.read(&grants.key).unwrap().unwrap(), value);
    grants.discard(&store).unwrap();
    assert!(store.read(&grants.key).unwrap().is_none());
}

#[test]
fn transfer_is_idempotent_and_never_consumes_a_newer_ticket() {
    let store = FakeCredentialStore::default();
    let grants = PendingLoginGrants::new(Path::new("fixture/accounts.json"));
    let first = grants
        .checkpoint(&store, &code(), "first-refresh", 1000)
        .unwrap();
    PendingLoginGrants::acknowledge_transfer(&store, first).unwrap();
    PendingLoginGrants::acknowledge_transfer(&store, first).unwrap();
    assert!(grants.read(&store).unwrap().is_none());
    grants
        .checkpoint(&store, &code(), "next-refresh", 1001)
        .unwrap();
    let next = store.read(&grants.key).unwrap().unwrap();
    PendingLoginGrants::acknowledge_transfer(&store, first).unwrap();
    assert_eq!(store.read(&grants.key).unwrap().unwrap(), next);
}

struct CleanupFailure {
    store: FakeCredentialStore,
    fail_delete: AtomicBool,
}
impl CredentialStore for CleanupFailure {
    fn read(&self, id: &str) -> Result<Option<String>, CredentialError> {
        self.store.read(id)
    }
    fn write(&self, id: &str, value: &str) -> Result<(), CredentialError> {
        self.store.write(id, value)
    }
    fn delete(&self, id: &str) -> Result<(), CredentialError> {
        if self.fail_delete.load(Ordering::SeqCst) {
            return Err(CredentialError::Vault {
                operation: "delete",
                source: VaultFailure("injected cleanup failure".into()),
            });
        }
        self.store.delete(id)
    }
}

#[test]
fn disposal_is_acknowledged_before_chunk_cleanup_and_can_be_retried() {
    let store = CleanupFailure {
        store: FakeCredentialStore::default(),
        fail_delete: AtomicBool::new(true),
    };
    let scope = Path::new("fixture/accounts.json");
    let grants = PendingLoginGrants::new(scope);
    grants
        .checkpoint(&store, &code(), "secret-refresh", 1000)
        .unwrap();
    assert!(matches!(
        grants.discard(&store),
        Err(PendingGrantError::Credentials(_))
    ));
    assert_eq!(store.read(&grants.key).unwrap().unwrap(), DISCARDED);
    let reconstructed = PendingLoginGrants::new(scope);
    assert!(reconstructed.read(&store).unwrap().is_none());
    store.fail_delete.store(false, Ordering::SeqCst);
    reconstructed.discard(&store).unwrap();
    assert!(store.read(&grants.key).unwrap().is_none());
}

#[test]
fn failed_checkpoint_and_unavailable_read_are_reported_without_overwriting_pending_work() {
    let grants = PendingLoginGrants::new(Path::new("fixture/accounts.json"));
    let store = FakeCredentialStore::default();
    store.fail_writes();
    assert!(matches!(
        grants.checkpoint(&store, &code(), "secret-refresh", 1000),
        Err(PendingGrantError::Credentials(_))
    ));
    assert!(grants.read(&store).unwrap().is_none());
    let store = FakeCredentialStore::default();
    grants
        .checkpoint(&store, &code(), "secret-refresh", 1000)
        .unwrap();
    assert!(matches!(
        grants.checkpoint(&store, &code(), "other-refresh", 1001),
        Err(PendingGrantError::AlreadyPending)
    ));
    assert_eq!(
        grants.read(&store).unwrap().unwrap().refresh_token,
        "secret-refresh"
    );
    store.fail_reads();
    assert!(matches!(
        grants.discard(&store),
        Err(PendingGrantError::Credentials(_))
    ));
}

#[test]
fn resume_rotation_rejects_wrong_scopes_old_tickets_and_empty_tokens_without_overwriting() {
    let store = FakeCredentialStore::default();
    let grants = PendingLoginGrants::new(Path::new("fixture/accounts.json"));
    let other = PendingLoginGrants::new(Path::new("fixture/other.json"));
    let receipt = grants
        .checkpoint(&store, &code(), "first-refresh", 1000)
        .unwrap();
    let unrelated = other
        .checkpoint(&store, &code(), "other-refresh", 1000)
        .unwrap();
    let original = store.read(&grants.key).unwrap();
    assert!(matches!(
        grants.rotate(&store, unrelated, "wrong-refresh"),
        Err(PendingGrantError::NotFound)
    ));
    assert!(matches!(
        grants.rotate(&store, receipt, ""),
        Err(PendingGrantError::Invalid)
    ));
    assert_eq!(store.read(&grants.key).unwrap(), original);
    grants.discard(&store).unwrap();
    grants
        .checkpoint(&store, &code(), "next-refresh", 1001)
        .unwrap();
    let next = store.read(&grants.key).unwrap();
    assert!(matches!(
        grants.rotate(&store, receipt, "stale-refresh"),
        Err(PendingGrantError::NotFound)
    ));
    assert_eq!(store.read(&grants.key).unwrap(), next);
}

#[test]
fn inspection_and_resume_report_corrupt_or_overflowing_metadata_without_erasing_it() {
    let store = FakeCredentialStore::default();
    let grants = PendingLoginGrants::new(Path::new("fixture/accounts.json"));
    let receipt = grants
        .checkpoint(&store, &code(), "private-refresh", 1000)
        .unwrap();
    let original = store.read(&grants.key).unwrap().unwrap();
    for invalid in [
        "private-refresh malformed".to_owned(),
        original.replace(
            "\"created_at\":1000",
            &format!("\"created_at\":{}", u64::MAX),
        ),
    ] {
        store.put(&grants.key, &invalid);
        assert!(matches!(
            grants.inspect(&store, 1000),
            Err(PendingGrantError::Invalid)
        ));
        assert!(matches!(
            grants.resume_grant(&store, &receipt.ticket().to_string(), 1000),
            Err(PendingGrantError::Invalid)
        ));
        assert_eq!(
            store.read(&grants.key).unwrap().as_deref(),
            Some(invalid.as_str())
        );
    }
}
