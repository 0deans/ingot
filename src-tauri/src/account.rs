use crate::auth::ely::{ElyAuthService, ElySkinsCatalogResponse};
use crate::keyring_store;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;

#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccountProfile {
    pub id: String,
    pub account_type: String, // "ely", "microsoft", "offline"
    pub username: String,
    pub uuid: String,
    pub skin_url: Option<String>,
    pub is_active: bool,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSecrets {
    pub access_token: String,
    pub client_token: String,
    #[serde(default)]
    pub password: Option<String>,
}

fn get_storage_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {e}"))?;

    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create app data directory: {e}"))?;
    }

    Ok(data_dir.join("accounts.json"))
}

pub(crate) fn load_accounts_file<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<AccountProfile>, String> {
    let file_path = get_storage_path(app)?;
    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let data =
        fs::read_to_string(&file_path).map_err(|e| format!("Failed to read accounts file: {e}"))?;

    let accounts: Vec<AccountProfile> = serde_json::from_str(&data).unwrap_or_default();
    Ok(accounts)
}

fn save_accounts_file<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    accounts: &[AccountProfile],
) -> Result<(), String> {
    let file_path = get_storage_path(app)?;
    let data = serde_json::to_string_pretty(accounts)
        .map_err(|e| format!("Failed to serialize accounts: {e}"))?;

    fs::write(&file_path, data).map_err(|e| format!("Failed to write accounts file: {e}"))?;

    Ok(())
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub async fn ely_login<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    username: String,
    password: String,
) -> Result<AccountProfile, String> {
    let username_trimmed = username.trim();
    if username_trimmed.is_empty() || password.is_empty() {
        return Err("Username and password are required".to_string());
    }

    let client_token = uuid::Uuid::new_v4().to_string();
    let auth_service = ElyAuthService::new();

    let auth_resp = auth_service
        .authenticate(username_trimmed, &password, &client_token)
        .await?;

    let profile = auth_resp
        .selected_profile
        .ok_or_else(|| "No Minecraft profile found for this Ely.by account".to_string())?;

    let account_id = format!("ely:{}", profile.id);
    let skin_url = format!("https://skinsystem.ely.by/skins/{}.png", profile.name);

    // 1. Securely store the sensitive tokens and password in the native OS Credential Vault
    let secrets = AccountSecrets {
        access_token: auth_resp.access_token,
        client_token: auth_resp.client_token,
        password: Some(password),
    };
    let secrets_json = serde_json::to_string(&secrets)
        .map_err(|e| format!("Failed to serialize credentials: {e}"))?;

    keyring_store::save_secret(&account_id, &secrets_json)?;

    // 2. Save non-sensitive metadata in accounts.json
    let mut accounts = load_accounts_file(&app)?;

    // Deactivate all existing accounts
    for acc in &mut accounts {
        acc.is_active = false;
    }

    let new_profile = AccountProfile {
        id: account_id.clone(),
        account_type: "ely".to_string(),
        username: profile.name,
        uuid: profile.id,
        skin_url: Some(skin_url),
        is_active: true,
        created_at: current_timestamp(),
    };

    if let Some(pos) = accounts.iter().position(|a| a.id == account_id) {
        accounts[pos] = new_profile.clone();
    } else {
        accounts.push(new_profile.clone());
    }

    save_accounts_file(&app, &accounts)?;

    Ok(new_profile)
}

pub fn add_offline_account<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    username: String,
) -> Result<AccountProfile, String> {
    let clean_name = username.trim();
    if clean_name.is_empty() {
        return Err("Player nickname cannot be empty".to_string());
    }

    // Minecraft offline UUID: MD5 nameUUIDFromBytes("OfflinePlayer:" + name)
    let offline_uuid = uuid::Uuid::new_v3(
        &uuid::Uuid::NAMESPACE_DNS,
        format!("OfflinePlayer:{}", clean_name).as_bytes(),
    )
    .as_simple()
    .to_string();

    let account_id = format!("offline:{}", clean_name.to_lowercase());
    let mut accounts = load_accounts_file(&app)?;

    for acc in &mut accounts {
        acc.is_active = false;
    }

    let new_profile = AccountProfile {
        id: account_id.clone(),
        account_type: "offline".to_string(),
        username: clean_name.to_string(),
        uuid: offline_uuid,
        skin_url: None,
        is_active: true,
        created_at: current_timestamp(),
    };

    if let Some(pos) = accounts.iter().position(|a| a.id == account_id) {
        accounts[pos] = new_profile.clone();
    } else {
        accounts.push(new_profile.clone());
    }

    save_accounts_file(&app, &accounts)?;

    Ok(new_profile)
}

pub fn get_accounts<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Vec<AccountProfile>, String> {
    load_accounts_file(&app)
}

pub fn set_active_account<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    account_id: String,
) -> Result<(), String> {
    let mut accounts = load_accounts_file(&app)?;
    let mut found = false;

    for acc in &mut accounts {
        if acc.id == account_id {
            acc.is_active = true;
            found = true;
        } else {
            acc.is_active = false;
        }
    }

    if !found {
        return Err("Account not found".to_string());
    }

    save_accounts_file(&app, &accounts)?;
    Ok(())
}

pub fn remove_account<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    account_id: String,
) -> Result<(), String> {
    let mut accounts = load_accounts_file(&app)?;
    let pos = accounts
        .iter()
        .position(|a| a.id == account_id)
        .ok_or_else(|| "Account not found".to_string())?;

    let removed = accounts.remove(pos);

    // Delete credentials from OS Keyring if applicable
    let _ = keyring_store::delete_secret(&account_id);

    // If removed account was active, set first remaining as active
    if removed.is_active && !accounts.is_empty() {
        accounts[0].is_active = true;
    }

    save_accounts_file(&app, &accounts)?;
    Ok(())
}

pub async fn get_active_account_token<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<String, String> {
    let accounts = load_accounts_file(&app)?;
    let active = accounts
        .iter()
        .find(|a| a.is_active)
        .ok_or_else(|| "No active account selected".to_string())?;

    if active.account_type == "offline" {
        return Ok("offline".to_string());
    }

    let secret_opt = keyring_store::get_secret(&active.id)?;
    let secret_str = secret_opt.ok_or_else(|| "No credentials found in OS Keyring".to_string())?;

    let secrets: AccountSecrets = serde_json::from_str(&secret_str)
        .map_err(|e| format!("Failed to parse stored credentials: {e}"))?;

    let auth_service = ElyAuthService::new();

    // Check if token is still valid
    if auth_service
        .validate(&secrets.access_token, &secrets.client_token)
        .await
    {
        return Ok(secrets.access_token);
    }

    // Attempt token refresh
    let refresh_res = auth_service
        .refresh(&secrets.access_token, &secrets.client_token)
        .await?;

    // Update refreshed token in OS keyring
    let updated_secrets = AccountSecrets {
        access_token: refresh_res.access_token.clone(),
        client_token: refresh_res.client_token,
        password: secrets.password,
    };
    let updated_json = serde_json::to_string(&updated_secrets)
        .map_err(|e| format!("Failed to serialize refreshed credentials: {e}"))?;

    keyring_store::save_secret(&active.id, &updated_json)?;

    Ok(refresh_res.access_token)
}

pub async fn get_skin_data_url<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    skin_url: String,
) -> Result<String, String> {
    use base64::Engine;

    if skin_url.trim().is_empty() {
        return Err("Skin URL is empty".to_string());
    }

    let url_hash = uuid::Uuid::new_v3(&uuid::Uuid::NAMESPACE_URL, skin_url.as_bytes());
    let filename = format!("{url_hash:x}.png");
    let cache_dir = app
        .path()
        .app_cache_dir()
        .or_else(|_| app.path().app_data_dir())
        .map_err(|e| format!("Failed to get cache dir: {e}"))?
        .join("skins");

    if !cache_dir.exists() {
        let _ = fs::create_dir_all(&cache_dir);
    }

    let file_path = cache_dir.join(&filename);

    if file_path.exists() {
        if let Ok(bytes) = fs::read(&file_path) {
            let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
            return Ok(format!("data:image/png;base64,{encoded}"));
        }
    }

    let client = reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to build client: {e}"))?;

    let resp = client
        .get(&skin_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch skin: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Server returned HTTP {}", resp.status()));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read skin bytes: {e}"))?;

    let _ = fs::write(&file_path, &bytes);

    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:image/png;base64,{encoded}"))
}

pub fn reorder_accounts<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    account_ids: Vec<String>,
) -> Result<(), String> {
    let accounts = load_accounts_file(&app)?;
    let mut reordered: Vec<AccountProfile> = Vec::new();

    for id in &account_ids {
        if let Some(acc) = accounts.iter().find(|a| &a.id == id) {
            reordered.push(acc.clone());
        }
    }

    for acc in &accounts {
        if !reordered.iter().any(|a| a.id == acc.id) {
            reordered.push(acc.clone());
        }
    }

    save_accounts_file(&app, &reordered)?;
    Ok(())
}

pub async fn save_skin_to_downloads<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    username: String,
    skin_url: String,
) -> Result<String, String> {
    use base64::Engine;

    if skin_url.trim().is_empty() {
        return Err("Skin URL is empty".to_string());
    }

    let bytes: Vec<u8> = if skin_url.starts_with("data:") {
        let comma_pos = skin_url
            .find(',')
            .ok_or_else(|| "Invalid data URL".to_string())?;
        let base64_data = &skin_url[comma_pos + 1..];
        base64::engine::general_purpose::STANDARD
            .decode(base64_data.trim())
            .map_err(|e| format!("Failed to decode base64 skin data: {e}"))?
    } else {
        let url_hash = uuid::Uuid::new_v3(&uuid::Uuid::NAMESPACE_URL, skin_url.as_bytes());
        let cache_filename = format!("{url_hash:x}.png");
        let cache_dir = app
            .path()
            .app_cache_dir()
            .or_else(|_| app.path().app_data_dir())
            .map_err(|e| format!("Failed to get cache dir: {e}"))?
            .join("skins");

        let cached_path = cache_dir.join(&cache_filename);
        if cached_path.exists() {
            fs::read(&cached_path).map_err(|e| format!("Failed to read cached skin: {e}"))?
        } else {
            let client = reqwest::Client::builder()
                .user_agent(crate::USER_AGENT)
                .build()
                .map_err(|e| format!("Failed to build client: {e}"))?;

            let resp = client
                .get(&skin_url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch skin: {e}"))?;

            if !resp.status().is_success() {
                return Err(format!("Server returned HTTP {}", resp.status()));
            }

            let b = resp
                .bytes()
                .await
                .map_err(|e| format!("Failed to read skin bytes: {e}"))?;

            let _ = fs::create_dir_all(&cache_dir);
            let _ = fs::write(&cached_path, &b);
            b.to_vec()
        }
    };

    let download_dir = app
        .path()
        .download_dir()
        .or_else(|_| app.path().app_data_dir())
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    let clean_user = username
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    let base_name = if clean_user.is_empty() {
        "minecraft-skin".to_string()
    } else {
        format!("{clean_user}-skin")
    };
    let default_file_name = format!("{base_name}.png");

    use tauri_plugin_dialog::DialogExt;
    let file_path = app
        .dialog()
        .file()
        .set_title("Save Skin As")
        .set_directory(&download_dir)
        .set_file_name(&default_file_name)
        .add_filter("PNG Image (*.png)", &["png"])
        .blocking_save_file();

    let target_path = match file_path {
        Some(path) => match path.into_path() {
            Ok(p) => p,
            Err(_) => return Err("Invalid destination file path".to_string()),
        },
        None => return Ok(String::new()), // User cancelled dialog
    };

    fs::write(&target_path, &bytes).map_err(|e| format!("Failed to write skin file: {e}"))?;

    Ok(target_path.to_string_lossy().to_string())
}

pub async fn get_ely_skins_catalog(
    page: u32,
    query: Option<String>,
    sort: Option<String>,
    model: Option<String>,
    uploader: Option<String>,
) -> Result<ElySkinsCatalogResponse, String> {
    let service = ElyAuthService::new();
    service.fetch_catalog(page, query, sort, model, uploader).await
}

pub fn has_ely_web_credentials(account_id: &str) -> bool {
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

pub async fn apply_ely_skin<R: tauri::Runtime>(
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

pub async fn upload_ely_skin<R: tauri::Runtime>(
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
