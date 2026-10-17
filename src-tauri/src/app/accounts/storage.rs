use crate::account::{recovery::RecoveryError, repository::AccountRepository, AccountProfile};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn load_accounts_file<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<AccountProfile>, RecoveryError> {
    with_recovered(app, |repository| repository.load())
}

pub(super) fn with_recovered<R: tauri::Runtime, T>(
    app: &tauri::AppHandle<R>,
    action: impl FnOnce(&AccountRepository) -> Result<T, crate::account::AccountError>,
) -> Result<T, RecoveryError> {
    let state = crate::app::state(app);
    state
        .account_recovery
        .with_recovered(state.credentials.as_ref(), &state.accounts, || {
            action(&state.accounts)
        })
}

pub(super) use with_recovered as with_recovered_typed;

pub(super) fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
