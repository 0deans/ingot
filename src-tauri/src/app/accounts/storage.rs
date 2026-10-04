//! Legacy account file adapter. Atomic updates and corrupt-file handling are a follow-up slice.
use crate::account::AccountProfile;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
fn get_storage_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let file_path = crate::app::state(app).accounts_file.clone();
    // AppPaths always constructs this file beneath the resolved application data directory.
    let data_dir = file_path.parent().ok_or("Invalid accounts storage path")?;

    if !data_dir.exists() {
        fs::create_dir_all(data_dir)
            .map_err(|e| format!("Failed to create app data directory: {e}"))?;
    }

    Ok(file_path)
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

pub(super) fn save_accounts_file<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    accounts: &[AccountProfile],
) -> Result<(), String> {
    let file_path = get_storage_path(app)?;
    let data = serde_json::to_string_pretty(accounts)
        .map_err(|e| format!("Failed to serialize accounts: {e}"))?;

    fs::write(&file_path, data).map_err(|e| format!("Failed to write accounts file: {e}"))?;

    Ok(())
}

pub(super) fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
