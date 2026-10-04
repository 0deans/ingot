//! Existing skin upload adapter; separate from session sign-in.
use super::exchange::McProfile;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;
const MC_SKINS_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins";
fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_default()
}

/// Sets the account's skin to a public PNG (e.g. a catalog skin). Returns the new skin URL.
pub async fn set_skin_from_url(
    access_token: &str,
    skin_url: &str,
    slim: bool,
) -> Result<Option<String>, String> {
    let res = http()
        .post(MC_SKINS_URL)
        .bearer_auth(access_token)
        .json(&json!({ "variant": skin_variant(slim), "url": skin_url }))
        .send()
        .await
        .map_err(|e| format!("Couldn't reach Minecraft services: {e}"))?;

    active_skin_url(res).await
}

/// Uploads a PNG as the account's skin. Returns the new skin URL.
pub async fn upload_skin(
    access_token: &str,
    png: Vec<u8>,
    slim: bool,
) -> Result<Option<String>, String> {
    let file = reqwest::multipart::Part::bytes(png)
        .file_name("skin.png")
        .mime_str("image/png")
        .map_err(|e| format!("Failed to prepare the skin file: {e}"))?;
    let form = reqwest::multipart::Form::new()
        .text("variant", skin_variant(slim))
        .part("file", file);

    let res = http()
        .post(MC_SKINS_URL)
        .bearer_auth(access_token)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Couldn't reach Minecraft services: {e}"))?;

    active_skin_url(res).await
}

fn skin_variant(slim: bool) -> &'static str {
    if slim {
        "slim"
    } else {
        "classic"
    }
}

/// Reads the updated profile a skin change returns, or the reason it was refused
async fn active_skin_url(res: reqwest::Response) -> Result<Option<String>, String> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct McError {
        error_message: Option<String>,
    }

    let status = res.status();
    if !status.is_success() {
        let reason = res
            .json::<McError>()
            .await
            .ok()
            .and_then(|e| e.error_message);
        return Err(match reason {
            Some(reason) => format!("Minecraft refused the skin: {reason}"),
            None => format!("Minecraft refused the skin (HTTP {status})"),
        });
    }

    let profile: McProfile = res
        .json()
        .await
        .map_err(|e| format!("Unexpected response from Minecraft services: {e}"))?;
    Ok(profile
        .skins
        .into_iter()
        .find(|s| s.state == "ACTIVE")
        .map(|s| s.url))
}
