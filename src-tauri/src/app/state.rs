use crate::minecraft::launcher::ProcessManager;
use crate::server::{PlayitManager, ServerProcessManager, ServerSupervisorManager};
use crate::settings::repository::SettingsRepository;
use tauri::Manager;

use super::{lifecycle::LifecycleState, AppPaths};

pub(crate) struct AppState {
    pub(crate) settings: SettingsRepository,
    pub(crate) games: ProcessManager,
    pub(crate) servers: ServerProcessManager,
    pub(crate) supervisor: ServerSupervisorManager,
    pub(crate) playit: PlayitManager,
    pub(crate) http: reqwest::Client,
    pub(crate) lifecycle: LifecycleState,
    pub(crate) account_login: crate::account::login::LoginAttempts,
    pub(crate) accounts: crate::account::repository::AccountRepository,
    pub(crate) account_operations: crate::account::operations::AccountOperations,
    pub(crate) credentials: Box<dyn crate::account::credentials::CredentialStore>,
}

impl AppState {
    pub(crate) fn new(paths: AppPaths) -> Result<Self, reqwest::Error> {
        let http = reqwest::Client::builder()
            .user_agent(crate::USER_AGENT)
            .build()?;
        Ok(Self {
            settings: SettingsRepository::new(paths.settings_file()),
            games: ProcessManager::new(),
            servers: ServerProcessManager::new(),
            supervisor: ServerSupervisorManager::new(),
            playit: PlayitManager::new(),
            http,
            lifecycle: LifecycleState::default(),
            account_login: crate::account::login::LoginAttempts::default(),
            accounts: crate::account::repository::AccountRepository::new(paths.accounts_file()),
            account_operations: crate::account::operations::AccountOperations::default(),
            credentials: Box::new(super::accounts::vault::OsCredentialStore),
        })
    }
}

/// App setup registers this state before starting tasks or accepting commands.
/// Missing state is an application wiring error, rather than a user-input error.
pub(crate) fn state<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> &AppState {
    app.state::<AppState>().inner()
}

#[cfg(test)]
mod tests;
