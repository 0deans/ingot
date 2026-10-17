//! Microsoft and Minecraft Services skin upload and change adapter.
use super::exchange::McProfile;
use reqwest::{Client, StatusCode};
use serde_json::json;
use std::time::Duration;

pub const DEFAULT_MINECRAFT_SERVICES_BASE: &str = "https://api.minecraftservices.com";

#[derive(thiserror::Error)]
pub enum MicrosoftSkinError {
    #[error("Couldn't reach Minecraft services")]
    Transport(#[source] reqwest::Error),

    #[error("Failed to prepare the skin file")]
    Multipart(#[source] reqwest::Error),

    #[error("Minecraft refused the skin (HTTP {0})")]
    HttpStatus(StatusCode),

    #[error("Unexpected response from Minecraft services")]
    Response(#[source] reqwest::Error),
}

impl std::fmt::Debug for MicrosoftSkinError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // A malformed profile can echo credentials in its decode source.
        formatter
            .debug_struct("MicrosoftSkinError")
            .field("message", &self.to_string())
            .finish()
    }
}

impl From<MicrosoftSkinError> for String {
    fn from(error: MicrosoftSkinError) -> Self {
        error.to_string()
    }
}

pub struct MicrosoftSkinClient {
    client: Client,
    base_url: String,
}

impl Default for MicrosoftSkinClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MicrosoftSkinClient {
    pub fn new() -> Self {
        Self::with_endpoint(
            Client::builder()
                .user_agent(crate::USER_AGENT)
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            DEFAULT_MINECRAFT_SERVICES_BASE,
        )
    }

    pub fn with_endpoint(client: Client, base_url: impl Into<String>) -> Self {
        Self {
            client,
            base_url: base_url.into(),
        }
    }

    /// Sets the account's skin to a public PNG (e.g. a catalog skin). Returns the new skin URL.
    pub async fn set_skin_from_url(
        &self,
        access_token: &str,
        skin_url: &str,
        slim: bool,
    ) -> Result<Option<String>, MicrosoftSkinError> {
        let endpoint = format!("{}/minecraft/profile/skins", self.base_url);
        let res = self
            .client
            .post(&endpoint)
            .bearer_auth(access_token)
            .json(&json!({
                "variant": skin_variant(slim),
                "url": skin_url
            }))
            .send()
            .await
            .map_err(|error| MicrosoftSkinError::Transport(error.without_url()))?;

        active_skin_url(res).await
    }

    /// Uploads a PNG as the account's skin. Returns the new skin URL.
    pub async fn upload_skin(
        &self,
        access_token: &str,
        png: Vec<u8>,
        slim: bool,
    ) -> Result<Option<String>, MicrosoftSkinError> {
        let file = reqwest::multipart::Part::bytes(png)
            .file_name("skin.png")
            .mime_str("image/png")
            .map_err(|error| MicrosoftSkinError::Multipart(error.without_url()))?;

        let form = reqwest::multipart::Form::new()
            .text("variant", skin_variant(slim))
            .part("file", file);

        let endpoint = format!("{}/minecraft/profile/skins", self.base_url);
        let res = self
            .client
            .post(&endpoint)
            .bearer_auth(access_token)
            .multipart(form)
            .send()
            .await
            .map_err(|error| MicrosoftSkinError::Transport(error.without_url()))?;

        active_skin_url(res).await
    }
}

/// Convenience function using the default client and production endpoint.
pub async fn set_skin_from_url(
    access_token: &str,
    skin_url: &str,
    slim: bool,
) -> Result<Option<String>, MicrosoftSkinError> {
    MicrosoftSkinClient::new()
        .set_skin_from_url(access_token, skin_url, slim)
        .await
}

/// Convenience function using the default client and production endpoint.
pub async fn upload_skin(
    access_token: &str,
    png: Vec<u8>,
    slim: bool,
) -> Result<Option<String>, MicrosoftSkinError> {
    MicrosoftSkinClient::new()
        .upload_skin(access_token, png, slim)
        .await
}

pub(crate) fn skin_variant(slim: bool) -> &'static str {
    if slim {
        "slim"
    } else {
        "classic"
    }
}

/// Reads the updated profile a skin change returns, or the reason it was refused.
async fn active_skin_url(res: reqwest::Response) -> Result<Option<String>, MicrosoftSkinError> {
    let status = res.status();
    if !status.is_success() {
        return Err(MicrosoftSkinError::HttpStatus(status));
    }

    let profile: McProfile = res
        .json()
        .await
        .map_err(|error| MicrosoftSkinError::Response(error.without_url()))?;

    Ok(profile
        .skins
        .into_iter()
        .find(|s| s.state == "ACTIVE")
        .map(|s| s.url))
}
