//! Native OS vault adapter; existing key names, chunking and service names are preserved.
use crate::{
    account::credentials::{CredentialError, CredentialStore, VaultFailure},
    keyring_store,
};

pub(crate) struct OsCredentialStore;

fn failure(operation: &'static str, source: String) -> CredentialError {
    CredentialError::Vault {
        operation,
        source: VaultFailure(source),
    }
}

impl CredentialStore for OsCredentialStore {
    fn read(&self, id: &str) -> Result<Option<String>, CredentialError> {
        keyring_store::get_secret(id).map_err(|source| failure("read", source))
    }
    fn write(&self, id: &str, value: &str) -> Result<(), CredentialError> {
        keyring_store::save_secret(id, value).map_err(|source| failure("write", source))
    }
    fn delete(&self, id: &str) -> Result<(), CredentialError> {
        keyring_store::delete_secret(id).map_err(|source| failure("delete", source))
    }
}
