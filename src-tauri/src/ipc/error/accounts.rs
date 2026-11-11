use super::ApiError;

impl From<crate::app::accounts::ProfileError> for ApiError {
    fn from(error: crate::app::accounts::ProfileError) -> Self {
        use crate::app::accounts::ProfileError;
        match error {
            ProfileError::EmptyNickname => Self::invalid_input("Player nickname cannot be empty"),
            ProfileError::Validation(error) => Self::invalid_input(error.to_string()),
            ProfileError::Ownership(error) => Self::conflict(error.to_string()),
            ProfileError::Recovery(error) => Self::from(error),
        }
    }
}

impl From<crate::app::accounts::SkinError> for ApiError {
    fn from(error: crate::app::accounts::SkinError) -> Self {
        use crate::app::accounts::SkinError;
        match error {
            SkinError::Ownership(error) => Self::conflict(error.to_string()),
            SkinError::Recovery(error) => Self::from(error),
            SkinError::Credentials(error) => Self::from(error),
            SkinError::Session(error) => Self::from(error),
            SkinError::AccountNotFound => Self::not_found(error.to_string()),
            SkinError::InvalidImage(_) | SkinError::NotEly => {
                Self::invalid_input(error.to_string())
            }
            SkinError::PasswordRequired => Self::unauthorized(error.to_string()),
            SkinError::MetadataAfterProvider(_) | SkinError::CredentialsAfterProvider(_) => {
                Self::incomplete_recovery(error.to_string())
            }
            SkinError::Ely(ingot_accounts::auth::ely::ElyError::UploadedNotApplied { .. }) => {
                Self::incomplete_recovery(error.to_string())
            }
            SkinError::Microsoft(_) | SkinError::Ely(_) => {
                Self::provider_unavailable(error.to_string())
            }
        }
    }
}

impl From<crate::app::accounts::DownloadError> for ApiError {
    fn from(error: crate::app::accounts::DownloadError) -> Self {
        use crate::app::accounts::DownloadError;
        match error {
            DownloadError::EmptyUrl
            | DownloadError::InvalidDataUrl
            | DownloadError::Decode(_)
            | DownloadError::InvalidDestination => Self::invalid_input(error.to_string()),
            DownloadError::Path(_) | DownloadError::Io(_) => {
                Self::storage_failure(error.to_string())
            }
            DownloadError::Transport(_) | DownloadError::Body(_) | DownloadError::HttpStatus(_) => {
                Self::provider_unavailable(error.to_string())
            }
            DownloadError::Task(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<ingot_accounts::account::credentials::CredentialError> for ApiError {
    fn from(error: ingot_accounts::account::credentials::CredentialError) -> Self {
        use ingot_accounts::account::credentials::CredentialError;
        match error {
            CredentialError::Missing => Self::not_found("No credentials found"),
            CredentialError::Vault { .. } => Self::provider_unavailable(error.to_string()),
            CredentialError::Decode(_) | CredentialError::Encode(_) => {
                Self::storage_failure(error.to_string())
            }
        }
    }
}

impl From<ingot_accounts::account::AccountError> for ApiError {
    fn from(error: ingot_accounts::account::AccountError) -> Self {
        use ingot_accounts::account::{policy::AccountPolicyError, AccountError};
        match error {
            AccountError::Policy(AccountPolicyError::NotFound) => {
                Self::not_found("Account not found")
            }
            AccountError::Policy(AccountPolicyError::DuplicateReorderId(id)) => {
                Self::invalid_input(format!("Duplicate account ID in reorder list: {id}"))
            }
            AccountError::Validation { .. } => Self::storage_failure(error.to_string()),
            AccountError::Io { .. } | AccountError::Decode { .. } | AccountError::Encode(_) => {
                Self::storage_failure(error.to_string())
            }
            AccountError::LockPoisoned => Self::internal(error.to_string()),
        }
    }
}

impl From<ingot_accounts::account::recovery::RecoveryError> for ApiError {
    fn from(error: ingot_accounts::account::recovery::RecoveryError) -> Self {
        use ingot_accounts::account::recovery::RecoveryError;
        match error {
            RecoveryError::Credentials(e) => Self::from(e),
            RecoveryError::Account(e) => Self::from(e),
            RecoveryError::PendingGrant(e) => Self::from(e),
            RecoveryError::PendingCredentials(_) | RecoveryError::Profile(_) => {
                Self::incomplete_recovery(error.to_string())
            }
            RecoveryError::Discovery(_) | RecoveryError::Invalid => {
                Self::storage_failure(error.to_string())
            }
            RecoveryError::LockPoisoned => Self::internal(error.to_string()),
        }
    }
}

impl From<ingot_accounts::account::pending_login::PendingGrantError> for ApiError {
    fn from(error: ingot_accounts::account::pending_login::PendingGrantError) -> Self {
        use ingot_accounts::account::pending_login::PendingGrantError;
        match error {
            PendingGrantError::Credentials(e) => Self::from(e),
            PendingGrantError::NotFound => Self::not_found(error.to_string()),
            PendingGrantError::AlreadyPending => Self::conflict(error.to_string()),
            PendingGrantError::Invalid | PendingGrantError::Expired | PendingGrantError::Legacy => {
                Self::invalid_input(error.to_string())
            }
        }
    }
}

impl From<ingot_accounts::account::session::SessionError> for ApiError {
    fn from(error: ingot_accounts::account::session::SessionError) -> Self {
        use ingot_accounts::account::session::SessionError;
        match error {
            SessionError::Credentials(e) => Self::from(e),
            SessionError::Recovery(e) => Self::from(e),
            SessionError::PendingGrant(e) => Self::from(e),
            SessionError::RotationCheckpoint(e) => Self::incomplete_recovery(e.to_string()),
            SessionError::PendingGrantCleanup(e) | SessionError::ResumeCheckpoint(e) => {
                Self::incomplete_recovery(e.to_string())
            }
            SessionError::AccountNotFound => Self::not_found("Account not found"),
            SessionError::NoActiveAccount => Self::not_found("No active account selected"),
            SessionError::Cancelled => Self::cancelled("Cancelled"),
            SessionError::Validation(e) => Self::invalid_input(e.to_string()),
            SessionError::InvalidLogin
            | SessionError::MissingMinecraftProfile
            | SessionError::NotMicrosoft
            | SessionError::MissingRefreshToken => Self::invalid_input(error.to_string()),
            SessionError::Ownership(_) => Self::conflict(error.to_string()),
            SessionError::Provider(_) | SessionError::Login(_) => {
                Self::provider_unavailable(error.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrupt_persisted_account_semantics_are_a_storage_failure() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("accounts.json");
        let invalid = ingot_accounts::account::AccountProfile {
            id: "unknown:id".into(),
            account_type: "unknown".into(),
            username: "Player".into(),
            uuid: "id".into(),
            skin_url: None,
            is_active: true,
            created_at: 0,
        };
        let bytes = serde_json::to_vec(&[invalid]).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        let repository = ingot_accounts::account::repository::AccountRepository::new(path.clone());
        let error = repository.load().unwrap_err();
        assert!(matches!(
            error,
            ingot_accounts::account::AccountError::Validation { .. }
        ));
        let ipc = ApiError::from(crate::app::accounts::ProfileError::Recovery(
            ingot_accounts::account::recovery::RecoveryError::Account(error),
        ));
        assert!(matches!(ipc, ApiError::StorageFailure { .. }));
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }

    #[test]
    fn invalid_profile_input_remains_distinct_from_storage_corruption() {
        use ingot_accounts::account::validation::AccountValidationError;
        let invalid = crate::app::accounts::ProfileError::Validation(
            AccountValidationError::UnsupportedKind("unknown".into()),
        );
        assert!(matches!(
            ApiError::from(invalid),
            ApiError::InvalidInput { .. }
        ));
    }
}
