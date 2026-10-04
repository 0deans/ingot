use super::*;
use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct State {
    entries: HashMap<String, String>,
    fail_write: bool,
    fail_read: bool,
}

#[derive(Clone, Default)]
pub(crate) struct FakeCredentialStore(Arc<Mutex<State>>);

impl FakeCredentialStore {
    pub(crate) fn put(&self, id: &str, value: &str) {
        self.0
            .lock()
            .unwrap()
            .entries
            .insert(id.to_owned(), value.to_owned());
    }
    pub(crate) fn fail_writes(&self) {
        self.0.lock().unwrap().fail_write = true;
    }
    pub(crate) fn fail_reads(&self) {
        self.0.lock().unwrap().fail_read = true;
    }
}

impl CredentialStore for FakeCredentialStore {
    fn read(&self, id: &str) -> Result<Option<String>, CredentialError> {
        let state = self.0.lock().unwrap();
        if state.fail_read {
            return Err(failure("read"));
        }
        Ok(state.entries.get(id).cloned())
    }
    fn write(&self, id: &str, value: &str) -> Result<(), CredentialError> {
        let mut state = self.0.lock().unwrap();
        if state.fail_write {
            return Err(failure("write"));
        }
        state.entries.insert(id.to_owned(), value.to_owned());
        Ok(())
    }
    fn delete(&self, id: &str) -> Result<(), CredentialError> {
        self.0.lock().unwrap().entries.remove(id);
        Ok(())
    }
}

fn failure(operation: &'static str) -> CredentialError {
    CredentialError::Vault {
        operation,
        source: VaultFailure("private vault diagnostic".to_owned()),
    }
}

fn profile() -> super::super::AccountProfile {
    super::super::AccountProfile {
        id: "microsoft:test".to_owned(),
        account_type: "microsoft".to_owned(),
        username: "Player".to_owned(),
        uuid: "test".to_owned(),
        skin_url: None,
        is_active: true,
        created_at: 42,
    }
}

#[tokio::test]
async fn failed_vault_write_leaves_profile_and_previous_credentials_unchanged() {
    let operations = super::super::operations::AccountOperations::default();
    let operation = operations.acquire("microsoft:test").await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    let repository = super::super::repository::AccountRepository::new(path.clone());
    repository.activate(profile()).unwrap();
    let previous = std::fs::read(&path).unwrap();
    let store = FakeCredentialStore::default();
    store.put("microsoft:test", "old-credentials");
    store.fail_writes();
    let mut next = profile();
    next.username = "Changed".to_owned();
    let error = publish(&store, &operation, &AccountSecrets::default(), || {
        repository.activate(next)
    })
    .unwrap_err();
    assert!(matches!(
        error,
        CredentialCommitError::Credentials(CredentialError::Vault {
            operation: "write",
            ..
        })
    ));
    assert_eq!(std::fs::read(path).unwrap(), previous);
    assert_eq!(
        store.read("microsoft:test").unwrap().as_deref(),
        Some("old-credentials")
    );
}

#[tokio::test]
async fn profile_failure_reports_partial_commit_and_retains_rotated_tokens() {
    let operations = super::super::operations::AccountOperations::default();
    let operation = operations.acquire("microsoft:test").await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    std::fs::write(&path, b"[corrupt").unwrap();
    let repository = super::super::repository::AccountRepository::new(path.clone());
    let store = FakeCredentialStore::default();
    let secrets = AccountSecrets {
        access_token: "new-access".to_owned(),
        refresh_token: Some("new-refresh".to_owned()),
        ..Default::default()
    };
    let error = publish(&store, &operation, &secrets, || {
        repository.activate(profile())
    })
    .unwrap_err();
    assert!(error.source().is_some());
    assert!(matches!(
        error,
        CredentialCommitError::ProfileAfterCredentials(AccountError::Decode { .. })
    ));
    let stored = load(&store, "microsoft:test").unwrap();
    assert_eq!(stored.access_token, "new-access");
    assert_eq!(stored.refresh_token.as_deref(), Some("new-refresh"));
    assert_eq!(std::fs::read(path).unwrap(), b"[corrupt");
}

#[tokio::test]
async fn successful_publication_keeps_credentials_out_of_profile_storage() {
    let operations = super::super::operations::AccountOperations::default();
    let operation = operations.acquire("microsoft:test").await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    let repository = super::super::repository::AccountRepository::new(path.clone());
    let store = FakeCredentialStore::default();
    let secrets = AccountSecrets {
        access_token: "private-access".to_owned(),
        password: Some("private-password".to_owned()),
        ..Default::default()
    };
    publish(&store, &operation, &secrets, || {
        repository.activate(profile())
    })
    .unwrap();
    assert_eq!(repository.load().unwrap()[0].username, "Player");
    assert_eq!(
        load(&store, "microsoft:test").unwrap().password.as_deref(),
        Some("private-password")
    );
    let file = std::fs::read_to_string(path).unwrap();
    assert!(!file.contains("private-access"));
    assert!(!file.contains("private-password"));
}

#[test]
fn missing_corrupt_and_unavailable_credentials_are_distinct_and_errors_hide_values() {
    let store = FakeCredentialStore::default();
    assert!(matches!(load(&store, "id"), Err(CredentialError::Missing)));
    store.put(
        "id",
        r#"{"accessToken":"private-access","clientToken":"c","expiresAt":"private-password"}"#,
    );
    let error = load(&store, "id").unwrap_err();
    assert!(error.source().is_some());
    assert!(!error.to_string().contains("private-password"));
    assert!(!error.to_string().contains("private-access"));
    assert!(matches!(error, CredentialError::Decode(_)));
    store.fail_reads();
    assert!(matches!(
        load(&store, "id"),
        Err(CredentialError::Vault {
            operation: "read",
            ..
        })
    ));
}
