//! Credential effects and serialization; coordinated publication lives in recovery.
use super::operations::AccountOperation;
use super::AccountSecrets;

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

#[cfg(test)]
pub(crate) mod tests;
