//! Native provider skin operations and account metadata updates.
use super::profiles::save_skin_url;
use super::session::microsoft_token_for_locked;
use super::storage::{current_timestamp, load_accounts_file, with_recovered};
use crate::account::credentials::{self, CredentialError};
use crate::auth::ely::{ElyAuthService, ElySkinsCatalogResponse};
use crate::auth::microsoft;
use base64::Engine;
pub(crate) async fn apply_microsoft_skin<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account_id: &str,
    skin_url: &str,
    is_slim: bool,
) -> Result<(), String> {
    let _operation = crate::app::state(app)
        .account_operations
        .acquire(account_id)
        .await
        .map_err(|error| error.to_string())?;
    let token = microsoft_token_for_locked(app, &_operation).await?;
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

    let _operation = crate::app::state(app)
        .account_operations
        .acquire(account_id)
        .await
        .map_err(|error| error.to_string())?;
    let token = microsoft_token_for_locked(app, &_operation).await?;
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

pub(crate) fn has_ely_web_credentials<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account_id: &str,
) -> Result<bool, String> {
    with_recovered(app, |_| Ok(()))?;
    match credentials::load(crate::app::state(app).credentials.as_ref(), account_id) {
        Ok(secrets) => Ok(secrets
            .password
            .as_deref()
            .is_some_and(|password| !password.trim().is_empty())),
        Err(CredentialError::Missing) => Ok(false),
        Err(error) => Err(error.to_string()),
    }
}
pub(crate) async fn apply_ely_skin<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account_id: &str,
    skin_id: u64,
    password: Option<String>,
) -> Result<(), String> {
    let state = crate::app::state(app);
    let _operation = state
        .account_operations
        .acquire(account_id)
        .await
        .map_err(|error| error.to_string())?;
    let accounts = load_accounts_file(app)?;
    let acc = accounts
        .iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| "Account not found".to_string())?;

    if acc.account_type != "ely" {
        return Err("Skin changes can only be applied to Ely.by accounts".to_string());
    }

    let username = acc.username.clone();

    // Determine password
    let secrets = match credentials::load(state.credentials.as_ref(), account_id) {
        Ok(secrets) => Some(secrets),
        Err(CredentialError::Missing) => None,
        Err(error) => return Err(error.to_string()),
    };

    let effective_password = match password {
        Some(p) if !p.trim().is_empty() => {
            let p_trimmed = p.trim().to_string();
            // Update saved password in keyring
            if let Some(mut s) = secrets {
                s.password = Some(p_trimmed.clone());
                credentials::save(state.credentials.as_ref(), &_operation, &s)
                    .map_err(|error| error.to_string())?;
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
    // The provider already committed the skin. Report metadata-cache failure diagnostically.
    if let Err(error) = with_recovered(app, |repository| {
        repository.update_profile(account_id, |stored| {
            stored.skin_url = Some(updated_skin_url);
        })
    }) {
        eprintln!("Skin changed, but account metadata could not be saved: {error}");
    }

    Ok(())
}

pub(crate) async fn upload_ely_skin<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account_id: &str,
    image_base64: &str,
    password: Option<String>,
) -> Result<(), String> {
    let state = crate::app::state(app);
    let _operation = state
        .account_operations
        .acquire(account_id)
        .await
        .map_err(|error| error.to_string())?;
    let accounts = load_accounts_file(app)?;
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
    let secrets = match credentials::load(state.credentials.as_ref(), account_id) {
        Ok(secrets) => Some(secrets),
        Err(CredentialError::Missing) => None,
        Err(error) => return Err(error.to_string()),
    };

    let effective_password = match password {
        Some(p) if !p.trim().is_empty() => {
            let p_trimmed = p.trim().to_string();
            // Update saved password in keyring
            if let Some(mut s) = secrets {
                s.password = Some(p_trimmed.clone());
                credentials::save(state.credentials.as_ref(), &_operation, &s)
                    .map_err(|error| error.to_string())?;
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
    // The provider already committed the skin. Report metadata-cache failure diagnostically.
    if let Err(error) = with_recovered(app, |repository| {
        repository.update_profile(account_id, |stored| {
            stored.skin_url = Some(updated_skin_url);
        })
    }) {
        eprintln!("Skin changed, but account metadata could not be saved: {error}");
    }

    Ok(())
}
