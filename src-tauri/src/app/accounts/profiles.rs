//! Native profile commands; pure list decisions live in account::policy.
use super::storage::{current_timestamp, load_accounts_file, save_accounts_file};
use crate::account::AccountProfile;
use crate::keyring_store;
pub(crate) fn add_offline_account<R: tauri::Runtime>(
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

pub(super) fn save_skin_url<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account_id: &str,
    skin_url: Option<String>,
) -> Result<(), String> {
    let mut accounts = load_accounts_file(app)?;
    if let Some(acc) = accounts.iter_mut().find(|a| a.id == account_id) {
        acc.skin_url = skin_url;
        save_accounts_file(app, &accounts)?;
    }
    Ok(())
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
    let mut accounts = load_accounts_file(&app)?;
    crate::account::policy::select(&mut accounts, &account_id)
        .map_err(|error| error.to_string())?;

    save_accounts_file(&app, &accounts)?;
    Ok(())
}

pub(crate) fn remove_account<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    account_id: String,
) -> Result<(), String> {
    let mut accounts = load_accounts_file(&app)?;
    crate::account::policy::remove(&mut accounts, &account_id)
        .map_err(|error| error.to_string())?;

    // Existing best-effort vault deletion policy; file/vault consistency is a later transaction slice.
    let _ = keyring_store::delete_secret(&account_id);
    save_accounts_file(&app, &accounts)?;
    Ok(())
}

pub(crate) fn reorder_accounts<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    account_ids: Vec<String>,
) -> Result<(), String> {
    let accounts = load_accounts_file(&app)?;
    let reordered = crate::account::policy::reorder(&accounts, &account_ids);

    save_accounts_file(&app, &reordered)?;
    Ok(())
}
