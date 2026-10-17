use super::*;
use crate::account::{
    credentials::CredentialError, pending_login::PendingGrantError, policy::AccountPolicyError,
    recovery::RecoveryError, session::SessionError, AccountError,
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
        ApiError::from(SessionError::Ownership(
            crate::account::operations::OperationError
        )),
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
        ApiError::from(AccountError::Policy(
            AccountPolicyError::DuplicateReorderId("id".into())
        )),
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
