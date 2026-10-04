//! Typed Ely session protocol, sharing the application client and injectable base URL.
use crate::account::provider::{
    AuthenticateResponse, ProviderError, ProviderOperation, RefreshResponse, TokenValidity,
};
use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthenticateRequest<'a> {
    username: &'a str,
    password: &'a str,
    client_token: &'a str,
    request_user: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RefreshRequest<'a> {
    access_token: &'a str,
    client_token: &'a str,
    request_user: bool,
}

pub(crate) struct ElySession {
    client: Client,
    base: String,
}

impl ElySession {
    pub(crate) fn new(client: Client, base: String) -> Self {
        Self { client, base }
    }

    async fn request(
        &self,
        path: &str,
        payload: &impl Serialize,
        operation: ProviderOperation,
    ) -> Result<Response, ProviderError> {
        self.client
            .post(format!("{}/auth/{path}", self.base))
            .json(payload)
            .send()
            .await
            .map_err(|source| ProviderError::Transport { operation, source })
    }

    pub(crate) async fn authenticate(
        &self,
        username: &str,
        password: &str,
        client_token: &str,
    ) -> Result<AuthenticateResponse, ProviderError> {
        let operation = ProviderOperation::ElySignIn;
        let response = self
            .request(
                "authenticate",
                &AuthenticateRequest {
                    username,
                    password,
                    client_token,
                    request_user: true,
                },
                operation,
            )
            .await?;
        if response.status() == StatusCode::UNAUTHORIZED {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct ErrorBody {
                error_message: Option<String>,
            }
            // Preserve the actionable two-factor condition without reflecting raw
            // server text (which could echo credentials) into the UI.
            if matches!(response.json::<ErrorBody>().await, Ok(body)
                if body.error_message.as_deref() == Some("Account protected with two factor auth."))
            {
                return Err(ProviderError::TwoFactorRequired);
            }
            return Err(ProviderError::Rejected {
                operation,
                status: StatusCode::UNAUTHORIZED,
            });
        }
        Self::decode(response, operation).await
    }

    pub(crate) async fn refresh(
        &self,
        access_token: &str,
        client_token: &str,
    ) -> Result<RefreshResponse, ProviderError> {
        let operation = ProviderOperation::ElyRefresh;
        let response = self
            .request(
                "refresh",
                &RefreshRequest {
                    access_token,
                    client_token,
                    request_user: true,
                },
                operation,
            )
            .await?;
        Self::decode(response, operation).await
    }

    pub(crate) async fn validate(
        &self,
        access_token: &str,
        client_token: &str,
    ) -> Result<TokenValidity, ProviderError> {
        let operation = ProviderOperation::ElyValidate;
        let response = self
            .request(
                "validate",
                &serde_json::json!({ "accessToken": access_token, "clientToken": client_token }),
                operation,
            )
            .await?;
        match response.status() {
            status if status.is_success() => Ok(TokenValidity::Valid),
            StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Ok(TokenValidity::Invalid)
            }
            status => Err(ProviderError::Rejected { operation, status }),
        }
    }

    async fn decode<T: serde::de::DeserializeOwned>(
        response: Response,
        operation: ProviderOperation,
    ) -> Result<T, ProviderError> {
        let status = response.status();
        if !status.is_success() {
            return Err(ProviderError::Rejected { operation, status });
        }
        response
            .json()
            .await
            .map_err(|source| ProviderError::Response { operation, source })
    }
}

#[cfg(test)]
mod tests;
