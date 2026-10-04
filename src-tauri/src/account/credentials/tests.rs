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
        source: VaultFailure("private vault diagnostic".to_owned()),
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
