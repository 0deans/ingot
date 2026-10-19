//! Native OS vault adapter; existing key names, chunking and service names are preserved.
use crate::{
    account::credentials::{CredentialError, CredentialStore, VaultFailure},
    keyring_store,
};

pub(crate) struct OsCredentialStore;

fn failure(operation: &'static str, source: keyring_store::KeyringError) -> CredentialError {
    CredentialError::Vault {
        operation,
        source: VaultFailure::from(source),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn native_adapter_keeps_vault_operation_and_source_category() {
        let error = failure("delete", keyring_store::KeyringError::IncompleteCredentials);
        assert!(matches!(
            error,
            CredentialError::Vault {
                operation: "delete",
                ..
            }
        ));
        let vault = error
            .source()
            .unwrap()
            .downcast_ref::<VaultFailure>()
            .unwrap();
        assert!(matches!(
            vault
                .source()
                .unwrap()
                .downcast_ref::<keyring_store::KeyringError>(),
            Some(keyring_store::KeyringError::IncompleteCredentials)
        ));
    }
}
