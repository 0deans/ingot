//! Microsoft (official Minecraft) sign-in.
//!
//! Uses the OAuth device code flow: the user opens microsoft.com/link in any browser and
//! enters a short code, so no embedded webview or redirect handling is needed (works on
//! desktop and Android alike). The Microsoft token is then exchanged along the chain
//! Xbox Live -> XSTS -> Minecraft services.

use serde::Deserialize;
use serde_json::json;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Azure app (client) ID. It must be registered as a public client and approved by Mojang
/// for the Minecraft services API (https://aka.ms/mce-reviewappid). Set at build time.
const CLIENT_ID: Option<&str> = option_env!("INGOT_MSA_CLIENT_ID");

const SCOPE: &str = "XboxLive.signin offline_access";
const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBL_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC_LOGIN_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
const MC_SKINS_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins";

/// What the user needs to finish signing in on microsoft.com/link
#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct MicrosoftDeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    pub device_code: String,
    /// Seconds between polls
    pub interval: u32,
    /// Seconds until the code expires
    pub expires_in: u32,
}

/// A signed-in Minecraft session, ready to launch the game with
#[derive(Debug, Clone)]
pub struct MinecraftSession {
    pub access_token: String,
    /// Unix seconds
    pub expires_at: u64,
    pub refresh_token: String,
    pub uuid: String,
    pub username: String,
    pub skin_url: Option<String>,
}

#[derive(Deserialize)]
struct MsTokenResponse {
    access_token: String,
    refresh_token: String,
}

#[derive(Deserialize)]
struct MsErrorResponse {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XboxTokenResponse {
    token: String,
    display_claims: XboxDisplayClaims,
}

#[derive(Deserialize)]
struct XboxDisplayClaims {
    xui: Vec<XboxUserInfo>,
}

#[derive(Deserialize)]
struct XboxUserInfo {
    uhs: String,
}

#[derive(Deserialize)]
struct XstsError {
    #[serde(rename = "XErr")]
    xerr: u64,
}

#[derive(Deserialize)]
struct McLoginResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Deserialize)]
struct McProfile {
    id: String,
    name: String,
    #[serde(default)]
    skins: Vec<McSkin>,
}

#[derive(Deserialize)]
struct McSkin {
    state: String,
    url: String,
}

fn client_id() -> Result<&'static str, String> {
    CLIENT_ID
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| "Microsoft sign-in isn't configured in this build".to_string())
}

fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_default()
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Step 1: ask Microsoft for a code the user enters on microsoft.com/link
pub async fn start_device_login() -> Result<MicrosoftDeviceCode, String> {
    #[derive(Deserialize)]
    struct Response {
        user_code: String,
        verification_uri: String,
        device_code: String,
        interval: u32,
        expires_in: u32,
    }

    let res = http()
        .post(DEVICE_CODE_URL)
        .form(&[("client_id", client_id()?), ("scope", SCOPE)])
        .send()
        .await
        .map_err(|e| format!("Couldn't reach Microsoft: {e}"))?;

    if !res.status().is_success() {
        return Err(ms_error(res).await);
    }

    let r: Response = res
        .json()
        .await
        .map_err(|e| format!("Unexpected response from Microsoft: {e}"))?;

    Ok(MicrosoftDeviceCode {
        user_code: r.user_code,
        verification_uri: r.verification_uri,
        device_code: r.device_code,
        interval: r.interval.max(1),
        expires_in: r.expires_in,
    })
}

/// Step 2: wait until the user has entered the code, then sign in to Minecraft.
/// `is_cancelled` is checked between polls so a closed dialog stops the wait.
pub async fn finish_device_login(
    code: &MicrosoftDeviceCode,
    is_cancelled: impl Fn() -> bool,
) -> Result<MinecraftSession, String> {
    let client_id = client_id()?;
    let deadline = Instant::now() + Duration::from_secs(code.expires_in.into());
    let mut interval = Duration::from_secs(code.interval.into());

    let ms_token = loop {
        tokio::time::sleep(interval).await;
        if is_cancelled() {
            return Err("Sign-in was cancelled".to_string());
        }
        if Instant::now() >= deadline {
            return Err("The code expired. Please try again.".to_string());
        }

        let res = http()
            .post(TOKEN_URL)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", client_id),
                ("device_code", code.device_code.as_str()),
            ])
            .send()
            .await
            .map_err(|e| format!("Couldn't reach Microsoft: {e}"))?;

        if res.status().is_success() {
            break res
                .json::<MsTokenResponse>()
                .await
                .map_err(|e| format!("Unexpected response from Microsoft: {e}"))?;
        }

        let err: MsErrorResponse = res
            .json()
            .await
            .map_err(|e| format!("Unexpected response from Microsoft: {e}"))?;
        match err.error.as_str() {
            "authorization_pending" => continue,
            "slow_down" => interval += Duration::from_secs(5),
            "authorization_declined" => return Err("Sign-in was declined".to_string()),
            "expired_token" => return Err("The code expired. Please try again.".to_string()),
            _ => return Err(err.error_description.unwrap_or(err.error)),
        }
    };

    minecraft_session(ms_token).await
}

/// Gets a fresh Minecraft session from a stored Microsoft refresh token
pub async fn refresh(refresh_token: &str) -> Result<MinecraftSession, String> {
    let res = http()
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", client_id()?),
            ("refresh_token", refresh_token),
            ("scope", SCOPE),
        ])
        .send()
        .await
        .map_err(|e| format!("Couldn't reach Microsoft: {e}"))?;

    if !res.status().is_success() {
        return Err(format!(
            "Your Microsoft session expired, please sign in again ({})",
            ms_error(res).await
        ));
    }

    let ms_token: MsTokenResponse = res
        .json()
        .await
        .map_err(|e| format!("Unexpected response from Microsoft: {e}"))?;

    minecraft_session(ms_token).await
}

/// Microsoft token -> Xbox Live -> XSTS -> Minecraft token + profile
async fn minecraft_session(ms: MsTokenResponse) -> Result<MinecraftSession, String> {
    let client = http();

    let xbl: XboxTokenResponse = client
        .post(XBL_URL)
        .json(&json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={}", ms.access_token),
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT",
        }))
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Xbox Live sign-in failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Unexpected response from Xbox Live: {e}"))?;

    let xsts_res = client
        .post(XSTS_URL)
        .json(&json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl.token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT",
        }))
        .send()
        .await
        .map_err(|e| format!("Couldn't reach Xbox Live: {e}"))?;

    if xsts_res.status() == reqwest::StatusCode::UNAUTHORIZED {
        let xerr = xsts_res.json::<XstsError>().await.map(|e| e.xerr).unwrap_or(0);
        return Err(xsts_error_message(xerr));
    }

    let xsts: XboxTokenResponse = xsts_res
        .error_for_status()
        .map_err(|e| format!("Xbox Live sign-in failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Unexpected response from Xbox Live: {e}"))?;

    let uhs = xsts
        .display_claims
        .xui
        .first()
        .map(|x| x.uhs.clone())
        .ok_or("Xbox Live returned no user")?;

    let mc_res = client
        .post(MC_LOGIN_URL)
        .json(&json!({ "identityToken": format!("XBL3.0 x={uhs};{}", xsts.token) }))
        .send()
        .await
        .map_err(|e| format!("Couldn't reach Minecraft services: {e}"))?;

    if mc_res.status() == reqwest::StatusCode::FORBIDDEN {
        return Err("Minecraft services rejected this launcher's app registration".to_string());
    }

    let mc: McLoginResponse = mc_res
        .error_for_status()
        .map_err(|e| format!("Minecraft sign-in failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Unexpected response from Minecraft services: {e}"))?;

    let profile_res = client
        .get(MC_PROFILE_URL)
        .bearer_auth(&mc.access_token)
        .send()
        .await
        .map_err(|e| format!("Couldn't reach Minecraft services: {e}"))?;

    // No profile means the account doesn't own Java Edition (or never picked a name)
    if profile_res.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(
            "This Microsoft account doesn't own Minecraft: Java Edition, or hasn't set up a profile on minecraft.net yet"
                .to_string(),
        );
    }

    let profile: McProfile = profile_res
        .error_for_status()
        .map_err(|e| format!("Couldn't load the Minecraft profile: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Unexpected response from Minecraft services: {e}"))?;

    let skin_url = profile
        .skins
        .into_iter()
        .find(|s| s.state == "ACTIVE")
        .map(|s| s.url);

    Ok(MinecraftSession {
        access_token: mc.access_token,
        expires_at: now() + mc.expires_in,
        refresh_token: ms.refresh_token,
        uuid: profile.id,
        username: profile.name,
        skin_url,
    })
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
        let reason = res.json::<McError>().await.ok().and_then(|e| e.error_message);
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

/// The Xbox user ID the game reports as `auth_xuid`, read from the Minecraft token (a JWT)
pub fn xuid_from_token(access_token: &str) -> Option<String> {
    use base64::Engine;

    let payload = access_token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    match &claims["xuid"] {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

async fn ms_error(res: reqwest::Response) -> String {
    let status = res.status();
    match res.json::<MsErrorResponse>().await {
        Ok(e) => e.error_description.unwrap_or(e.error),
        Err(_) => format!("Microsoft responded with HTTP {status}"),
    }
}

/// XSTS refuses some accounts with a numeric reason
fn xsts_error_message(xerr: u64) -> String {
    match xerr {
        2148916233 => "This Microsoft account has no Xbox profile. Sign in once at xbox.com to create one.".to_string(),
        2148916235 => "Xbox Live isn't available in your country".to_string(),
        2148916236 | 2148916237 => "This account needs adult verification on xbox.com".to_string(),
        2148916238 => "This is a child account. An adult must add it to a Microsoft family first.".to_string(),
        _ => format!("Xbox Live refused the sign-in (error {xerr})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    fn jwt(claims: serde_json::Value) -> String {
        let b64 = |v: &[u8]| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(v);
        format!("{}.{}.sig", b64(br#"{"alg":"none"}"#), b64(claims.to_string().as_bytes()))
    }

    #[test]
    fn reads_xuid_from_minecraft_token() {
        assert_eq!(
            xuid_from_token(&jwt(json!({ "xuid": "2535400000000000" }))).as_deref(),
            Some("2535400000000000")
        );
        assert_eq!(
            xuid_from_token(&jwt(json!({ "xuid": 2535400000000000u64 }))).as_deref(),
            Some("2535400000000000")
        );
    }

    #[test]
    fn missing_or_malformed_xuid_is_none() {
        assert_eq!(xuid_from_token(&jwt(json!({ "sub": "x" }))), None);
        assert_eq!(xuid_from_token("not-a-jwt"), None);
    }
}
