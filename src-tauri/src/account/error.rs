use std::{io, path::PathBuf};

use super::{policy::AccountPolicyError, validation::AccountValidationError};

#[derive(Debug, thiserror::Error)]
pub(crate) enum AccountError {
    #[error("Failed to {operation} accounts at {}: {source}", path.display())]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Failed to parse accounts at {}: {source}", path.display())]
    Decode {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("Failed to serialize accounts: {0}")]
    Encode(#[from] serde_json::Error),
    #[error(transparent)]
    Policy(#[from] AccountPolicyError),
    #[error("Invalid accounts data at {}: {source}", path.display())]
    Validation {
        path: PathBuf,
        #[source]
        source: AccountValidationError,
    },
    #[error("Accounts store lock poisoned")]
    LockPoisoned,
}
