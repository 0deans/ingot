use super::*;

#[test]
fn corrupt_credentials_diagnostics_do_not_echo_secret_values() {
    let raw =
        r#"{"accessToken":"access", "clientToken":"client", "expiresAt":"private-refresh-token"}"#;
    let source = serde_json::from_str::<AccountSecrets>(raw).unwrap_err();
    let error = CredentialError::Decode(source);
    assert!(std::error::Error::source(&error).is_some());
    assert!(!format!("{error:?} {error}").contains("private-refresh-token"));
}
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
    fail_write_id: Option<String>,
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
    pub(crate) fn fail_write_for(&self, id: Option<&str>) {
        self.0.lock().unwrap().fail_write_id = id.map(str::to_owned);
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
        if state.fail_write || state.fail_write_id.as_deref() == Some(id) {
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
        source: VaultFailure::injected(),
    }
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

#[test]
fn vault_failure_preserves_native_categories_and_redacts_outer_diagnostics() {
    let native = crate::keyring_store::KeyringError::Read(keyring::Error::Invalid(
        "private-access-token".to_owned(),
        "private-refresh-token".to_owned(),
    ));
    let failure = VaultFailure::from(native);
    let native = failure
        .source()
        .unwrap()
        .downcast_ref::<crate::keyring_store::KeyringError>()
        .unwrap();
    assert!(matches!(
        native,
        crate::keyring_store::KeyringError::Read(_)
    ));
    assert!(matches!(
        native.source().unwrap().downcast_ref::<keyring::Error>(),
        Some(keyring::Error::Invalid(_, _))
    ));
    let failure_diagnostics = format!("{failure} {failure:?}");
    let credentials = CredentialError::Vault {
        operation: "read",
        source: failure,
    };
    let credential_diagnostics = format!("{credentials} {credentials:?}");
    let recovery = crate::account::recovery::RecoveryError::Credentials(credentials);
    let recovery_diagnostics = format!("{recovery} {recovery:?}");
    for diagnostics in [
        failure_diagnostics,
        credential_diagnostics,
        recovery_diagnostics,
    ] {
        assert!(!diagnostics.contains("private-access-token"));
        assert!(!diagnostics.contains("private-refresh-token"));
    }
}

#[test]
fn fake_vault_failures_keep_a_typed_source() {
    let store = FakeCredentialStore::default();
    store.fail_reads();
    let error = load(&store, "account").unwrap_err();
    let failure = error
        .source()
        .unwrap()
        .downcast_ref::<VaultFailure>()
        .unwrap();
    assert!(failure
        .source()
        .unwrap()
        .downcast_ref::<InjectedVaultFailure>()
        .is_some());
}
