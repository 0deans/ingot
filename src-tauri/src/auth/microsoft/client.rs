use super::{
    clock::{ProtocolClock, SystemClock},
    error::{MicrosoftError, Stage},
};
use reqwest::{Client, RequestBuilder, Response};
use std::{sync::Arc, time::Duration};

pub(crate) struct Endpoints {
    pub(crate) device_code: String,
    pub(crate) token: String,
    pub(crate) xbox: String,
    pub(crate) xsts: String,
    pub(crate) minecraft_login: String,
    pub(crate) profile: String,
}
impl Default for Endpoints {
    fn default() -> Self {
        Self {
            device_code: "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode"
                .to_owned(),
            token: "https://login.microsoftonline.com/consumers/oauth2/v2.0/token".to_owned(),
            xbox: "https://user.auth.xboxlive.com/user/authenticate".to_owned(),
            xsts: "https://xsts.auth.xboxlive.com/xsts/authorize".to_owned(),
            minecraft_login: "https://api.minecraftservices.com/authentication/login_with_xbox"
                .to_owned(),
            profile: "https://api.minecraftservices.com/minecraft/profile".to_owned(),
        }
    }
}

pub(crate) struct MicrosoftSession {
    pub(super) client: Client,
    pub(super) endpoints: Endpoints,
    client_id: Option<String>,
    pub(super) clock: Arc<dyn ProtocolClock>,
}
impl MicrosoftSession {
    pub(crate) fn native(client: Client) -> Self {
        Self::new(
            client,
            Endpoints::default(),
            option_env!("INGOT_MSA_CLIENT_ID").map(str::to_owned),
            Arc::new(SystemClock::default()),
        )
    }
    pub(crate) fn new(
        client: Client,
        endpoints: Endpoints,
        client_id: Option<String>,
        clock: Arc<dyn ProtocolClock>,
    ) -> Self {
        Self {
            client,
            endpoints,
            client_id,
            clock,
        }
    }
    pub(super) fn client_id(&self) -> Result<&str, MicrosoftError> {
        self.client_id
            .as_deref()
            .filter(|id| !id.trim().is_empty())
            .ok_or(MicrosoftError::NotConfigured)
    }
    pub(super) async fn send(
        &self,
        request: RequestBuilder,
        stage: Stage,
    ) -> Result<Response, MicrosoftError> {
        // Preserve the provider's 30-second request/body timeout with a shared client.
        request
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|source| MicrosoftError::Transport { stage, source })
    }
    pub(super) async fn decode<T: serde::de::DeserializeOwned>(
        response: Response,
        stage: Stage,
    ) -> Result<T, MicrosoftError> {
        let status = response.status();
        if !status.is_success() {
            return Err(MicrosoftError::Rejected { stage, status });
        }
        Self::body(response, stage).await
    }
    pub(super) async fn body<T: serde::de::DeserializeOwned>(
        response: Response,
        stage: Stage,
    ) -> Result<T, MicrosoftError> {
        response
            .json()
            .await
            .map_err(|source| MicrosoftError::Response { stage, source })
    }
    pub(super) fn nonempty(
        value: &str,
        stage: Stage,
        field: &'static str,
    ) -> Result<(), MicrosoftError> {
        if value.trim().is_empty() {
            return Err(MicrosoftError::Protocol { stage, field });
        }
        Ok(())
    }
}
