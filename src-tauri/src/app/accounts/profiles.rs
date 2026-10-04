//! Native profile commands; pure list decisions live in account::policy.
use super::storage::{current_timestamp, load_accounts_file, repository};
use crate::account::credentials::CredentialError;
use crate::account::{repository::AccountRepository, AccountError, AccountProfile};
pub(crate) async fn add_offline_account<R: tauri::Runtime>(
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
    let new_profile = AccountProfile {
        id: account_id.clone(),
        account_type: "offline".to_string(),
        username: clean_name.to_string(),
        uuid: offline_uuid,
        skin_url: None,
        is_active: true,
        created_at: current_timestamp(),
    };

    let _operation = crate::app::state(&app)
        .account_operations
        .acquire(&account_id)
        .await
        .map_err(|error| error.to_string())?;
    repository(&app)
        .activate(new_profile)
        .map_err(|error| error.to_string())
}

pub(super) fn save_skin_url<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account_id: &str,
    skin_url: Option<String>,
) -> Result<(), String> {
    repository(app)
        .update_profile(account_id, |account| account.skin_url = skin_url)
        .map_err(|error| error.to_string())
}
pub(crate) fn get_accounts<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Vec<AccountProfile>, String> {
    load_accounts_file(&app)
}

pub(crate) fn set_active_account<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    account_id: String,
) -> Result<(), String> {
    repository(&app)
        .update(|accounts| {
            crate::account::policy::select(accounts, &account_id)?;
            Ok(())
        })
        .map_err(|error| error.to_string())
}
pub(crate) async fn remove_account<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    account_id: String,
) -> Result<(), String> {
    let state = crate::app::state(&app);
    let _operation = state
        .account_operations
        .acquire(&account_id)
        .await
        .map_err(|error| error.to_string())?;
    remove_with_cleanup(repository(&app), &account_id, |id| {
        state.credentials.delete(id)
    })
    .map_err(|error| error.to_string())
}

fn remove_with_cleanup(
    repository: &AccountRepository,
    account_id: &str,
    cleanup: impl FnOnce(&str) -> Result<(), CredentialError>,
) -> Result<(), AccountError> {
    repository.update(|accounts| {
        crate::account::policy::remove(accounts, account_id)?;
        Ok(())
    })?;

    // Commit the profile removal first. Vault cleanup is best effort, outside the store lock.
    if let Err(error) = cleanup(account_id) {
        eprintln!("Account removed, but credential cleanup failed: {error}");
    }
    Ok(())
}

#[cfg(test)]
mod tests;
pub(crate) fn reorder_accounts<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    account_ids: Vec<String>,
) -> Result<(), String> {
    repository(&app)
        .update(|accounts| {
            *accounts = crate::account::policy::reorder(accounts, &account_ids);
            Ok(())
        })
        .map_err(|error| error.to_string())
}
