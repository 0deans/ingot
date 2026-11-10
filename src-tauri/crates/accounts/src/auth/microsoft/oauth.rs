use super::{
    client::MicrosoftSession,
    error::{MicrosoftError, Stage},
    MicrosoftDeviceCode, MicrosoftGrant,
};
use crate::account::provider::Cancelled;
use serde::Deserialize;
use std::time::Duration;

const SCOPE: &str = "XboxLive.signin offline_access";
#[derive(Deserialize)]
pub(super) struct MsToken {
    pub access_token: String,
    pub refresh_token: String,
}
#[derive(Deserialize)]
struct OAuthError {
    error: String,
}
#[derive(Deserialize)]
struct DeviceCode {
    user_code: String,
    verification_uri: String,
    device_code: String,
    interval: u32,
    expires_in: u32,
}

impl MicrosoftSession {
    pub(crate) async fn start(&self) -> Result<MicrosoftDeviceCode, MicrosoftError> {
        let stage = Stage::DeviceCode;
        let request = self
            .client
            .post(&self.endpoints.device_code)
            .form(&[("client_id", self.client_id()?), ("scope", SCOPE)]);
        let response: DeviceCode = Self::decode(self.send(request, stage).await?, stage).await?;
        Self::nonempty(&response.device_code, stage, "device code")?;
        Self::nonempty(&response.user_code, stage, "user code")?;
        Self::nonempty(&response.verification_uri, stage, "verification URI")?;
        Ok(MicrosoftDeviceCode {
            user_code: response.user_code,
            verification_uri: response.verification_uri,
            device_code: response.device_code,
            interval: response.interval.max(1),
            expires_in: response.expires_in,
        })
    }

    pub(crate) async fn redeem(
        &self,
        code: &MicrosoftDeviceCode,
        cancelled: Cancelled<'_>,
    ) -> Result<MicrosoftGrant, MicrosoftError> {
        let client_id = self.client_id()?;
        Self::nonempty(&code.device_code, Stage::DeviceToken, "device code")?;
        let deadline = self
            .clock
            .elapsed()
            .saturating_add(Duration::from_secs(code.expires_in.into()));
        let mut interval = Duration::from_secs(code.interval.max(1).into());
        loop {
            if cancelled() {
                return Err(MicrosoftError::Cancelled);
            }
            let remaining = deadline.saturating_sub(self.clock.elapsed());
            if remaining.is_zero() {
                return Err(MicrosoftError::CodeExpired);
            }
            self.clock.sleep(interval.min(remaining)).await;
            if cancelled() {
                return Err(MicrosoftError::Cancelled);
            }
            if self.clock.elapsed() >= deadline {
                return Err(MicrosoftError::CodeExpired);
            }
            let request = self.client.post(&self.endpoints.token).form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", client_id),
                ("device_code", code.device_code.as_str()),
            ]);
            let response = self.send(request, Stage::DeviceToken).await?;
            if cancelled() {
                return Err(MicrosoftError::Cancelled);
            }
            if response.status().is_success() {
                let token: MsToken = Self::body(response, Stage::DeviceToken).await?;
                return MicrosoftGrant::at_stage(
                    token.access_token,
                    token.refresh_token,
                    Stage::DeviceToken,
                );
            }
            let status = response.status();
            if !status.is_client_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Err(MicrosoftError::Rejected {
                    stage: Stage::DeviceToken,
                    status,
                });
            }
            let error: OAuthError = Self::body(response, Stage::DeviceToken).await?;
            match error.error.as_str() {
                "authorization_pending" => {}
                "slow_down" => interval = interval.saturating_add(Duration::from_secs(5)),
                "authorization_declined" => return Err(MicrosoftError::Declined),
                "expired_token" => return Err(MicrosoftError::CodeExpired),
                "bad_verification_code" => return Err(MicrosoftError::InvalidCode),
                _ => {
                    return Err(MicrosoftError::Rejected {
                        stage: Stage::DeviceToken,
                        status,
                    })
                }
            }
        }
    }

    pub(crate) async fn refresh_grant(
        &self,
        refresh: &str,
    ) -> Result<MicrosoftGrant, MicrosoftError> {
        let stage = Stage::Refresh;
        Self::nonempty(refresh, stage, "refresh token")?;
        let request = self.client.post(&self.endpoints.token).form(&[
            ("grant_type", "refresh_token"),
            ("client_id", self.client_id()?),
            ("refresh_token", refresh),
            ("scope", SCOPE),
        ]);
        let response = self.send(request, stage).await?;
        if !response.status().is_success() {
            let status = response.status();
            if status == reqwest::StatusCode::BAD_REQUEST {
                let error: OAuthError = Self::body(response, stage).await?;
                if error.error == "invalid_grant" {
                    return Err(MicrosoftError::RefreshExpired);
                }
            }
            return Err(MicrosoftError::Rejected { stage, status });
        }
        let token: MsToken = Self::body(response, stage).await?;
        MicrosoftGrant::new(token.access_token, token.refresh_token)
    }
}
