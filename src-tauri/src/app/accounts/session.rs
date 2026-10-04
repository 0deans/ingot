//! Native login/token orchestration over provider and credential adapters.
use super::storage::{current_timestamp, load_accounts_file, repository};
use crate::account::{AccountProfile, AccountSecrets};
use crate::auth::ely::ElyAuthService;
use crate::auth::microsoft::{self, MicrosoftDeviceCode, MinecraftSession};
use crate::keyring_store;
pub(crate) async fn ely_login<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    username: String,
    password: String,
) -> Result<AccountProfile, String> {
    let username_trimmed = username.trim();
    if username_trimmed.is_empty() || password.is_empty() {
        return Err("Username and password are required".to_string());
    }

    repository(&app).load().map_err(|error| error.to_string())?;
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
        ..Default::default()
    };
    let secrets_json = serde_json::to_string(&secrets)
        .map_err(|e| format!("Failed to serialize credentials: {e}"))?;

    keyring_store::save_secret(&account_id, &secrets_json)?;

    // 2. Save non-sensitive metadata in accounts.json
    let new_profile = AccountProfile {
        id: account_id.clone(),
        account_type: "ely".to_string(),
        username: profile.name,
        uuid: profile.id,
        skin_url: Some(skin_url),
        is_active: true,
        created_at: current_timestamp(),
    };

    repository(&app)
        .activate(new_profile)
        .map_err(|error| error.to_string())
}

pub(crate) async fn microsoft_login_start<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<MicrosoftDeviceCode, String> {
    crate::app::state(&app).account_login.invalidate();
    microsoft::start_device_login().await
}

pub(crate) fn microsoft_login_cancel<R: tauri::Runtime>(app: tauri::AppHandle<R>) {
    crate::app::state(&app).account_login.invalidate();
}

/// Waits for the user to enter the code, then saves and activates the account
pub(crate) async fn microsoft_login_finish<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    code: MicrosoftDeviceCode,
) -> Result<AccountProfile, String> {
    repository(&app).load().map_err(|error| error.to_string())?;
    let login = &crate::app::state(&app).account_login;
    let attempt = login.current();
    let is_cancelled = || login.is_cancelled(attempt);

    let session = microsoft::finish_device_login(&code, is_cancelled).await?;
    let account_id = format!("microsoft:{}", session.uuid);
    save_microsoft_secrets(&account_id, &session)?;

    let new_profile = AccountProfile {
        id: account_id.clone(),
        account_type: "microsoft".to_string(),
        username: session.username,
        uuid: session.uuid,
        skin_url: session.skin_url,
        is_active: true,
        created_at: current_timestamp(),
    };

    repository(&app)
        .activate(new_profile)
        .map_err(|error| error.to_string())
}

fn save_microsoft_secrets(account_id: &str, session: &MinecraftSession) -> Result<(), String> {
    let secrets = AccountSecrets {
        access_token: session.access_token.clone(),
        refresh_token: Some(session.refresh_token.clone()),
        expires_at: Some(session.expires_at),
        ..Default::default()
    };
    let json = serde_json::to_string(&secrets)
        .map_err(|e| format!("Failed to serialize credentials: {e}"))?;
    keyring_store::save_secret(account_id, &json)
}

/// A valid Minecraft token for a Microsoft account, refreshed when close to expiry.
/// Also picks up name and skin changes made on minecraft.net.
async fn microsoft_access_token<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account: &AccountProfile,
    secrets: AccountSecrets,
) -> Result<String, String> {
    let still_valid =
        crate::account::policy::microsoft_token_is_fresh(secrets.expires_at, current_timestamp());
    if still_valid {
        return Ok(secrets.access_token);
    }

    let refresh_token = secrets
        .refresh_token
        .ok_or("Your Microsoft session expired, please sign in again")?;
    let session = microsoft::refresh(&refresh_token).await?;
    save_microsoft_secrets(&account.id, &session)?;

    repository(app)
        .update_profile(&account.id, |stored| {
            stored.username = session.username.clone();
            stored.skin_url = session.skin_url.clone();
        })
        .map_err(|error| error.to_string())?;
    Ok(session.access_token)
}

/// A valid Minecraft token for the given Microsoft account (not necessarily the active one)
pub(super) async fn microsoft_token_for<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account_id: &str,
) -> Result<String, String> {
    let account = load_accounts_file(app)?
        .into_iter()
        .find(|a| a.id == account_id)
        .ok_or("Account not found")?;
    if account.account_type != "microsoft" {
        return Err("This isn't a Microsoft account".to_string());
    }

    let secret_str = keyring_store::get_secret(account_id)?
        .ok_or("Your Microsoft session expired, please sign in again")?;
    let secrets: AccountSecrets = serde_json::from_str(&secret_str)
        .map_err(|e| format!("Failed to parse stored credentials: {e}"))?;

    microsoft_access_token(app, &account, secrets).await
}

/// Return the active account token, refreshing provider credentials when necessary.
pub(crate) async fn get_active_account_token<R: tauri::Runtime>(
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

    if active.account_type == "microsoft" {
        return microsoft_access_token(&app, active, secrets).await;
    }

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
        ..Default::default()
    };
    let updated_json = serde_json::to_string(&updated_secrets)
        .map_err(|e| format!("Failed to serialize refreshed credentials: {e}"))?;

    keyring_store::save_secret(&active.id, &updated_json)?;

    Ok(refresh_res.access_token)
}
