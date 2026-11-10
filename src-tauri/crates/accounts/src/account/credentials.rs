//! Credential effects and serialization; coordinated publication lives in recovery.
use super::operations::AccountOperation;
use super::AccountSecrets;

pub struct VaultFailure {
    cause: VaultCause,
}

enum VaultCause {
    Native(crate::keyring_store::KeyringError),
    Injected(InjectedVaultFailure),
}

impl From<crate::keyring_store::KeyringError> for VaultFailure {
    fn from(source: crate::keyring_store::KeyringError) -> Self {
        Self {
            cause: VaultCause::Native(source),
        }
    }
}

impl std::fmt::Display for VaultFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("OS credential vault operation failed")
    }
}

impl std::fmt::Debug for VaultFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("VaultFailure")
            .finish_non_exhaustive()
    }
}

impl std::error::Error for VaultFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.cause {
            VaultCause::Native(source) => Some(source),
            VaultCause::Injected(source) => Some(source),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Injected credential vault failure")]
pub struct InjectedVaultFailure;

impl VaultFailure {
    pub fn injected() -> Self {
        Self {
            cause: VaultCause::Injected(InjectedVaultFailure),
        }
    }
}

#[derive(thiserror::Error)]
pub enum CredentialError {
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

impl std::fmt::Debug for CredentialError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CredentialError")
            .field("message", &self.to_string())
            .finish_non_exhaustive()
    }
}

pub trait CredentialStore: Send + Sync {
    fn read(&self, id: &str) -> Result<Option<String>, CredentialError>;
    fn write(&self, id: &str, value: &str) -> Result<(), CredentialError>;
    fn delete(&self, id: &str) -> Result<(), CredentialError>;
}

pub fn load(store: &dyn CredentialStore, id: &str) -> Result<AccountSecrets, CredentialError> {
    let value = store.read(id)?.ok_or(CredentialError::Missing)?;
    serde_json::from_str(&value).map_err(CredentialError::Decode)
}

pub fn save(
    store: &dyn CredentialStore,
    operation: &AccountOperation,
    secrets: &AccountSecrets,
) -> Result<(), CredentialError> {
    let value = serde_json::to_string(secrets).map_err(CredentialError::Encode)?;
    store.write(operation.id(), &value)
}

pub mod tests;
