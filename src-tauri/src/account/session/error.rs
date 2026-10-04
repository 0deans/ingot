use crate::account::{
    credentials::CredentialError, login::LoginError, operations::OperationError,
    provider::ProviderError, recovery::RecoveryError,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum SessionError {
    #[error(transparent)]
    Credentials(#[from] CredentialError),
    #[error(transparent)]
    Recovery(#[from] RecoveryError),
    #[error(
        "Microsoft refresh token rotated, but its recovery checkpoint could not be completed: {0}"
    )]
    RotationCheckpoint(#[source] RecoveryError),
    #[error(transparent)]
    PendingGrant(#[from] crate::account::pending_login::PendingGrantError),
    #[error("Account published, but pending Microsoft sign-in cleanup failed: {0}")]
    PendingGrantCleanup(#[source] crate::account::pending_login::PendingGrantError),
    #[error("Microsoft resume token rotated, but its checkpoint could not be completed: {0}")]
    ResumeCheckpoint(#[source] crate::account::pending_login::PendingGrantError),
    #[error(transparent)]
    Ownership(#[from] OperationError),
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error(transparent)]
    Login(#[from] LoginError),
    #[error("Username and password are required")]
    InvalidLogin,
    #[error("No Minecraft profile found for this Ely.by account")]
    MissingMinecraftProfile,
    #[error("No active account selected")]
    NoActiveAccount,
    #[error("Account not found")]
    AccountNotFound,
    #[error("This isn't a Microsoft account")]
    NotMicrosoft,
    #[error("Your Microsoft session expired, please sign in again")]
    MissingRefreshToken,
    #[error("Microsoft sign-in cancelled")]
    Cancelled,
}
