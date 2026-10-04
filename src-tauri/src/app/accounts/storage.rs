//! Native access to the application-owned account repository.
use crate::account::{repository::AccountRepository, AccountProfile};
use std::time::{SystemTime, UNIX_EPOCH};

/// Transitional string mapping for existing native account callers.
pub(crate) fn load_accounts_file<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<AccountProfile>, String> {
    with_recovered(app, |repository| repository.load())
}

pub(super) fn with_recovered<R: tauri::Runtime, T>(
    app: &tauri::AppHandle<R>,
    action: impl FnOnce(&AccountRepository) -> Result<T, crate::account::AccountError>,
) -> Result<T, String> {
    let state = crate::app::state(app);
    state
        .account_recovery
        .with_recovered(state.credentials.as_ref(), &state.accounts, || {
            action(&state.accounts)
        })
        .map_err(|error| error.to_string())
}

pub(super) fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
