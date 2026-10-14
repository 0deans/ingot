//! Thin native adapters; account::session owns provider/session orchestration.
use super::storage::current_timestamp;
use crate::account::pending_login::PendingMicrosoftLogin;
use crate::account::session::SessionError;
use crate::account::{operations::AccountOperation, session::SessionService, AccountProfile};
use crate::auth::microsoft::MicrosoftDeviceCode;

fn service<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> SessionService<'_> {
    let state = crate::app::state(app);
    SessionService {
        repository: &state.accounts,
        credentials: state.credentials.as_ref(),
        operations: &state.account_operations,
        recovery: &state.account_recovery,
        provider: state.session_provider.as_ref(),
        login: &state.account_login,
        pending_login: &state.pending_login,
        now: current_timestamp,
    }
}

pub(crate) async fn ely_login<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    username: String,
    password: String,
) -> Result<AccountProfile, SessionError> {
    service(&app).ely_login(&username, password).await
}
pub(crate) async fn microsoft_login_start<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<MicrosoftDeviceCode, SessionError> {
    service(&app).microsoft_start().await
}
pub(crate) fn microsoft_login_cancel<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<(), SessionError> {
    service(&app).cancel_login()
}
pub(crate) async fn microsoft_login_finish<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    code: MicrosoftDeviceCode,
) -> Result<AccountProfile, SessionError> {
    service(&app).microsoft_finish(&code).await
}
pub(super) async fn microsoft_token_for_locked<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    operation: &AccountOperation,
) -> Result<String, SessionError> {
    service(app).microsoft_token_for_locked(operation).await
}

pub(crate) fn get_pending_microsoft_login<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Option<PendingMicrosoftLogin>, SessionError> {
    service(&app).pending_microsoft_login()
}

pub(crate) async fn resume_microsoft_login<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    ticket: String,
) -> Result<AccountProfile, SessionError> {
    service(&app).resume_microsoft_login(&ticket).await
}

pub(crate) fn discard_pending_microsoft_login<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    ticket: String,
) -> Result<(), SessionError> {
    service(&app).discard_pending_microsoft_login(&ticket)
}
pub(crate) async fn get_active_account_token<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<String, SessionError> {
    service(&app).active_token().await
}

#[cfg(test)]
mod tests;
