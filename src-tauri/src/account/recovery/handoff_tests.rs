use super::*;
use crate::account::pending_login::{GrantReceipt, PendingLoginGrants};
use crate::auth::microsoft::MicrosoftDeviceCode;

fn checkpoint(store: &Store, path: &std::path::Path) -> (PendingLoginGrants, GrantReceipt) {
    let grants = PendingLoginGrants::new(path);
    let receipt = grants
        .checkpoint(
            store,
            &MicrosoftDeviceCode {
                user_code: "fixture-user".into(),
                verification_uri: "https://fixture.invalid/link".into(),
                device_code: "private-device".into(),
                interval: 1,
                expires_in: 900,
            },
            "private-grant-refresh",
            1000,
        )
        .unwrap();
    (grants, receipt)
}

fn grant_key(path: &std::path::Path) -> String {
    let scope = uuid::Uuid::new_v3(
        &uuid::Uuid::NAMESPACE_URL,
        path.to_string_lossy().as_bytes(),
    );
    format!("ingot:microsoft-pending:v1:{scope}")
}

#[tokio::test]
async fn failed_journal_or_handoff_retains_material_before_any_account_effects() {
    for fault in [Fault::Journal, Fault::GrantAcknowledgement] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("accounts.json");
        let repository = AccountRepository::new(path.clone());
        let store = store(path.clone());
        let (_, receipt) = checkpoint(&store, &path);
        let original = store.read(&grant_key(&path)).unwrap();
        let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
        let operations = AccountOperations::default();
        let operation = operations.acquire("microsoft:test").await.unwrap();
        *store.fault.lock().unwrap() = fault;
        let result = recovery.publish_sign_in(
            &store,
            &repository,
            &operation,
            &secrets(),
            profile(operation.id()),
            receipt,
        );
        match fault {
            Fault::Journal => assert!(matches!(result, Err(RecoveryError::Credentials(_)))),
            Fault::GrantAcknowledgement => {
                assert!(matches!(result, Err(RecoveryError::PendingGrant(_))))
            }
            _ => unreachable!(),
        }
        assert_eq!(store.read(&grant_key(&path)).unwrap(), original);
        assert!(store.read(operation.id()).unwrap().is_none());
        assert!(!path.exists());
        *store.fault.lock().unwrap() = Fault::None;
        let profiles = AccountRecovery::new(recovery.marker.clone())
            .with_recovered(&store, &repository, || repository.load())
            .unwrap();
        if matches!(fault, Fault::Journal) {
            assert!(profiles.is_empty());
            assert_eq!(store.read(&grant_key(&path)).unwrap(), original);
        } else {
            assert_eq!(profiles[0].id, operation.id());
            assert_eq!(
                store.read(&grant_key(&path)).unwrap().as_deref(),
                Some("ingot-microsoft-pending-discarded-v1")
            );
        }
    }
}

#[tokio::test]
async fn interrupted_publication_consumes_grant_and_recovers_before_subsequent_removal() {
    for fault in [Fault::Canonical, Fault::Profile, Fault::Acknowledgement] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("accounts.json");
        let repository = AccountRepository::new(path.clone());
        let store = store(path.clone());
        let (_, receipt) = checkpoint(&store, &path);
        let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
        let operations = AccountOperations::default();
        let operation = operations.acquire("microsoft:test").await.unwrap();
        *store.fault.lock().unwrap() = fault;
        assert!(recovery
            .publish_sign_in(
                &store,
                &repository,
                &operation,
                &secrets(),
                profile(operation.id()),
                receipt
            )
            .is_err());
        assert_eq!(
            store.read(&grant_key(&path)).unwrap().as_deref(),
            Some("ingot-microsoft-pending-discarded-v1")
        );
        assert!(store.read(&recovery.key).unwrap().is_some());
        *store.fault.lock().unwrap() = Fault::None;
        if matches!(fault, Fault::Profile) {
            std::fs::write(&path, b"[]").unwrap();
        }
        let restarted = AccountRecovery::new(recovery.marker.clone());
        restarted
            .with_recovered(&store, &repository, || Ok(()))
            .unwrap();
        restarted.remove(&store, &repository, &operation).unwrap();
        assert!(restarted
            .with_recovered(&store, &repository, || repository.load())
            .unwrap()
            .is_empty());
        assert!(store.read(operation.id()).unwrap().is_none());
        assert_eq!(
            store.read(&grant_key(&path)).unwrap().as_deref(),
            Some("ingot-microsoft-pending-discarded-v1")
        );
    }
}

#[tokio::test]
async fn grant_chunk_cleanup_failure_after_publication_leaves_an_inert_record() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    let repository = AccountRepository::new(path.clone());
    let store = store(path.clone());
    let (grants, receipt) = checkpoint(&store, &path);
    let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
    let operations = AccountOperations::default();
    let operation = operations.acquire("microsoft:test").await.unwrap();
    recovery
        .publish_sign_in(
            &store,
            &repository,
            &operation,
            &secrets(),
            profile(operation.id()),
            receipt,
        )
        .unwrap();
    *store.fault.lock().unwrap() = Fault::GrantCleanup;
    assert!(grants.discard(&store).is_err());
    assert_eq!(
        store.read(&grant_key(&path)).unwrap().as_deref(),
        Some("ingot-microsoft-pending-discarded-v1")
    );
    assert_eq!(repository.load().unwrap()[0].id, operation.id());
    *store.fault.lock().unwrap() = Fault::None;
    recovery.remove(&store, &repository, &operation).unwrap();
    grants.discard(&store).unwrap();
    assert!(recovery
        .with_recovered(&store, &repository, || repository.load())
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn replay_of_an_older_handoff_preserves_a_newer_pending_ticket() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    let repository = AccountRepository::new(path.clone());
    let store = store(path.clone());
    let (_, receipt) = checkpoint(&store, &path);
    let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
    let operations = AccountOperations::default();
    let operation = operations.acquire("microsoft:test").await.unwrap();
    *store.fault.lock().unwrap() = Fault::Canonical;
    assert!(recovery
        .publish_sign_in(
            &store,
            &repository,
            &operation,
            &secrets(),
            profile(operation.id()),
            receipt
        )
        .is_err());
    *store.fault.lock().unwrap() = Fault::None;
    checkpoint(&store, &path);
    let next = store.read(&grant_key(&path)).unwrap();
    AccountRecovery::new(recovery.marker.clone())
        .with_recovered(&store, &repository, || Ok(()))
        .unwrap();
    assert_eq!(store.read(&grant_key(&path)).unwrap(), next);
    assert_eq!(repository.load().unwrap()[0].id, operation.id());
}

#[tokio::test]
async fn handoff_from_another_account_scope_is_rejected_without_consuming_either_grant() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    let other = dir.path().join("other.json");
    let repository = AccountRepository::new(path.clone());
    let store = store(path.clone());
    checkpoint(&store, &path);
    let (_, receipt) = checkpoint(&store, &other);
    let current = store.read(&grant_key(&path)).unwrap();
    let unrelated = store.read(&grant_key(&other)).unwrap();
    let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
    let operations = AccountOperations::default();
    let operation = operations.acquire("microsoft:test").await.unwrap();
    assert!(matches!(
        recovery.publish_sign_in(
            &store,
            &repository,
            &operation,
            &secrets(),
            profile(operation.id()),
            receipt
        ),
        Err(RecoveryError::Invalid)
    ));
    assert!(!recovery.marker.exists());
    assert!(!path.exists());
    assert_eq!(store.read(&grant_key(&path)).unwrap(), current);
    assert_eq!(store.read(&grant_key(&other)).unwrap(), unrelated);
}

#[test]
fn invalid_handoff_journals_are_preserved_without_reflecting_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    let repository = AccountRepository::new(path.clone());
    let store = store(path.clone());
    let (_, receipt) = checkpoint(&store, &path);
    let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
    recovery.create_marker().unwrap();
    let value = serde_json::to_value([Intent {
        version: 2,
        change: ProfileChange::Activate {
            profile: profile("microsoft:test"),
        },
        secrets: Some(secrets()),
        grant: Some(receipt),
    }])
    .unwrap();
    for case in 0..5 {
        let mut invalid = value.clone();
        match case {
            0 => invalid[0]["grant"]["scope"] = serde_json::json!(uuid::Uuid::new_v4()),
            1 => invalid[0]["grant"]["ticket"] = serde_json::json!(uuid::Uuid::nil()),
            2 => invalid[0]["version"] = serde_json::json!(1),
            3 => {
                invalid[0]["change"] = serde_json::json!({ "kind": "Keep", "id": "microsoft:test" })
            }
            4 => invalid[0]["change"]["profile"]["accountType"] = serde_json::json!("ely"),
            _ => unreachable!(),
        }
        let invalid = serde_json::to_string(&invalid).unwrap();
        store.vault.put(&recovery.key, &invalid);
        let error = recovery
            .with_recovered(&store, &repository, || Ok(()))
            .unwrap_err();
        assert!(matches!(error, RecoveryError::Invalid));
        assert!(!format!("{error:?}: {error}").contains("private-"));
        assert_eq!(
            store.read(&recovery.key).unwrap().as_deref(),
            Some(invalid.as_str())
        );
        assert!(!path.exists());
    }
}

#[test]
fn restart_at_each_handoff_boundary_finishes_publication_without_reusable_grant() {
    for phase in 0..6 {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("accounts.json");
        let repository = AccountRepository::new(path.clone());
        let store = store(path.clone());
        let (_, receipt) = checkpoint(&store, &path);
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
            .write(&recovery.key, &serde_json::to_string(&[&intent]).unwrap())
            .unwrap();
        if phase >= 1 {
            PendingLoginGrants::acknowledge_transfer(&store, receipt).unwrap();
        }
        if phase >= 2 {
            store
                .write(
                    "microsoft:test",
                    &serde_json::to_string(&secrets()).unwrap(),
                )
                .unwrap();
        }
        if phase >= 3 {
            intent.change.apply(&repository).unwrap();
        }
        if phase >= 4 {
            store.write(&recovery.key, COMPLETE).unwrap();
        }
        if phase >= 5 {
            store.delete(&recovery.key).unwrap();
        }
        let profiles = AccountRecovery::new(recovery.marker.clone())
            .with_recovered(&store, &repository, || repository.load())
            .unwrap();
        assert_eq!(profiles[0].id, "microsoft:test");
        assert_eq!(
            store.read(&grant_key(&path)).unwrap().as_deref(),
            Some("ingot-microsoft-pending-discarded-v1")
        );
        assert_eq!(
            credentials::load(&store, "microsoft:test")
                .unwrap()
                .refresh_token
                .as_deref(),
            Some("private-refresh")
        );
        assert!(!recovery.marker.exists());
        for file in std::fs::read_dir(dir.path()).unwrap() {
            assert!(!std::fs::read_to_string(file.unwrap().path())
                .unwrap()
                .contains("private-"));
        }
    }
}

#[test]
fn legacy_account_journal_without_a_grant_receipt_still_recovers() {
    let dir = tempfile::tempdir().unwrap();
    let repository = AccountRepository::new(dir.path().join("accounts.json"));
    let store = store(repository.path().to_owned());
    let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
    recovery.create_marker().unwrap();
    store.vault.put(&recovery.key, r#"[{"version":1,"change":{"kind":"Activate","profile":{"id":"microsoft:test","accountType":"microsoft","username":"LegacyPlayer","uuid":"test","skinUrl":null,"isActive":true,"createdAt":42}},"secrets":{"accessToken":"private-access","clientToken":"","refreshToken":"private-refresh"}}]"#);
    let profiles = recovery
        .with_recovered(&store, &repository, || repository.load())
        .unwrap();
    assert_eq!(profiles[0].username, "LegacyPlayer");
    assert_eq!(
        credentials::load(&store, "microsoft:test")
            .unwrap()
            .refresh_token
            .as_deref(),
        Some("private-refresh")
    );
    assert!(!recovery.marker.exists());
}

#[tokio::test]
async fn corrupt_pending_grant_blocks_handoff_without_discarding_either_record() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    let repository = AccountRepository::new(path.clone());
    let store = store(path.clone());
    let (_, receipt) = checkpoint(&store, &path);
    store
        .vault
        .put(&grant_key(&path), "private-refresh malformed");
    let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
    let operations = AccountOperations::default();
    let operation = operations.acquire("microsoft:test").await.unwrap();
    let error = recovery
        .publish_sign_in(
            &store,
            &repository,
            &operation,
            &secrets(),
            profile(operation.id()),
            receipt,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        RecoveryError::PendingGrant(crate::account::pending_login::PendingGrantError::Invalid)
    ));
    assert!(!format!("{error:?}: {error}").contains("private-refresh"));
    assert_eq!(
        store.read(&grant_key(&path)).unwrap().as_deref(),
        Some("private-refresh malformed")
    );
    assert!(store
        .read(&recovery.key)
        .unwrap()
        .unwrap()
        .contains("private-access"));
    assert!(!path.exists());
    assert!(store.read(operation.id()).unwrap().is_none());
}
