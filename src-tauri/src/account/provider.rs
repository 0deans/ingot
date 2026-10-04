//! Injectable sign-in/session effects. Provider protocol details belong in auth adapters.
use crate::auth::microsoft::{MicrosoftDeviceCode, MicrosoftGrant, MinecraftSession};
use serde::Deserialize;
use std::{future::Future, pin::Pin};

#[derive(Deserialize)]
pub(crate) struct ProfileInfo {
    pub id: String,
    pub name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthenticateResponse {
    pub access_token: String,
    pub client_token: String,
    pub selected_profile: Option<ProfileInfo>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RefreshResponse {
    pub access_token: String,
    pub client_token: String,
}

pub(crate) type ProviderFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ProviderError>> + Send + 'a>>;
pub(crate) type Cancelled<'a> = &'a (dyn Fn() -> bool + Send + Sync);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TokenValidity {
    Valid,
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProviderOperation {
    ElySignIn,
    ElyValidate,
    ElyRefresh,
    MicrosoftStart,
    MicrosoftFinish,
    MicrosoftRefresh,
    MicrosoftExchange,
}

impl std::fmt::Display for ProviderOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::ElySignIn => "Ely.by sign-in",
            Self::ElyValidate => "Ely.by token validation",
            Self::ElyRefresh => "Ely.by token refresh",
            Self::MicrosoftStart => "Microsoft sign-in setup",
            Self::MicrosoftFinish => "Microsoft sign-in",
            Self::MicrosoftRefresh => "Microsoft token refresh",
            Self::MicrosoftExchange => "Microsoft session exchange",
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProviderError {
    #[error("Could not reach the provider during {operation}")]
    Transport {
        operation: ProviderOperation,
        #[source]
        source: reqwest::Error,
    },
    #[error("{operation} was rejected (HTTP {status})")]
    Rejected {
        operation: ProviderOperation,
        status: reqwest::StatusCode,
    },
    #[error("Unexpected provider response during {operation}")]
    Response {
        operation: ProviderOperation,
        #[source]
        source: reqwest::Error,
    },
    #[error("Account protected with two factor auth.")]
    TwoFactorRequired,
    #[error("{operation} failed: {source}")]
    Microsoft {
        operation: ProviderOperation,
        #[source]
        source: crate::auth::microsoft::MicrosoftError,
    },
    #[error("Microsoft sign-in cancelled")]
    Cancelled,
}

pub(crate) trait SessionProvider: Send + Sync {
    fn ely_login<'a>(
        &'a self,
        username: &'a str,
        password: &'a str,
        client_token: &'a str,
    ) -> ProviderFuture<'a, AuthenticateResponse>;
    fn ely_validate<'a>(
        &'a self,
        access: &'a str,
        client: &'a str,
    ) -> ProviderFuture<'a, TokenValidity>;
    fn ely_refresh<'a>(
        &'a self,
        access: &'a str,
        client: &'a str,
    ) -> ProviderFuture<'a, RefreshResponse>;
    fn microsoft_start(&self) -> ProviderFuture<'_, MicrosoftDeviceCode>;
    fn microsoft_finish<'a>(
        &'a self,
        code: &'a MicrosoftDeviceCode,
        cancelled: Cancelled<'a>,
    ) -> ProviderFuture<'a, MicrosoftGrant>;
    /// Return the rotated grant without issuing any dependent exchange requests.
    fn microsoft_refresh<'a>(&'a self, refresh: &'a str) -> ProviderFuture<'a, MicrosoftGrant>;
    /// Call only after the service has durably checkpointed the refresh token.
    fn microsoft_exchange<'a>(
        &'a self,
        grant: MicrosoftGrant,
        cancelled: Cancelled<'a>,
    ) -> ProviderFuture<'a, MinecraftSession>;
}
