//! Native access to the application-owned account repository.
use crate::account::{repository::AccountRepository, AccountProfile};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn repository<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> &AccountRepository {
    &crate::app::state(app).accounts
}

/// Transitional string mapping for existing native account callers.
pub(crate) fn load_accounts_file<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<AccountProfile>, String> {
    repository(app).load().map_err(|error| error.to_string())
}

pub(super) fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
