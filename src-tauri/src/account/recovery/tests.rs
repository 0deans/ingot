use super::*;
use crate::account::{
    credentials::{self, tests::FakeCredentialStore, CredentialError, VaultFailure},
    operations::AccountOperations,
    AccountProfile,
};

#[derive(Clone, Copy, Default)]
enum Fault {
    #[default]
    None,
    Journal,
    Canonical,
    Profile,
    Cleanup,
    Acknowledgement,
    DeleteAccount,
}

struct Store {
    vault: FakeCredentialStore,
    fault: Mutex<Fault>,
    profile_path: PathBuf,
}
impl CredentialStore for Store {
    fn read(&self, id: &str) -> Result<Option<String>, CredentialError> {
        self.vault.read(id)
    }
    fn write(&self, id: &str, value: &str) -> Result<(), CredentialError> {
        let fault = *self.fault.lock().unwrap();
        let journal = id.starts_with("ingot:account-recovery:");
        if matches!(fault, Fault::Journal) && journal
            || matches!(fault, Fault::Canonical) && !journal
            || matches!(fault, Fault::Acknowledgement) && value == COMPLETE
        {
            return Err(failure("write"));
        }
        self.vault.write(id, value)?;
        if matches!(fault, Fault::Profile) && !journal {
            std::fs::write(&self.profile_path, b"[corrupt").unwrap();
        }
        Ok(())
    }
    fn delete(&self, id: &str) -> Result<(), CredentialError> {
        let fault = *self.fault.lock().unwrap();
        if matches!(fault, Fault::Cleanup) && id.starts_with("ingot:account-recovery:")
            || matches!(fault, Fault::DeleteAccount) && !id.starts_with("ingot:account-recovery:")
        {
            return Err(failure("delete"));
        }
        self.vault.delete(id)
    }
}
fn failure(operation: &'static str) -> CredentialError {
    CredentialError::Vault {
        operation,
        source: VaultFailure("private diagnostic".to_owned()),
    }
}
fn profile(id: &str) -> AccountProfile {
    AccountProfile {
        id: id.to_owned(),
        account_type: "microsoft".to_owned(),
        username: "Player".to_owned(),
        uuid: "test".to_owned(),
        skin_url: None,
        is_active: true,
        created_at: 42,
    }
}
fn secrets() -> AccountSecrets {
    AccountSecrets {
        access_token: "private-access".to_owned(),
        refresh_token: Some("private-refresh".to_owned()),
        password: Some("private-password".to_owned()),
        ..Default::default()
    }
}
fn store(path: PathBuf) -> Store {
    Store {
        vault: FakeCredentialStore::default(),
        fault: Mutex::new(Fault::None),
        profile_path: path,
    }
}

#[tokio::test]
async fn failed_journal_publication_changes_neither_profile_nor_canonical_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    let repository = AccountRepository::new(path.clone());
    repository.activate(profile("microsoft:test")).unwrap();
    let previous = std::fs::read(&path).unwrap();
    let store = store(path);
    store.vault.put("microsoft:test", "old");
    *store.fault.lock().unwrap() = Fault::Journal;
    let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
    let operations = AccountOperations::default();
    let operation = operations.acquire("microsoft:test").await.unwrap();
    assert!(matches!(
        recovery.publish(
            &store,
            &repository,
            &operation,
            &secrets(),
            ProfileChange::Activate {
                profile: profile("microsoft:test")
            }
        ),
        Err(RecoveryError::Credentials(_))
    ));
    assert_eq!(std::fs::read(&store.profile_path).unwrap(), previous);
    assert_eq!(store.read(operation.id()).unwrap().as_deref(), Some("old"));
    AccountRecovery::new(recovery.marker.clone())
        .with_recovered(&store, &repository, || repository.load())
        .unwrap();
    assert!(!recovery.marker.exists());
}

#[tokio::test]
async fn canonical_write_failure_recovers_new_login_after_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    let repository = AccountRepository::new(path.clone());
    let store = store(path);
    *store.fault.lock().unwrap() = Fault::Canonical;
    let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
    let operations = AccountOperations::default();
    let operation = operations.acquire("microsoft:test").await.unwrap();
    assert!(matches!(
        recovery.publish(
            &store,
            &repository,
            &operation,
            &secrets(),
            ProfileChange::Activate {
                profile: profile("microsoft:test")
            }
        ),
        Err(RecoveryError::PendingCredentials(_))
    ));
    assert!(repository.load().unwrap().is_empty());
    assert!(store.read(operation.id()).unwrap().is_none());
    *store.fault.lock().unwrap() = Fault::None;
    let profiles = AccountRecovery::new(recovery.marker.clone())
        .with_recovered(&store, &repository, || repository.load())
        .unwrap();
    assert_eq!(profiles[0].id, operation.id());
    assert_eq!(
        credentials::load(&store, operation.id())
            .unwrap()
            .refresh_token
            .as_deref(),
        Some("private-refresh")
    );
    assert!(store.read(&recovery.key).unwrap().is_none());
    assert!(!recovery.marker.exists());
}

#[tokio::test]
async fn profile_failure_preserves_rotated_tokens_and_recovery_until_storage_is_repaired() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    let repository = AccountRepository::new(path.clone());
    repository.activate(profile("microsoft:test")).unwrap();
    let previous = std::fs::read(&path).unwrap();
    let store = store(path.clone());
    *store.fault.lock().unwrap() = Fault::Profile;
    let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
    let operations = AccountOperations::default();
    let operation = operations.acquire("microsoft:test").await.unwrap();
    assert!(matches!(
        recovery.publish(
            &store,
            &repository,
            &operation,
            &secrets(),
            ProfileChange::MicrosoftMetadata {
                id: operation.id().to_owned(),
                username: "Updated".to_owned(),
                skin_url: None
            }
        ),
        Err(RecoveryError::Profile(AccountError::Decode { .. }))
    ));
    assert_eq!(
        credentials::load(&store, operation.id())
            .unwrap()
            .access_token,
        "private-access"
    );
    let restarted = AccountRecovery::new(recovery.marker.clone());
    *store.fault.lock().unwrap() = Fault::None;
    assert!(restarted
        .with_recovered(&store, &repository, || Ok(()))
        .is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"[corrupt");
    assert!(store.read(&recovery.key).unwrap().is_some());
    std::fs::write(&path, previous).unwrap();
    let profiles = restarted
        .with_recovered(&store, &repository, || repository.load())
        .unwrap();
    assert_eq!(profiles[0].username, "Updated");
    assert!(!std::fs::read_to_string(path).unwrap().contains("private-"));
}

#[tokio::test]
async fn cleanup_failure_replays_before_new_selection_and_never_steals_it_later() {
    let dir = tempfile::tempdir().unwrap();
    let repository = AccountRepository::new(dir.path().join("accounts.json"));
    repository.activate(profile("microsoft:other")).unwrap();
    let store = store(dir.path().join("accounts.json"));
    *store.fault.lock().unwrap() = Fault::Cleanup;
    let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
    let operations = AccountOperations::default();
    let operation = operations.acquire("microsoft:test").await.unwrap();
    assert!(recovery
        .publish(
            &store,
            &repository,
            &operation,
            &secrets(),
            ProfileChange::Activate {
                profile: profile(operation.id())
            }
        )
        .is_err());
    let mut changed = false;
    assert!(recovery
        .with_recovered(&store, &repository, || {
            changed = true;
            Ok(())
        })
        .is_err());
    assert!(!changed);
    *store.fault.lock().unwrap() = Fault::None;
    let restarted = AccountRecovery::new(recovery.marker.clone());
    restarted
        .with_recovered(&store, &repository, || {
            repository.update(|profiles| {
                crate::account::policy::select(profiles, "microsoft:other")?;
                Ok(())
            })
        })
        .unwrap();
    let profiles = restarted
        .with_recovered(&store, &repository, || repository.load())
        .unwrap();
    assert_eq!(
        profiles.iter().find(|p| p.is_active).unwrap().id,
        "microsoft:other"
    );
}

#[tokio::test]
async fn removal_cleanup_is_retryable_after_restart_and_preserves_promoted_selection() {
    let dir = tempfile::tempdir().unwrap();
    let repository = AccountRepository::new(dir.path().join("accounts.json"));
    repository.activate(profile("microsoft:other")).unwrap();
    repository.activate(profile("microsoft:test")).unwrap();
    let store = store(dir.path().join("accounts.json"));
    store.vault.put("microsoft:test", "old");
    *store.fault.lock().unwrap() = Fault::DeleteAccount;
    let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
    let operations = AccountOperations::default();
    let operation = operations.acquire("microsoft:test").await.unwrap();
    assert!(matches!(
        recovery.remove(&store, &repository, &operation),
        Err(RecoveryError::PendingCredentials(_))
    ));
    let profiles = repository.load().unwrap();
    assert_eq!(profiles.len(), 1);
    assert!(profiles[0].is_active);
    assert!(store.read(operation.id()).unwrap().is_some());
    *store.fault.lock().unwrap() = Fault::None;
    AccountRecovery::new(recovery.marker.clone())
        .with_recovered(&store, &repository, || Ok(()))
        .unwrap();
    assert!(store.read(operation.id()).unwrap().is_none());
    assert_eq!(repository.load().unwrap()[0].id, "microsoft:other");
}

#[test]
fn interruption_at_each_publication_boundary_is_recoverable_and_secret_safe() {
    for phase in 0..5 {
        let dir = tempfile::tempdir().unwrap();
        let repository = AccountRepository::new(dir.path().join("accounts.json"));
        let store = store(dir.path().join("accounts.json"));
        let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
        let intent = Intent {
            version: 1,
            change: ProfileChange::Activate {
                profile: profile("microsoft:test"),
            },
            secrets: Some(secrets()),
        };
        recovery.create_marker().unwrap();
        store
            .write(&recovery.key, &serde_json::to_string(&[&intent]).unwrap())
            .unwrap();
        if phase >= 1 {
            store
                .write(
                    intent.change.id(),
                    &serde_json::to_string(&secrets()).unwrap(),
                )
                .unwrap();
        }
        if phase >= 2 {
            intent.change.apply(&repository).unwrap();
        }
        if phase >= 3 {
            store.write(&recovery.key, COMPLETE).unwrap();
        }
        if phase >= 4 {
            store.delete(&recovery.key).unwrap();
        }
        let profiles = AccountRecovery::new(recovery.marker.clone())
            .with_recovered(&store, &repository, || repository.load())
            .unwrap();
        assert_eq!(profiles[0].id, "microsoft:test");
        for file in std::fs::read_dir(dir.path()).unwrap() {
            assert!(!std::fs::read_to_string(file.unwrap().path())
                .unwrap()
                .contains("private-"));
        }
        assert!(!recovery.marker.exists());
    }
}

#[test]
fn invalid_unknown_version_or_unavailable_journal_blocks_mutations_without_discarding_data() {
    let dir = tempfile::tempdir().unwrap();
    let repository = AccountRepository::new(dir.path().join("accounts.json"));
    let store = store(dir.path().join("accounts.json"));
    let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
    recovery.create_marker().unwrap();
    for value in [
        "private-password",
        r#"[{"version":2,"change":{"kind":"Remove","id":"microsoft:test"},"secrets":null}]"#,
        r#"[{"version":1,"change":{"kind":"private-token"},"secrets":null}]"#,
    ] {
        store.vault.put(&recovery.key, value);
        let error = recovery
            .with_recovered(&store, &repository, || -> Result<(), AccountError> {
                panic!("must not mutate")
            })
            .unwrap_err();
        assert!(matches!(error, RecoveryError::Invalid));
        assert!(!format!("{error:?}").contains("private-"));
        assert_eq!(store.read(&recovery.key).unwrap().as_deref(), Some(value));
        assert!(recovery.marker.exists());
    }
    store.vault.fail_reads();
    assert!(matches!(
        recovery.with_recovered(&store, &repository, || Ok(())),
        Err(RecoveryError::Credentials(_))
    ));
    assert!(recovery.marker.exists());
}

#[tokio::test]
async fn corrupt_profile_or_unwritable_discovery_never_creates_a_pending_intent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    let repository = AccountRepository::new(path.clone());
    let store = store(path.clone());
    let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
    let operations = AccountOperations::default();
    let operation = operations.acquire("microsoft:test").await.unwrap();
    std::fs::write(&path, b"[corrupt").unwrap();
    assert!(matches!(
        recovery.remove(&store, &repository, &operation),
        Err(RecoveryError::Account(AccountError::Decode { .. }))
    ));
    assert!(!recovery.marker.exists());
    assert!(store.read(&recovery.key).unwrap().is_none());
    std::fs::remove_file(&path).unwrap();
    std::fs::write(dir.path().join("blocked"), b"file").unwrap();
    let blocked = AccountRecovery::new(dir.path().join("blocked/marker"));
    assert!(matches!(
        blocked.publish(
            &store,
            &repository,
            &operation,
            &secrets(),
            ProfileChange::Activate {
                profile: profile(operation.id())
            }
        ),
        Err(RecoveryError::Discovery(_))
    ));
    assert!(store.read(operation.id()).unwrap().is_none());
}

#[test]
fn no_marker_reads_no_vault_and_creates_no_directories() {
    let dir = tempfile::tempdir().unwrap();
    let repository = AccountRepository::new(dir.path().join("missing/accounts.json"));
    let store = FakeCredentialStore::default();
    store.fail_reads();
    let recovery = AccountRecovery::new(dir.path().join("missing/accounts.recovery"));
    assert!(recovery
        .with_recovered(&store, &repository, || repository.load())
        .unwrap()
        .is_empty());
    assert!(!dir.path().join("missing").exists());
}

#[test]
fn interrupted_removal_at_each_boundary_finishes_without_resurrecting_profiles() {
    for phase in 0..5 {
        let dir = tempfile::tempdir().unwrap();
        let repository = AccountRepository::new(dir.path().join("accounts.json"));
        repository.activate(profile("microsoft:other")).unwrap();
        repository.activate(profile("microsoft:test")).unwrap();
        let store = store(dir.path().join("accounts.json"));
        store.vault.put("microsoft:test", "old");
        let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
        let intent = Intent {
            version: 1,
            change: ProfileChange::Remove {
                id: "microsoft:test".to_owned(),
            },
            secrets: None,
        };
        recovery.create_marker().unwrap();
        store
            .write(&recovery.key, &serde_json::to_string(&[&intent]).unwrap())
            .unwrap();
        if phase >= 1 {
            intent.change.apply(&repository).unwrap();
        }
        if phase >= 2 {
            store.delete(intent.change.id()).unwrap();
        }
        if phase >= 3 {
            store.write(&recovery.key, COMPLETE).unwrap();
        }
        if phase >= 4 {
            store.delete(&recovery.key).unwrap();
        }
        let profiles = AccountRecovery::new(recovery.marker.clone())
            .with_recovered(&store, &repository, || repository.load())
            .unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].id, "microsoft:other");
        assert!(profiles[0].is_active);
        assert!(store.read(intent.change.id()).unwrap().is_none());
        assert!(!recovery.marker.exists());
    }
}

#[test]
fn refresh_recovery_patches_metadata_preserving_selection_and_never_recreates_missing_target() {
    let dir = tempfile::tempdir().unwrap();
    let repository = AccountRepository::new(dir.path().join("accounts.json"));
    repository.activate(profile("microsoft:test")).unwrap();
    repository.activate(profile("microsoft:other")).unwrap();
    let store = store(dir.path().join("accounts.json"));
    let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
    let intent = Intent {
        version: 1,
        change: ProfileChange::MicrosoftMetadata {
            id: "microsoft:test".to_owned(),
            username: "Updated".to_owned(),
            skin_url: Some("skin".to_owned()),
        },
        secrets: Some(secrets()),
    };
    recovery.create_marker().unwrap();
    store
        .write(&recovery.key, &serde_json::to_string(&[&intent]).unwrap())
        .unwrap();
    recovery
        .with_recovered(&store, &repository, || Ok(()))
        .unwrap();
    let profiles = repository.load().unwrap();
    assert_eq!(
        profiles.iter().find(|p| p.is_active).unwrap().id,
        "microsoft:other"
    );
    assert_eq!(profiles[0].username, "Updated");
    repository
        .update(|profiles| {
            crate::account::policy::remove(profiles, "microsoft:test")?;
            Ok(())
        })
        .unwrap();
    recovery.create_marker().unwrap();
    store
        .write(&recovery.key, &serde_json::to_string(&[&intent]).unwrap())
        .unwrap();
    assert!(matches!(
        recovery.with_recovered(&store, &repository, || Ok(())),
        Err(RecoveryError::Profile(AccountError::Policy(_)))
    ));
    assert_eq!(repository.load().unwrap().len(), 1);
    assert!(store.read(&recovery.key).unwrap().is_some());
}

#[tokio::test]
async fn failed_completion_acknowledgement_retains_replayable_intent() {
    let dir = tempfile::tempdir().unwrap();
    let repository = AccountRepository::new(dir.path().join("accounts.json"));
    let store = store(dir.path().join("accounts.json"));
    *store.fault.lock().unwrap() = Fault::Acknowledgement;
    let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
    let operations = AccountOperations::default();
    let operation = operations.acquire("microsoft:test").await.unwrap();
    assert!(matches!(
        recovery.publish(
            &store,
            &repository,
            &operation,
            &secrets(),
            ProfileChange::Activate {
                profile: profile(operation.id())
            }
        ),
        Err(RecoveryError::PendingCredentials(_))
    ));
    assert_eq!(repository.load().unwrap()[0].id, operation.id());
    let retained = store.read(&recovery.key).unwrap().unwrap();
    assert!(serde_json::from_str::<Vec<Intent>>(&retained).is_ok());
    *store.fault.lock().unwrap() = Fault::None;
    AccountRecovery::new(recovery.marker.clone())
        .with_recovered(&store, &repository, || Ok(()))
        .unwrap();
    assert!(store.read(&recovery.key).unwrap().is_none());
}

#[tokio::test]
async fn rotation_is_preserved_if_storage_becomes_corrupt_before_local_commit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    let repository = AccountRepository::new(path.clone());
    let store = store(path.clone());
    let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
    std::fs::write(&path, b"[corrupt").unwrap();
    let operations = AccountOperations::default();
    let operation = operations.acquire("microsoft:test").await.unwrap();
    assert!(matches!(
        recovery.publish(
            &store,
            &repository,
            &operation,
            &secrets(),
            ProfileChange::Keep {
                id: operation.id().to_owned()
            }
        ),
        Err(RecoveryError::Profile(AccountError::Decode { .. }))
    ));
    assert_eq!(
        credentials::load(&store, operation.id())
            .unwrap()
            .refresh_token
            .as_deref(),
        Some("private-refresh")
    );
    assert!(store.read(&recovery.key).unwrap().is_some());
    assert_eq!(std::fs::read(path).unwrap(), b"[corrupt");
}

#[tokio::test]
async fn another_accounts_completed_rotation_is_queued_behind_failed_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    let repository = AccountRepository::new(path.clone());
    repository.activate(profile("microsoft:first")).unwrap();
    let previous = std::fs::read(&path).unwrap();
    let store = store(path.clone());
    *store.fault.lock().unwrap() = Fault::Profile;
    let recovery = AccountRecovery::new(dir.path().join("accounts.recovery"));
    let operations = AccountOperations::default();
    let first = operations.acquire("microsoft:first").await.unwrap();
    assert!(recovery
        .publish(
            &store,
            &repository,
            &first,
            &secrets(),
            ProfileChange::Keep {
                id: first.id().to_owned()
            }
        )
        .is_err());
    drop(first);
    *store.fault.lock().unwrap() = Fault::None;
    let second = operations.acquire("microsoft:second").await.unwrap();
    let next = AccountSecrets {
        access_token: "second-access".to_owned(),
        refresh_token: Some("second-refresh".to_owned()),
        ..Default::default()
    };
    assert!(recovery
        .publish(
            &store,
            &repository,
            &second,
            &next,
            ProfileChange::Activate {
                profile: profile(second.id())
            }
        )
        .is_err());
    let value = store.read(&recovery.key).unwrap().unwrap();
    let queued: Vec<Intent> = serde_json::from_str(&value).unwrap();
    assert_eq!(queued.len(), 2);
    assert_eq!(
        queued[1].secrets.as_ref().unwrap().refresh_token.as_deref(),
        Some("second-refresh")
    );
    std::fs::write(&path, previous).unwrap();
    AccountRecovery::new(recovery.marker.clone())
        .with_recovered(&store, &repository, || Ok(()))
        .unwrap();
    assert_eq!(
        credentials::load(&store, second.id())
            .unwrap()
            .refresh_token
            .as_deref(),
        Some("second-refresh")
    );
    let profiles = repository.load().unwrap();
    assert_eq!(profiles.len(), 2);
    assert_eq!(
        profiles.iter().find(|p| p.is_active).unwrap().id,
        second.id()
    );
}
