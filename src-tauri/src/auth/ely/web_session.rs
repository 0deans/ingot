//! Ely.by authenticated web session and OAuth handshake.
use super::error::ElyError;
use reqwest::{header, redirect::Policy, Client};
use std::sync::Arc;

pub(crate) fn build_oauth_complete_url(account_base: &str, redirect_url: &str) -> String {
    let (path_part, query_part) = redirect_url.split_once('?').unwrap_or((redirect_url, ""));

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

    format!("{account_base}/api/oauth2/v1/complete?{query_string}")
}

pub(crate) async fn create_authenticated_web_client(
    login: &str,
    password: &str,
    ely_base: &str,
    account_base: &str,
) -> Result<Client, ElyError> {
    let jar = Arc::new(reqwest::cookie::Jar::default());
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .cookie_provider(jar.clone())
        .redirect(Policy::none())
        .build()
        .map_err(ElyError::ClientInit)?;

    // Step A: GET authorization/login to obtain redirect location and initial session cookie
    let login_init_res = client
        .get(format!("{ely_base}/authorization/login"))
        .send()
        .await
        .map_err(ElyError::Transport)?;

    let redirect_url = login_init_res
        .headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(ElyError::MissingRedirect)?
        .to_string();

    // Step B: Authenticate with account service
    let auth_payload = serde_json::json!({
        "login": login,
        "password": password,
        "rememberMe": true,
    });

    let auth_res = client
        .post(format!("{account_base}/api/authentication/login"))
        .json(&auth_payload)
        .send()
        .await
        .map_err(ElyError::Transport)?;

    let auth_json = auth_res
        .json::<serde_json::Value>()
        .await
        .map_err(ElyError::ParseResponse)?;

    let success = auth_json
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !success {
        if let Some(errors) = auth_json.get("errors") {
            if let Some(pw_err) = errors.get("password").and_then(|v| v.as_str()) {
                if pw_err.contains("password_incorrect") {
                    return Err(ElyError::IncorrectPassword);
                }
                return Err(ElyError::Account(format!(
                    "Authentication failed: {pw_err}"
                )));
            }
            if let Some(login_err) = errors.get("login").and_then(|v| v.as_str()) {
                return Err(ElyError::Account(login_err.to_string()));
            }
            if errors.get("totp").and_then(|v| v.as_str()).is_some() {
                return Err(ElyError::TwoFactorRequired);
            }
            return Err(ElyError::Account(errors.to_string()));
        }
        return Err(ElyError::Account(
            "Ely.by authentication failed".to_string(),
        ));
    }

    let web_access_token = auth_json
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or(ElyError::MissingAccessToken)?;

    // Step C: Complete OAuth flow
    let complete_url = build_oauth_complete_url(account_base, &redirect_url);
    let complete_res = client
        .post(&complete_url)
        .header("Authorization", format!("Bearer {web_access_token}"))
        .form(&[("accept", "true")])
        .send()
        .await
        .map_err(ElyError::Transport)?;

    let status = complete_res.status();
    let complete_json = complete_res
        .json::<serde_json::Value>()
        .await
        .map_err(ElyError::ParseResponse)?;

    if !status.is_success() {
        let err_detail = complete_json
            .get("message")
            .or_else(|| complete_json.get("error_description"))
            .or_else(|| complete_json.get("error"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| complete_json.to_string());
        return Err(ElyError::OAuthCompletion {
            status,
            message: err_detail,
        });
    }

    let redirect_uri = complete_json
        .get("redirectUri")
        .or_else(|| complete_json.get("redirect_uri"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| ElyError::MissingRedirectUri(complete_json.to_string()))?;

    // Step D: GET redirect_uri to finalize session cookies
    let finalize_url = if redirect_uri.starts_with('/') {
        format!("{ely_base}{redirect_uri}")
    } else {
        redirect_uri.to_string()
    };
    let finalize_res = client
        .get(&finalize_url)
        .send()
        .await
        .map_err(ElyError::Transport)?;

    if !finalize_res.status().is_redirection() && !finalize_res.status().is_success() {
        return Err(ElyError::HttpStatus(finalize_res.status()));
    }

    // Follow redirect if 302 Found
    if finalize_res.status().is_redirection() {
        if let Some(loc) = finalize_res
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
        {
            let next_url = if loc.starts_with('/') {
                format!("{ely_base}{loc}")
            } else {
                loc.to_string()
            };
            let _ = client.get(&next_url).send().await;
        }
    }

    Ok(client)
}
