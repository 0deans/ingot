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
}
