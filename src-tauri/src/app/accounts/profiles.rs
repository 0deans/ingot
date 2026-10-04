//! Native profile commands; pure list decisions live in account::policy.
use super::storage::{current_timestamp, load_accounts_file, with_recovered};
use crate::account::AccountProfile;
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
    with_recovered(&app, |repository| repository.activate(new_profile))
}

pub(super) fn save_skin_url<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account_id: &str,
    skin_url: Option<String>,
) -> Result<(), String> {
    with_recovered(app, |repository| {
        repository.update_profile(account_id, |account| account.skin_url = skin_url)
    })
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
    with_recovered(&app, |repository| {
        repository.update(|accounts| {
            crate::account::policy::select(accounts, &account_id)?;
            Ok(())
        })
    })
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
    let accounts = load_accounts_file(&app)?;
    if accounts
        .iter()
        .any(|account| account.id == account_id && account.account_type == "offline")
    {
        return with_recovered(&app, |repository| {
            repository.update(|accounts| {
                crate::account::policy::remove(accounts, &account_id)?;
                Ok(())
            })
        });
    }
    state
        .account_recovery
        .remove(state.credentials.as_ref(), &state.accounts, &_operation)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests;
pub(crate) fn reorder_accounts<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    account_ids: Vec<String>,
) -> Result<(), String> {
    with_recovered(&app, |repository| {
        repository.update(|accounts| {
            *accounts = crate::account::policy::reorder(accounts, &account_ids);
            Ok(())
        })
    })
}
