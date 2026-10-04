//! Application-owned native sign-in providers over shared HTTP configuration.
use super::{
    ely::session::ElySession,
    microsoft::{
        MicrosoftDeviceCode, MicrosoftError, MicrosoftGrant, MicrosoftSession, MinecraftSession,
    },
};
use crate::account::provider::{
    AuthenticateResponse, Cancelled, ProviderError, ProviderFuture, ProviderOperation,
    RefreshResponse, SessionProvider, TokenValidity,
};

pub(crate) struct NativeSessionProvider {
    ely: ElySession,
    microsoft: MicrosoftSession,
}
impl NativeSessionProvider {
    pub(crate) fn new(client: reqwest::Client) -> Self {
        Self {
            ely: ElySession::new(client.clone(), "https://authserver.ely.by".to_owned()),
            microsoft: MicrosoftSession::native(client),
        }
    }
}

fn microsoft_error(operation: ProviderOperation, error: MicrosoftError) -> ProviderError {
    match error {
        MicrosoftError::Cancelled => ProviderError::Cancelled,
        source => ProviderError::Microsoft { operation, source },
    }
}

impl SessionProvider for NativeSessionProvider {
    fn ely_login<'a>(
        &'a self,
        username: &'a str,
        password: &'a str,
        client_token: &'a str,
    ) -> ProviderFuture<'a, AuthenticateResponse> {
        Box::pin(self.ely.authenticate(username, password, client_token))
    }
    fn ely_validate<'a>(
        &'a self,
        access: &'a str,
        client: &'a str,
    ) -> ProviderFuture<'a, TokenValidity> {
        Box::pin(self.ely.validate(access, client))
    }
    fn ely_refresh<'a>(
        &'a self,
        access: &'a str,
        client: &'a str,
    ) -> ProviderFuture<'a, RefreshResponse> {
        Box::pin(self.ely.refresh(access, client))
    }
    fn microsoft_start(&self) -> ProviderFuture<'_, MicrosoftDeviceCode> {
        Box::pin(async {
            self.microsoft
                .start()
                .await
                .map_err(|error| microsoft_error(ProviderOperation::MicrosoftStart, error))
        })
    }
    fn microsoft_finish<'a>(
        &'a self,
        code: &'a MicrosoftDeviceCode,
        cancelled: Cancelled<'a>,
    ) -> ProviderFuture<'a, MicrosoftGrant> {
        Box::pin(async move {
            let result = self.microsoft.redeem(code, cancelled).await;
            if cancelled() {
                return Err(ProviderError::Cancelled);
            }
            result.map_err(|error| microsoft_error(ProviderOperation::MicrosoftFinish, error))
        })
    }
    fn microsoft_refresh<'a>(&'a self, refresh: &'a str) -> ProviderFuture<'a, MicrosoftGrant> {
        Box::pin(async move {
            self.microsoft
                .refresh_grant(refresh)
                .await
                .map_err(|error| microsoft_error(ProviderOperation::MicrosoftRefresh, error))
        })
    }

    fn microsoft_exchange<'a>(
        &'a self,
        grant: MicrosoftGrant,
        cancelled: Cancelled<'a>,
    ) -> ProviderFuture<'a, MinecraftSession> {
        Box::pin(async move {
            self.microsoft
                .exchange_grant(grant, cancelled)
                .await
                .map_err(|error| microsoft_error(ProviderOperation::MicrosoftExchange, error))
        })
    }
}
