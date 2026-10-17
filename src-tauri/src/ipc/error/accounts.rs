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
            SkinError::Ely(crate::auth::ely::ElyError::UploadedNotApplied { .. }) => {
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

impl From<crate::account::credentials::CredentialError> for ApiError {
    fn from(error: crate::account::credentials::CredentialError) -> Self {
        match error {
            crate::account::credentials::CredentialError::Missing => {
                Self::not_found("No credentials found")
            }
            crate::account::credentials::CredentialError::Vault { .. } => {
                Self::provider_unavailable(error.to_string())
            }
            crate::account::credentials::CredentialError::Decode(_)
            | crate::account::credentials::CredentialError::Encode(_) => {
                Self::storage_failure(error.to_string())
            }
        }
    }
}

impl From<crate::account::AccountError> for ApiError {
    fn from(error: crate::account::AccountError) -> Self {
        match error {
            crate::account::AccountError::Policy(
                crate::account::policy::AccountPolicyError::NotFound,
            ) => Self::not_found("Account not found"),
            crate::account::AccountError::Policy(
                crate::account::policy::AccountPolicyError::DuplicateReorderId(id),
            ) => Self::invalid_input(format!("Duplicate account ID in reorder list: {id}")),
            crate::account::AccountError::Validation { .. } => {
                Self::storage_failure(error.to_string())
            }
            crate::account::AccountError::Io { .. }
            | crate::account::AccountError::Decode { .. }
            | crate::account::AccountError::Encode(_) => Self::storage_failure(error.to_string()),
            crate::account::AccountError::LockPoisoned => Self::internal(error.to_string()),
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
            crate::account::pending_login::PendingGrantError::NotFound => {
                Self::not_found(error.to_string())
            }
            crate::account::pending_login::PendingGrantError::AlreadyPending => {
                Self::conflict(error.to_string())
            }
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
            crate::account::session::SessionError::RotationCheckpoint(e) => {
                Self::incomplete_recovery(e.to_string())
            }
            crate::account::session::SessionError::PendingGrantCleanup(e)
            | crate::account::session::SessionError::ResumeCheckpoint(e) => {
                Self::incomplete_recovery(e.to_string())
            }
            crate::account::session::SessionError::AccountNotFound => {
                Self::not_found("Account not found")
            }
            crate::account::session::SessionError::NoActiveAccount => {
                Self::not_found("No active account selected")
            }
            crate::account::session::SessionError::Cancelled => Self::cancelled("Cancelled"),
            crate::account::session::SessionError::Validation(e) => {
                Self::invalid_input(e.to_string())
            }
            crate::account::session::SessionError::InvalidLogin
            | crate::account::session::SessionError::MissingMinecraftProfile
            | crate::account::session::SessionError::NotMicrosoft
            | crate::account::session::SessionError::MissingRefreshToken => {
                Self::invalid_input(error.to_string())
            }
            crate::account::session::SessionError::Ownership(_) => {
                Self::conflict(error.to_string())
            }
            crate::account::session::SessionError::Provider(_)
            | crate::account::session::SessionError::Login(_) => {
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
        let invalid = crate::account::AccountProfile {
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
        let repository = crate::account::repository::AccountRepository::new(path.clone());
        let error = repository.load().unwrap_err();
        assert!(matches!(
            error,
            crate::account::AccountError::Validation { .. }
        ));
        let ipc = ApiError::from(crate::app::accounts::ProfileError::Recovery(
            crate::account::recovery::RecoveryError::Account(error),
        ));
        assert!(matches!(ipc, ApiError::StorageFailure { .. }));
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }

    #[test]
    fn invalid_profile_input_remains_distinct_from_storage_corruption() {
        let invalid = crate::app::accounts::ProfileError::Validation(
            crate::account::validation::AccountValidationError::UnsupportedKind("unknown".into()),
        );
        assert!(matches!(
            ApiError::from(invalid),
            ApiError::InvalidInput { .. }
        ));
    }
}
