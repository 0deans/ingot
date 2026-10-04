//! Native login/token orchestration over provider and credential adapters.
use super::storage::{current_timestamp, load_accounts_file, repository};
use crate::account::credentials;
use crate::account::{AccountProfile, AccountSecrets};
use crate::auth::ely::ElyAuthService;
use crate::auth::microsoft::{self, MicrosoftDeviceCode, MinecraftSession};

#[cfg(test)]
mod tests;
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

    let secrets = AccountSecrets {
        access_token: auth_resp.access_token,
        client_token: auth_resp.client_token,
        password: Some(password),
        ..Default::default()
    };

    let new_profile = AccountProfile {
        id: account_id.clone(),
        account_type: "ely".to_string(),
        username: profile.name,
        uuid: profile.id,
        skin_url: Some(skin_url),
        is_active: true,
        created_at: current_timestamp(),
    };

    let state = crate::app::state(&app);
    let _operation = state
        .account_operations
        .acquire(&account_id)
        .await
        .map_err(|error| error.to_string())?;
    credentials::publish(state.credentials.as_ref(), &_operation, &secrets, || {
        repository(&app).activate(new_profile)
    })
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
    let secrets = microsoft_secrets(&session);

    let new_profile = AccountProfile {
        id: account_id.clone(),
        account_type: "microsoft".to_string(),
        username: session.username,
        uuid: session.uuid,
        skin_url: session.skin_url,
        is_active: true,
        created_at: current_timestamp(),
    };

    let state = crate::app::state(&app);
    let _operation = state
        .account_operations
        .acquire(&account_id)
        .await
        .map_err(|error| error.to_string())?;
    if login.is_cancelled(attempt) {
        return Err("Microsoft sign-in cancelled".to_string());
    }
    credentials::publish(state.credentials.as_ref(), &_operation, &secrets, || {
        repository(&app).activate(new_profile)
    })
    .map_err(|error| error.to_string())
}

fn microsoft_secrets(session: &MinecraftSession) -> AccountSecrets {
    AccountSecrets {
        access_token: session.access_token.clone(),
        refresh_token: Some(session.refresh_token.clone()),
        expires_at: Some(session.expires_at),
        ..Default::default()
    }
}
/// A valid Minecraft token for a Microsoft account, refreshed when close to expiry.
/// Also picks up name and skin changes made on minecraft.net.
async fn microsoft_access_token<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account: &AccountProfile,
    secrets: AccountSecrets,
    operation: &crate::account::operations::AccountOperation,
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
    let next = microsoft_secrets(&session);

    credentials::publish(
        crate::app::state(app).credentials.as_ref(),
        operation,
        &next,
        || {
            repository(app).update_profile(&account.id, |stored| {
                stored.username = session.username.clone();
                stored.skin_url = session.skin_url.clone();
            })
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(session.access_token)
}

/// A valid Minecraft token for the given Microsoft account (not necessarily the active one)
pub(super) async fn microsoft_token_for_locked<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    operation: &crate::account::operations::AccountOperation,
) -> Result<String, String> {
    let state = crate::app::state(app);
    let account_id = operation.id();
    let account = load_accounts_file(app)?
        .into_iter()
        .find(|account| account.id == account_id)
        .ok_or("Account not found")?;
    if account.account_type != "microsoft" {
        return Err("This isn't a Microsoft account".to_string());
    }
    let secrets = credentials::load(state.credentials.as_ref(), account_id)
        .map_err(|error| error.to_string())?;
    microsoft_access_token(app, &account, secrets, operation).await
}
/// Return the active account token, refreshing provider credentials when necessary.
pub(crate) async fn get_active_account_token<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<String, String> {
    let id = load_accounts_file(&app)?
        .into_iter()
        .find(|account| account.is_active)
        .ok_or("No active account selected")?
        .id;
    let state = crate::app::state(&app);
    let _operation = state
        .account_operations
        .acquire(&id)
        .await
        .map_err(|error| error.to_string())?;
    // Recheck after acquiring ownership: removal or re-login may have completed while waiting.
    let active = load_accounts_file(&app)?
        .into_iter()
        .find(|account| account.id == id)
        .ok_or("Account not found")?;
    if active.account_type == "offline" {
        return Ok("offline".to_string());
    }
    let secrets =
        credentials::load(state.credentials.as_ref(), &id).map_err(|error| error.to_string())?;
    if active.account_type == "microsoft" {
        return microsoft_access_token(&app, &active, secrets, &_operation).await;
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
    credentials::publish(
        state.credentials.as_ref(),
        &_operation,
        &updated_secrets,
        || {
            // The operation guard prevents removal/re-login while validation and refresh run.
            repository(&app).update_profile(&active.id, |_| {})
        },
    )
    .map_err(|error| error.to_string())?;

    Ok(refresh_res.access_token)
}
