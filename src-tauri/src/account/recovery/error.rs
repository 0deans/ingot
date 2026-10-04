use crate::account::{credentials::CredentialError, AccountError};

#[derive(Debug, thiserror::Error)]
pub(crate) enum RecoveryError {
    #[error("Account recovery discovery failed")]
    Discovery(#[source] std::io::Error),
    #[error("Account recovery record is invalid; recovery data was preserved")]
    Invalid,
    #[error("Account recovery registry lock poisoned")]
    LockPoisoned,
    #[error("Account recovery could not access credentials: {0}")]
    Credentials(#[from] CredentialError),
    #[error("Account change is pending recovery: {0}")]
    PendingCredentials(#[source] CredentialError),
    #[error(transparent)]
    Account(#[from] AccountError),
    #[error("Account change is pending recovery: {0}")]
    Profile(#[source] AccountError),
}
