use serde::{Deserialize, Serialize};

const AUTH_SERVER: &str = "https://authserver.ely.by";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticateRequest<'a> {
    pub username: &'a str,
    pub password: &'a str,
    pub client_token: &'a str,
    pub request_user: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticateResponse {
    pub access_token: String,
    pub client_token: String,
    pub selected_profile: Option<ProfileInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElyErrorResponse {
    pub error: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshRequest<'a> {
    pub access_token: &'a str,
    pub client_token: &'a str,
    pub request_user: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResponse {
    pub access_token: String,
    pub client_token: String,
}

pub struct ElyAuthService {
    client: reqwest::Client,
}

impl Default for ElyAuthService {
    fn default() -> Self {
        Self::new()
    }
}

impl ElyAuthService {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Ingot-Launcher/0.1.0")
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn authenticate(
        &self,
        username: &str,
        password: &str,
        client_token: &str,
    ) -> Result<AuthenticateResponse, String> {
        let payload = AuthenticateRequest {
            username,
            password,
            client_token,
            request_user: true,
        };

        let response = self
            .client
            .post(format!("{AUTH_SERVER}/auth/authenticate"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Network request failed: {e}"))?;

        if !response.status().is_success() {
            if let Ok(err) = response.json::<ElyErrorResponse>().await {
                let msg = err
                    .error_message
                    .unwrap_or_else(|| format!("Authentication error: {}", err.error));
                return Err(msg);
            }
            return Err("Authentication failed with an unknown error".to_string());
        }

        let auth_response = response
            .json::<AuthenticateResponse>()
            .await
            .map_err(|e| format!("Failed to parse response from Ely.by: {e}"))?;

        Ok(auth_response)
    }

    pub async fn refresh(
        &self,
        access_token: &str,
        client_token: &str,
    ) -> Result<RefreshResponse, String> {
        let payload = RefreshRequest {
            access_token,
            client_token,
            request_user: true,
        };

        let response = self
            .client
            .post(format!("{AUTH_SERVER}/auth/refresh"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Network request failed: {e}"))?;

        if !response.status().is_success() {
            if let Ok(err) = response.json::<ElyErrorResponse>().await {
                let msg = err
                    .error_message
                    .unwrap_or_else(|| format!("Token refresh error: {}", err.error));
                return Err(msg);
            }
            return Err("Failed to refresh token".to_string());
        }

        let refresh_response = response
            .json::<RefreshResponse>()
            .await
            .map_err(|e| format!("Failed to parse refresh response: {e}"))?;

        Ok(refresh_response)
    }

    pub async fn validate(&self, access_token: &str, client_token: &str) -> bool {
        let payload = serde_json::json!({
            "accessToken": access_token,
            "clientToken": client_token,
        });

        match self
            .client
            .post(format!("{AUTH_SERVER}/auth/validate"))
            .json(&payload)
            .send()
            .await
        {
            Ok(res) => res.status().is_success(),
            Err(_) => false,
        }
    }

    pub async fn fetch_catalog(
        &self,
        page: u32,
        query: Option<String>,
        sort: Option<String>,
        model: Option<String>,
        uploader: Option<String>,
    ) -> Result<ElySkinsCatalogResponse, String> {
        let mut query_params: Vec<String> = Vec::new();

        if let Some(q) = query {
            let trimmed = q.trim();
            if !trimmed.is_empty() {
                let words: Vec<&str> = trimmed.split_whitespace().collect();
                if words.is_empty() {
                    query_params.push(format!("tags[0]={}", url_encode(trimmed)));
                } else {
                    for (idx, word) in words.iter().enumerate() {
                        query_params.push(format!("tags[{idx}]={}", url_encode(word)));
                    }
                }
            }
        }
        if let Some(s) = sort {
            let trimmed = s.trim().to_lowercase();
            if trimmed == "latest" || trimmed == "time" {
                query_params.push("sort=time".to_string());
            } else if !trimmed.is_empty() && trimmed != "default" {
                query_params.push("sort=best".to_string());
            }
        }
        if let Some(m) = model {
            let trimmed = m.trim().to_lowercase();
            if trimmed == "slim" || trimmed == "alex" {
                query_params.push("type=slim".to_string());
            } else if trimmed == "steve" || trimmed == "classic" || trimmed == "new" {
                query_params.push("type=new".to_string());
            }
        }
        if let Some(u) = uploader {
            let trimmed = u.trim();
            if !trimmed.is_empty() {
                query_params.push(format!("uploader={}", url_encode(trimmed)));
            }
        }

        let mut query_string = String::from("_url=%2Fskins");
        for p in query_params {
            query_string.push('&');
            query_string.push_str(&p);
        }

        let target_url = format!("https://ely.by/skins/get?{query_string}");
        let post_body = format!("skinsPage={page}");

        let res = self
            .client
            .post(&target_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("X-Requested-With", "XMLHttpRequest")
            .body(post_body)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch Ely.by skins catalog: {e}"))?;

        if !res.status().is_success() {
            return Err(format!(
                "Ely.by responded with HTTP status {}",
                res.status()
            ));
        }

        let raw = res
            .json::<ElyRawCatalogResponse>()
            .await
            .map_err(|e| format!("Failed to parse skins catalog response: {e}"))?;

        let items = raw
            .items
            .into_iter()
            .map(|i| ElySkinItem {
                id: i.id,
                skin_url: i.skin_url,
                is_slim: i.is_slim,
                count_wearers: i.count_wearers,
                count_cubes: i.count_cubes,
                count_views: i.count_views_total,
                tags: i.tags,
            })
            .collect();

        Ok(ElySkinsCatalogResponse {
            items,
            total_items: raw.total_items.unwrap_or(0),
            current_page: raw.current.unwrap_or(page),
            last_page: raw.last.unwrap_or(page),
        })
    }

    pub async fn wear_skin(&self, login: &str, password: &str, skin_id: u64) -> Result<(), String> {
        let client = create_authenticated_web_client(login, password).await?;

        let res = client
            .post("https://ely.by/skins/wear")
            .header("X-Requested-With", "XMLHttpRequest")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(format!("skinId={skin_id}"))
            .send()
            .await
            .map_err(|e| format!("Failed to wear skin request: {e}"))?;

        if !res.status().is_success() {
            return Err(format!(
                "Ely.by responded with HTTP status {}",
                res.status()
            ));
        }

        let json = res
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("Failed to parse response: {e}"))?;

        check_ely_web_response(&json)?;
        Ok(())
    }

    pub async fn upload_skin(
        &self,
        login: &str,
        password: &str,
        png_bytes: Vec<u8>,
    ) -> Result<(), String> {
        let client = create_authenticated_web_client(login, password).await?;

        let part = reqwest::multipart::Part::bytes(png_bytes)
            .file_name("skin.png")
            .mime_str("image/png")
            .map_err(|e| format!("Failed to construct file part: {e}"))?;

        let form = reqwest::multipart::Form::new().part("file", part);

        let res = client
            .post("https://ely.by/skins/upload")
            .header("X-Requested-With", "XMLHttpRequest")
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("Failed to upload skin request: {e}"))?;

        if !res.status().is_success() {
            return Err(format!(
                "Ely.by responded with HTTP status {}",
                res.status()
            ));
        }

        let json = res
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("Failed to parse response: {e}"))?;

        check_ely_web_response(&json)?;

        // If Ely.by returns a skin ID (or redirect URL with skin ID), automatically wear the uploaded skin
        if let Some(skin_id) = extract_skin_id(&json) {
            let wear_res = client
                .post("https://ely.by/skins/wear")
                .header("X-Requested-With", "XMLHttpRequest")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(format!("skinId={skin_id}"))
                .send()
                .await
                .map_err(|e| format!("Failed to wear uploaded skin: {e}"))?;

            if wear_res.status().is_success() {
                if let Ok(wear_json) = wear_res.json::<serde_json::Value>().await {
                    let _ = check_ely_web_response(&wear_json);
                }
            }
        }

        Ok(())
    }
}

#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct ElySkinItem {
    pub id: u64,
    pub skin_url: String,
    pub is_slim: bool,
    pub count_wearers: u64,
    pub count_cubes: u64,
    pub count_views: u64,
    pub tags: Vec<String>,
}

#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct ElySkinsCatalogResponse {
    pub items: Vec<ElySkinItem>,
    pub total_items: u64,
    pub current_page: u32,
    pub last_page: u32,
}

#[derive(Debug, Deserialize)]
struct ElyRawSkinItem {
    id: u64,
    skin_url: String,
    #[serde(default)]
    is_slim: bool,
    #[serde(default)]
    count_wearers: u64,
    #[serde(default)]
    count_cubes: u64,
    #[serde(default)]
    count_views_total: u64,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ElyRawCatalogResponse {
    #[serde(default)]
    items: Vec<ElyRawSkinItem>,
    #[serde(default)]
    total_items: Option<u64>,
    #[serde(default)]
    current: Option<u32>,
    #[serde(default)]
    last: Option<u32>,
}

fn url_encode(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len() * 3);
    for byte in input.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}

fn strip_html_tags(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }
    result.trim().to_string()
}

fn extract_skin_id(json: &serde_json::Value) -> Option<u64> {
    if let Some(id) = json.get("id").and_then(|v| v.as_u64()) {
        return Some(id);
    }
    if let Some(id_str) = json.get("id").and_then(|v| v.as_str()) {
        if let Ok(id) = id_str.parse::<u64>() {
            return Some(id);
        }
    }
    if let Some(id) = json.get("skinId").and_then(|v| v.as_u64()) {
        return Some(id);
    }
    if let Some(url) = json.get("url").and_then(|v| v.as_str()) {
        if let Some(pos) = url.find("/skins/s") {
            let rest = &url[pos + 8..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(id) = digits.parse::<u64>() {
                return Some(id);
            }
        }
        if let Some(pos) = url.find("/skins/") {
            let rest = &url[pos + 7..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(id) = digits.parse::<u64>() {
                return Some(id);
            }
        }
    }
    if let Some(extra) = json.get("extra") {
        if let Some(id) = extra.get("id").and_then(|v| v.as_u64()) {
            return Some(id);
        }
        if let Some(id) = extra.get("skinId").and_then(|v| v.as_u64()) {
            return Some(id);
        }
    }
    None
}

fn check_ely_web_response(json: &serde_json::Value) -> Result<(), String> {
    if let Some(err_val) = json.get("error") {
        if let Some(err_str) = err_val.as_str() {
            if !err_str.is_empty() && !err_str.contains("success") {
                let msg = json
                    .get("text")
                    .and_then(|t| t.as_str())
                    .map(strip_html_tags)
                    .unwrap_or_else(|| err_str.to_string());
                return Err(msg);
            }
        }
    }
    if let Some(success) = json.get("success").and_then(|v| v.as_bool()) {
        if !success {
            let msg = json
                .get("message")
                .or_else(|| json.get("error"))
                .or_else(|| json.get("text"))
                .and_then(|t| t.as_str())
                .map(strip_html_tags)
                .unwrap_or_else(|| json.to_string());
            return Err(msg);
        }
    }
    Ok(())
}

async fn create_authenticated_web_client(
    login: &str,
    password: &str,
) -> Result<reqwest::Client, String> {
    let jar = std::sync::Arc::new(reqwest::cookie::Jar::default());
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .cookie_provider(jar.clone())
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    // Step A: GET https://ely.by/authorization/login to obtain redirect location and initial session cookie
    let login_init_res = client
        .get("https://ely.by/authorization/login")
        .send()
        .await
        .map_err(|e| format!("Failed to initiate Ely.by login: {e}"))?;

    let redirect_url = login_init_res
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| "Ely.by login redirect not found".to_string())?
        .to_string();

    // Step B: Authenticate with account.ely.by
    let auth_payload = serde_json::json!({
        "login": login,
        "password": password,
        "rememberMe": true,
    });

    let auth_res = client
        .post("https://account.ely.by/api/authentication/login")
        .json(&auth_payload)
        .send()
        .await
        .map_err(|e| format!("Failed to authenticate with Ely.by account: {e}"))?;

    let auth_json = auth_res
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse Ely.by login response: {e}"))?;

    let success = auth_json
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !success {
        let err_msg = if let Some(errors) = auth_json.get("errors") {
            if let Some(pw_err) = errors.get("password").and_then(|v| v.as_str()) {
                if pw_err.contains("password_incorrect") {
                    "Incorrect Ely.by password".to_string()
                } else {
                    format!("Authentication failed: {pw_err}")
                }
            } else if let Some(login_err) = errors.get("login").and_then(|v| v.as_str()) {
                format!("Account error: {login_err}")
            } else if let Some(_totp_err) = errors.get("totp").and_then(|v| v.as_str()) {
                "Ely.by Two-Factor Authentication (TOTP) is required on this account".to_string()
            } else {
                format!("Login failed: {errors}")
            }
        } else {
            "Ely.by authentication failed".to_string()
        };
        return Err(err_msg);
    }

    let web_access_token = auth_json
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing access_token in Ely.by response".to_string())?;

    // Step C: Complete OAuth flow on account.ely.by
    let complete_url = build_oauth_complete_url(&redirect_url);

    // Post with accept: true (form urlencoded as required by Ely.by OAuth completion)
    let complete_res = client
        .post(&complete_url)
        .header("Authorization", format!("Bearer {web_access_token}"))
        .form(&[("accept", "true")])
        .send()
        .await
        .map_err(|e| format!("Failed to complete Ely.by OAuth: {e}"))?;

    let status = complete_res.status();
    let complete_json = complete_res
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse OAuth complete response: {e}"))?;

    if !status.is_success() {
        let err_detail = complete_json
            .get("message")
            .or_else(|| complete_json.get("error_description"))
            .or_else(|| complete_json.get("error"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| complete_json.to_string());
        return Err(format!("Ely.by OAuth completion error ({status}): {err_detail}"));
    }

    let redirect_uri = complete_json
        .get("redirectUri")
        .or_else(|| complete_json.get("redirect_uri"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("Missing redirectUri from Ely.by OAuth response: {complete_json}"))?;

    // Step D: GET redirect_uri on ely.by to finalize session cookies
    let finalize_res = client
        .get(redirect_uri)
        .send()
        .await
        .map_err(|e| format!("Failed to finalize Ely.by session: {e}"))?;

    if !finalize_res.status().is_redirection() && !finalize_res.status().is_success() {
        return Err(format!(
            "Ely.by OAuth finalization failed with status {}",
            finalize_res.status()
        ));
    }

    // Follow redirect if Ely.by responded with 302 Found to complete session initialization
    if finalize_res.status().is_redirection() {
        if let Some(loc) = finalize_res
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
        {
            let next_url = if loc.starts_with('/') {
                format!("https://ely.by{loc}")
            } else {
                loc.to_string()
            };
            let _ = client.get(&next_url).send().await;
        }
    }

    Ok(client)
}

fn build_oauth_complete_url(redirect_url: &str) -> String {
    let (path_part, query_part) = redirect_url
        .split_once('?')
        .unwrap_or((redirect_url, ""));

    // Extract client_id from path if present (e.g. "/oauth2/v1/ely" -> "ely")
    let client_id_from_path = path_part
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty() && *s != "oauth2" && !s.starts_with('v'));

    let mut has_client_id = false;
    let mut params: Vec<(String, String)> = Vec::new();

    for pair in query_part.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if k == "client_id" {
            has_client_id = true;
        }
        if k == "scope" {
            // Replace comma with space in scope (RFC 6749 & Ely.by standard)
            let scope_val = v.replace("%2C", " ").replace(',', " ");
            params.push((k.to_string(), scope_val));
        } else {
            params.push((k.to_string(), v.to_string()));
        }
    }

    if !has_client_id {
        let cid = client_id_from_path.unwrap_or("ely");
        params.insert(0, ("client_id".to_string(), cid.to_string()));
    }

    let query_string = params
        .iter()
        .map(|(k, v)| {
            if k == "scope" {
                format!("{k}={}", v.replace(' ', "%20"))
            } else {
                format!("{k}={v}")
            }
        })
        .collect::<Vec<_>>()
        .join("&");

    format!("https://account.ely.by/api/oauth2/v1/complete?{query_string}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_oauth_complete_url() {
        let url = "https://account.ely.by/oauth2/v1/ely?scope=account_info%2Caccount_email&state=826fcd58c1a95a8f7418ac1211990821&response_type=code&redirect_uri=https%3A%2F%2Fely.by%2Fauthorization%2Foauth";
        let complete_url = build_oauth_complete_url(url);
        assert!(complete_url.starts_with("https://account.ely.by/api/oauth2/v1/complete?"));
        assert!(complete_url.contains("client_id=ely"));
        assert!(complete_url.contains("scope=account_info%20account_email"));
        assert!(complete_url.contains("state=826fcd58c1a95a8f7418ac1211990821"));
        assert!(complete_url.contains("redirect_uri=https%3A%2F%2Fely.by%2Fauthorization%2Foauth"));
    }

    #[tokio::test]
    async fn test_fetch_catalog_with_uploader() {
        let service = ElyAuthService::new();
        let res = service
            .fetch_catalog(1, None, Some("latest".into()), None, Some("ErickSkrauch".into()))
            .await;
        assert!(res.is_ok(), "Expected fetch_catalog with uploader to succeed, got: {:?}", res.err());
    }

    #[test]
    fn test_extract_skin_id() {
        let json1 = serde_json::json!({
            "error": "success",
            "url": "/skins/s98765/edit",
            "text": "Success"
        });
        assert_eq!(extract_skin_id(&json1), Some(98765));

        let json2 = serde_json::json!({
            "id": 123456,
            "success": true
        });
        assert_eq!(extract_skin_id(&json2), Some(123456));

        let json3 = serde_json::json!({
            "url": "https://ely.by/skins/s45678",
        });
        assert_eq!(extract_skin_id(&json3), Some(45678));
    }
}
