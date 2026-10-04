//! Credential publication precedes profile publication. Partial success is explicit.
use super::operations::AccountOperation;
use super::{AccountError, AccountSecrets};

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub(crate) struct VaultFailure(pub(crate) String);

#[derive(Debug, thiserror::Error)]
pub(crate) enum CredentialError {
    #[error("Cannot {operation} account credentials")]
    Vault {
        operation: &'static str,
        #[source]
        source: VaultFailure,
    },
    #[error("Stored account credentials are invalid; please sign in again")]
    Decode(#[source] serde_json::Error),
    #[error("Cannot serialize account credentials")]
    Encode(#[source] serde_json::Error),
    #[error("No credentials found in OS Keyring")]
    Missing,
}

pub(crate) trait CredentialStore: Send + Sync {
    fn read(&self, id: &str) -> Result<Option<String>, CredentialError>;
    fn write(&self, id: &str, value: &str) -> Result<(), CredentialError>;
    fn delete(&self, id: &str) -> Result<(), CredentialError>;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum CredentialCommitError {
    #[error(transparent)]
    Credentials(#[from] CredentialError),
    // Keep rotated refresh tokens: reverting them could destroy the usable session.
    #[error("Credentials were saved, but the account profile could not be saved: {0}")]
    ProfileAfterCredentials(#[source] AccountError),
}

pub(crate) fn load(
    store: &dyn CredentialStore,
    id: &str,
) -> Result<AccountSecrets, CredentialError> {
    let value = store.read(id)?.ok_or(CredentialError::Missing)?;
    serde_json::from_str(&value).map_err(CredentialError::Decode)
}

pub(crate) fn save(
    store: &dyn CredentialStore,
    operation: &AccountOperation,
    secrets: &AccountSecrets,
) -> Result<(), CredentialError> {
    let value = serde_json::to_string(secrets).map_err(CredentialError::Encode)?;
    store.write(operation.id(), &value)
}

/// Ownership identifies the account. No await occurs between the writes.
pub(crate) fn publish<T>(
    store: &dyn CredentialStore,
    operation: &AccountOperation,
    secrets: &AccountSecrets,
    commit_profile: impl FnOnce() -> Result<T, AccountError>,
) -> Result<T, CredentialCommitError> {
    save(store, operation, secrets)?;
    commit_profile().map_err(CredentialCommitError::ProfileAfterCredentials)
}

#[cfg(test)]
pub(crate) mod tests;
