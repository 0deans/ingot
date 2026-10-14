//! Serializable API error union types for TauRPC result envelopes.
use serde::{Deserialize, Serialize};
use specta::Type;
use thiserror::Error;

/// Standardized, typed error envelope returned across all TauRPC APIs.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Error, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ApiError {
    #[error("Invalid input: {message}")]
    InvalidInput { message: String },

    #[error("Not found: {message}")]
    NotFound { message: String },

    #[error("Conflict: {message}")]
    Conflict { message: String },

    #[error("Authentication required: {message}")]
    Unauthorized { message: String },

    #[error("Operation cancelled: {message}")]
    Cancelled { message: String },

    #[error("Provider unavailable: {message}")]
    ProviderUnavailable { message: String },

    #[error("Storage failure: {message}")]
    StorageFailure { message: String },

    #[error("Incomplete recovery: {message}")]
    IncompleteRecovery { message: String },

    #[error("Internal failure: {message}")]
    Internal { message: String },
}

impl ApiError {
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput {
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            message: message.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Unauthorized {
            message: message.into(),
        }
    }

    pub fn cancelled(message: impl Into<String>) -> Self {
        Self::Cancelled {
            message: message.into(),
        }
    }

    pub fn provider_unavailable(message: impl Into<String>) -> Self {
        Self::ProviderUnavailable {
            message: message.into(),
        }
    }

    pub fn storage_failure(message: impl Into<String>) -> Self {
        Self::StorageFailure {
            message: message.into(),
        }
    }

    pub fn incomplete_recovery(message: impl Into<String>) -> Self {
        Self::IncompleteRecovery {
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    pub fn from_display(error: impl std::fmt::Display) -> Self {
        Self::internal(error.to_string())
    }
}

impl From<String> for ApiError {
    fn from(message: String) -> Self {
        if message.starts_with("Failed to load accounts at")
            || message.starts_with("Failed to parse accounts at")
            || message.starts_with("Failed to write accounts at")
            || message.starts_with("Failed to store upload")
            || message.starts_with("Corrupt settings at")
            || message.contains("Stored account credentials are invalid")
        {
            Self::storage_failure(message)
        } else if message == "Account not found"
            || message.starts_with("Server not found")
            || message.starts_with("Instance not found")
            || message == "No credentials found in OS Keyring"
            || message.contains("not found")
            || message.contains("Not found")
        {
            Self::not_found(message)
        } else if message.contains("already in use")
            || message.contains("still running")
            || message.contains("Stop the server")
            || message.contains("Stop the instance")
        {
            Self::conflict(message)
        } else if message.starts_with("Invalid")
            || message.contains("cannot be empty")
            || message.contains("is empty")
            || message.contains("not a valid")
            || message.contains("must be positive")
            || message.contains("is invalid")
            || message.contains("ticket is invalid")
            || message.contains("Pending Microsoft sign-in expired")
            || message.contains("Pending Microsoft sign-in record is invalid")
            || message.contains("This older Microsoft sign-in cannot be resumed")
        {
            Self::invalid_input(message)
        } else if message.contains("Cannot read account credentials")
            || message.contains("Cannot write account credentials")
            || message.contains("Cannot delete account credentials")
        {
            Self::provider_unavailable(message)
        } else {
            Self::internal(message)
        }
    }
}

impl From<&str> for ApiError {
    fn from(message: &str) -> Self {
        Self::from(message.to_string())
    }
}

impl From<crate::settings::SettingsError> for ApiError {
    fn from(error: crate::settings::SettingsError) -> Self {
        match error {
            crate::settings::SettingsError::InvalidMemory { .. }
            | crate::settings::SettingsError::InvalidWindow { .. }
            | crate::settings::SettingsError::InvalidBehavior(_) => {
                Self::invalid_input(error.to_string())
            }
            crate::settings::SettingsError::Decode { .. }
            | crate::settings::SettingsError::Encode(_)
            | crate::settings::SettingsError::Io { .. } => {
                Self::storage_failure(error.to_string())
            }
            crate::settings::SettingsError::LockPoisoned
            | crate::settings::SettingsError::StateUnavailable => {
                Self::internal(error.to_string())
            }
        }
    }
}

impl From<crate::account::credentials::CredentialError> for ApiError {
    fn from(error: crate::account::credentials::CredentialError) -> Self {
        match error {
            crate::account::credentials::CredentialError::Missing => Self::not_found("No credentials found"),
            crate::account::credentials::CredentialError::Vault { .. } => Self::provider_unavailable(error.to_string()),
            crate::account::credentials::CredentialError::Decode(_)
            | crate::account::credentials::CredentialError::Encode(_) => Self::storage_failure(error.to_string()),
        }
    }
}

impl From<crate::account::AccountError> for ApiError {
    fn from(error: crate::account::AccountError) -> Self {
        match error {
            crate::account::AccountError::Policy(crate::account::policy::AccountPolicyError::NotFound) => {
                Self::not_found("Account not found")
            }
            crate::account::AccountError::Policy(crate::account::policy::AccountPolicyError::DuplicateReorderId(id)) => {
                Self::invalid_input(format!("Duplicate account ID in reorder list: {id}"))
            }
            crate::account::AccountError::Validation { source, .. } => {
                Self::invalid_input(source.to_string())
            }
            crate::account::AccountError::Io { .. }
            | crate::account::AccountError::Decode { .. }
            | crate::account::AccountError::Encode(_) => {
                Self::storage_failure(error.to_string())
            }
            crate::account::AccountError::LockPoisoned => {
                Self::internal(error.to_string())
            }
        }
    }
}

impl From<crate::account::recovery::RecoveryError> for ApiError {
    fn from(error: crate::account::recovery::RecoveryError) -> Self {
        match error {
            crate::account::recovery::RecoveryError::Credentials(e) => Self::from(e),
            crate::account::recovery::RecoveryError::Account(e) => Self::from(e),
            crate::account::recovery::RecoveryError::PendingGrant(e) => Self::from(e),
            crate::account::recovery::RecoveryError::PendingCredentials(_)
            | crate::account::recovery::RecoveryError::Profile(_) => {
                Self::incomplete_recovery(error.to_string())
            }
            crate::account::recovery::RecoveryError::Discovery(_)
            | crate::account::recovery::RecoveryError::Invalid => {
                Self::storage_failure(error.to_string())
            }
            crate::account::recovery::RecoveryError::LockPoisoned => {
                Self::internal(error.to_string())
            }
        }
    }
}

impl From<crate::account::pending_login::PendingGrantError> for ApiError {
    fn from(error: crate::account::pending_login::PendingGrantError) -> Self {
        match error {
            crate::account::pending_login::PendingGrantError::Credentials(e) => Self::from(e),
            crate::account::pending_login::PendingGrantError::NotFound => Self::not_found(error.to_string()),
            crate::account::pending_login::PendingGrantError::AlreadyPending => Self::conflict(error.to_string()),
            crate::account::pending_login::PendingGrantError::Invalid
            | crate::account::pending_login::PendingGrantError::Expired
            | crate::account::pending_login::PendingGrantError::Legacy => {
                Self::invalid_input(error.to_string())
            }
        }
    }
}

impl From<crate::account::session::SessionError> for ApiError {
    fn from(error: crate::account::session::SessionError) -> Self {
        match error {
            crate::account::session::SessionError::Credentials(e) => Self::from(e),
            crate::account::session::SessionError::Recovery(e) => Self::from(e),
            crate::account::session::SessionError::PendingGrant(e) => Self::from(e),
            crate::account::session::SessionError::RotationCheckpoint(e) => Self::incomplete_recovery(e.to_string()),
            crate::account::session::SessionError::PendingGrantCleanup(e)
            | crate::account::session::SessionError::ResumeCheckpoint(e) => Self::incomplete_recovery(e.to_string()),
            crate::account::session::SessionError::AccountNotFound => Self::not_found("Account not found"),
            crate::account::session::SessionError::NoActiveAccount => Self::not_found("No active account selected"),
            crate::account::session::SessionError::Cancelled => Self::cancelled("Cancelled"),
            crate::account::session::SessionError::Validation(e) => Self::invalid_input(e.to_string()),
            crate::account::session::SessionError::InvalidLogin
            | crate::account::session::SessionError::MissingMinecraftProfile
            | crate::account::session::SessionError::NotMicrosoft
            | crate::account::session::SessionError::MissingRefreshToken => Self::invalid_input(error.to_string()),
            crate::account::session::SessionError::Ownership(_) => Self::conflict(error.to_string()),
            crate::account::session::SessionError::Provider(_)
            | crate::account::session::SessionError::Login(_) => Self::provider_unavailable(error.to_string()),
        }
    }
}

impl From<crate::version_change::VersionChangeError> for ApiError {
    fn from(error: crate::version_change::VersionChangeError) -> Self {
        match error {
            crate::version_change::VersionChangeError::InvalidFileName(msg)
            | crate::version_change::VersionChangeError::InvalidPath(msg)
            | crate::version_change::VersionChangeError::PlanError(msg)
            | crate::version_change::VersionChangeError::UndoError(msg) => Self::invalid_input(msg),
            crate::version_change::VersionChangeError::RollbackFailed { primary, restore, backup_path } => {
                Self::incomplete_recovery(format!(
                    "{primary}; restore also failed: {restore}. Recovery backup preserved at {}",
                    backup_path.display()
                ))
            }
            crate::version_change::VersionChangeError::RollbackSucceeded(msg) => {
                Self::conflict(format!("Version change aborted and rolled back: {msg}"))
            }
            crate::version_change::VersionChangeError::BackupError(msg)
            | crate::version_change::VersionChangeError::RestoreError(msg)
            | crate::version_change::VersionChangeError::FileOp(msg) => Self::storage_failure(msg),
            crate::version_change::VersionChangeError::DownloadPrep(msg)
            | crate::version_change::VersionChangeError::DownloadFailed(_, msg)
            | crate::version_change::VersionChangeError::ChecksumMismatch(msg)
            | crate::version_change::VersionChangeError::UnknownHost(msg)
            | crate::version_change::VersionChangeError::NoChecksum(msg) => Self::provider_unavailable(msg),
            crate::version_change::VersionChangeError::Custom(msg) => Self::internal(msg),
        }
    }
}

impl From<crate::minecraft::launcher::LaunchError> for ApiError {
    fn from(error: crate::minecraft::launcher::LaunchError) -> Self {
        match error {
            crate::minecraft::launcher::LaunchError::AlreadyRunning => {
                Self::conflict("Instance is already running")
            }
            crate::minecraft::launcher::LaunchError::NotRunning(id) => {
                Self::not_found(format!("Instance {id} is not running"))
            }
            crate::minecraft::launcher::LaunchError::LoaderVersionNotSelected => {
                Self::invalid_input("Loader version not selected")
            }
            crate::minecraft::launcher::LaunchError::AppDataDir(msg)
            | crate::minecraft::launcher::LaunchError::InstanceDir(msg) => {
                Self::storage_failure(msg)
            }
            crate::minecraft::launcher::LaunchError::IncompleteTransaction(msg) => {
                Self::incomplete_recovery(format!(
                    "Cannot launch instance: version change transaction is in incomplete state ({msg}). Please recover or revert it first."
                ))
            }
            crate::minecraft::launcher::LaunchError::VersionPackage(msg)
            | crate::minecraft::launcher::LaunchError::LoaderProfile(msg)
            | crate::minecraft::launcher::LaunchError::JavaRuntime(msg)
            | crate::minecraft::launcher::LaunchError::Download(msg) => {
                Self::provider_unavailable(msg)
            }
            crate::minecraft::launcher::LaunchError::Account(msg) => {
                Self::invalid_input(msg)
            }
            crate::minecraft::launcher::LaunchError::Settings(msg) => {
                Self::storage_failure(msg)
            }
            crate::minecraft::launcher::LaunchError::ProcessSpawn(err) => {
                Self::internal(format!("Failed to spawn Minecraft process: {err}"))
            }
            crate::minecraft::launcher::LaunchError::Io(err) => {
                Self::storage_failure(err.to_string())
            }
            crate::minecraft::launcher::LaunchError::Other(msg) => {
                Self::from(msg)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{
        credentials::CredentialError,
        pending_login::PendingGrantError,
        policy::AccountPolicyError,
        recovery::RecoveryError,
        session::SessionError,
        AccountError,
    };
    use crate::minecraft::launcher::LaunchError;
    use crate::version_change::VersionChangeError;

    #[test]
    fn test_launch_error_boundary_mappings() {
        assert!(matches!(
            ApiError::from(LaunchError::AlreadyRunning),
            ApiError::Conflict { .. }
        ));
        assert!(matches!(
            ApiError::from(LaunchError::NotRunning("inst-1".into())),
            ApiError::NotFound { .. }
        ));
        assert!(matches!(
            ApiError::from(LaunchError::IncompleteTransaction("test".into())),
            ApiError::IncompleteRecovery { .. }
        ));
        assert!(matches!(
            ApiError::from(LaunchError::JavaRuntime("missing".into())),
            ApiError::ProviderUnavailable { .. }
        ));
        assert!(matches!(
            ApiError::from(LaunchError::LoaderVersionNotSelected),
            ApiError::InvalidInput { .. }
        ));
        assert!(matches!(
            ApiError::from(LaunchError::AppDataDir("permission".into())),
            ApiError::StorageFailure { .. }
        ));
    }

    #[test]
    fn test_session_error_boundary_mappings() {
        assert_eq!(
            ApiError::from(SessionError::Cancelled),
            ApiError::Cancelled {
                message: "Cancelled".into()
            }
        );
        assert_eq!(
            ApiError::from(SessionError::AccountNotFound),
            ApiError::NotFound {
                message: "Account not found".into()
            }
        );
        assert_eq!(
            ApiError::from(SessionError::NoActiveAccount),
            ApiError::NotFound {
                message: "No active account selected".into()
            }
        );
        assert_eq!(
            ApiError::from(SessionError::InvalidLogin),
            ApiError::InvalidInput {
                message: "Username and password are required".into()
            }
        );
        assert_eq!(
            ApiError::from(SessionError::RotationCheckpoint(RecoveryError::Invalid)),
            ApiError::IncompleteRecovery {
                message: RecoveryError::Invalid.to_string()
            }
        );
        assert_eq!(
            ApiError::from(SessionError::ResumeCheckpoint(PendingGrantError::NotFound)),
            ApiError::IncompleteRecovery {
                message: PendingGrantError::NotFound.to_string()
            }
        );
        assert_eq!(
            ApiError::from(SessionError::Ownership(crate::account::operations::OperationError)),
            ApiError::Conflict {
                message: crate::account::operations::OperationError.to_string()
            }
        );
    }

    #[test]
    fn test_credential_error_boundary_mappings() {
        assert_eq!(
            ApiError::from(CredentialError::Missing),
            ApiError::NotFound {
                message: "No credentials found".into()
            }
        );
        assert!(matches!(
            ApiError::from(CredentialError::Vault {
                operation: "read",
                source: crate::account::credentials::VaultFailure("os keyring failed".into()),
            }),
            ApiError::ProviderUnavailable { .. }
        ));
    }

    #[test]
    fn test_pending_grant_error_boundary_mappings() {
        assert!(matches!(
            ApiError::from(PendingGrantError::NotFound),
            ApiError::NotFound { .. }
        ));
        assert!(matches!(
            ApiError::from(PendingGrantError::AlreadyPending),
            ApiError::Conflict { .. }
        ));
        assert!(matches!(
            ApiError::from(PendingGrantError::Expired),
            ApiError::InvalidInput { .. }
        ));
        assert!(matches!(
            ApiError::from(PendingGrantError::Invalid),
            ApiError::InvalidInput { .. }
        ));
    }

    #[test]
    fn test_account_error_boundary_mappings() {
        assert_eq!(
            ApiError::from(AccountError::Policy(AccountPolicyError::NotFound)),
            ApiError::NotFound {
                message: "Account not found".into()
            }
        );
        assert!(matches!(
            ApiError::from(AccountError::Policy(AccountPolicyError::DuplicateReorderId("id".into()))),
            ApiError::InvalidInput { .. }
        ));
    }

    #[test]
    fn test_version_change_error_boundary_mappings() {
        assert!(matches!(
            ApiError::from(VersionChangeError::RollbackFailed {
                primary: "failed".into(),
                restore: "also failed".into(),
                backup_path: std::path::PathBuf::from("/tmp/backup"),
            }),
            ApiError::IncompleteRecovery { .. }
        ));
        assert!(matches!(
            ApiError::from(VersionChangeError::RollbackSucceeded("reverted".into())),
            ApiError::Conflict { .. }
        ));
    }
}
