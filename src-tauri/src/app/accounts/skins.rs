//! Native provider skin operations and account metadata updates.
use super::profiles::save_skin_url;
use super::session::microsoft_token_for;
use super::storage::{current_timestamp, load_accounts_file, save_accounts_file};
use crate::account::AccountSecrets;
use crate::auth::ely::{ElyAuthService, ElySkinsCatalogResponse};
use crate::auth::microsoft;
use crate::keyring_store;
use base64::Engine;
pub(crate) async fn apply_microsoft_skin<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account_id: &str,
    skin_url: &str,
    is_slim: bool,
) -> Result<(), String> {
    let token = microsoft_token_for(app, account_id).await?;
    let new_url = microsoft::set_skin_from_url(&token, skin_url, is_slim).await?;
    save_skin_url(app, account_id, new_url)
}

/// Uploads a PNG (base64 or data URL) as a Microsoft account's skin
pub(crate) async fn upload_microsoft_skin<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account_id: &str,
    image_base64: &str,
    is_slim: bool,
) -> Result<(), String> {
    let base64_data = image_base64
        .split_once(',')
        .map_or(image_base64, |(_, data)| data);
    let png = base64::engine::general_purpose::STANDARD
        .decode(base64_data.trim())
        .map_err(|e| format!("Invalid base64 skin image data: {e}"))?;

    let token = microsoft_token_for(app, account_id).await?;
    let new_url = microsoft::upload_skin(&token, png, is_slim).await?;
    save_skin_url(app, account_id, new_url)
}

pub(crate) async fn get_ely_skins_catalog(
    page: u32,
    query: Option<String>,
    sort: Option<String>,
    model: Option<String>,
    uploader: Option<String>,
) -> Result<ElySkinsCatalogResponse, String> {
    let service = ElyAuthService::new();
    service
        .fetch_catalog(page, query, sort, model, uploader)
        .await
}

pub(crate) fn has_ely_web_credentials(account_id: &str) -> bool {
    if let Ok(Some(secret_str)) = keyring_store::get_secret(account_id) {
        if let Ok(secrets) = serde_json::from_str::<AccountSecrets>(&secret_str) {
            return secrets
                .password
                .as_deref()
                .map(|p| !p.trim().is_empty())
                .unwrap_or(false);
        }
    }
    false
}

pub(crate) async fn apply_ely_skin<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account_id: &str,
    skin_id: u64,
    password: Option<String>,
) -> Result<(), String> {
    let mut accounts = load_accounts_file(app)?;
    let acc = accounts
        .iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| "Account not found".to_string())?;

    if acc.account_type != "ely" {
        return Err("Skin changes can only be applied to Ely.by accounts".to_string());
    }

    let username = acc.username.clone();

    // Determine password
    let mut secrets: Option<AccountSecrets> = None;
    if let Ok(Some(secret_str)) = keyring_store::get_secret(account_id) {
        secrets = serde_json::from_str::<AccountSecrets>(&secret_str).ok();
    }

    let effective_password = match password {
        Some(p) if !p.trim().is_empty() => {
            let p_trimmed = p.trim().to_string();
            // Update saved password in keyring
            if let Some(mut s) = secrets {
                s.password = Some(p_trimmed.clone());
                if let Ok(serialized) = serde_json::to_string(&s) {
                    let _ = keyring_store::save_secret(account_id, &serialized);
                }
            }
            p_trimmed
        }
        _ => secrets
            .and_then(|s| s.password)
            .filter(|p| !p.trim().is_empty())
            .ok_or_else(|| "PASSWORD_REQUIRED".to_string())?,
    };

    let service = ElyAuthService::new();
    service
        .wear_skin(&username, &effective_password, skin_id)
        .await?;

    // Update account skin_url with timestamp to invalidate local cache
    let updated_skin_url = format!(
        "https://skinsystem.ely.by/skins/{username}.png?t={}",
        current_timestamp()
    );
    if let Some(pos) = accounts.iter().position(|a| a.id == account_id) {
        accounts[pos].skin_url = Some(updated_skin_url);
        let _ = save_accounts_file(app, &accounts);
    }

    Ok(())
}

pub(crate) async fn upload_ely_skin<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account_id: &str,
    image_base64: &str,
    password: Option<String>,
) -> Result<(), String> {
    let mut accounts = load_accounts_file(app)?;
    let acc = accounts
        .iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| "Account not found".to_string())?;

    if acc.account_type != "ely" {
        return Err("Skin uploads can only be applied to Ely.by accounts".to_string());
    }

    let username = acc.username.clone();

    // Clean base64 data
    let clean_base64 = if let Some(comma_pos) = image_base64.find(',') {
        &image_base64[comma_pos + 1..]
    } else {
        image_base64
    };

    let png_bytes = base64::engine::general_purpose::STANDARD
        .decode(clean_base64.trim())
        .map_err(|e| format!("Invalid base64 skin image data: {e}"))?;

    // Determine password
    let mut secrets: Option<AccountSecrets> = None;
    if let Ok(Some(secret_str)) = keyring_store::get_secret(account_id) {
        secrets = serde_json::from_str::<AccountSecrets>(&secret_str).ok();
    }

    let effective_password = match password {
        Some(p) if !p.trim().is_empty() => {
            let p_trimmed = p.trim().to_string();
            // Update saved password in keyring
            if let Some(mut s) = secrets {
                s.password = Some(p_trimmed.clone());
                if let Ok(serialized) = serde_json::to_string(&s) {
                    let _ = keyring_store::save_secret(account_id, &serialized);
                }
            }
            p_trimmed
        }
        _ => secrets
            .and_then(|s| s.password)
            .filter(|p| !p.trim().is_empty())
            .ok_or_else(|| "PASSWORD_REQUIRED".to_string())?,
    };

    let service = ElyAuthService::new();
    service
        .upload_skin(&username, &effective_password, png_bytes)
        .await?;

    // Update account skin_url with timestamp to invalidate local cache
    let updated_skin_url = format!(
        "https://skinsystem.ely.by/skins/{username}.png?t={}",
        current_timestamp()
    );
    if let Some(pos) = accounts.iter().position(|a| a.id == account_id) {
        accounts[pos].skin_url = Some(updated_skin_url);
        let _ = save_accounts_file(app, &accounts);
    }

    Ok(())
}
