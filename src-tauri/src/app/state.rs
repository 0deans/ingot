use crate::minecraft::launcher::ProcessManager;
use crate::server::{PlayitManager, ServerProcessManager, ServerSupervisorManager};
use crate::settings::repository::SettingsRepository;
use tauri::Manager;

use super::{lifecycle::LifecycleState, AppPaths};

pub(crate) struct AppState {
    pub(crate) instances:
        crate::minecraft::metadata::MetadataRepository<crate::minecraft::instance::InstanceConfig>,
    pub(crate) server_configs:
        crate::minecraft::metadata::MetadataRepository<crate::server::ServerConfig>,
    pub(crate) settings: SettingsRepository,
    pub(crate) games: ProcessManager,
    pub(crate) servers: ServerProcessManager,
    pub(crate) supervisor: ServerSupervisorManager,
    pub(crate) playit: PlayitManager,
    pub(crate) http: reqwest::Client,
    pub(crate) lifecycle: LifecycleState,
    pub(crate) account_login: crate::account::login::LoginAttempts,
    pub(crate) pending_login: crate::account::pending_login::PendingLoginGrants,
    pub(crate) accounts: crate::account::repository::AccountRepository,
    pub(crate) account_operations: crate::account::operations::AccountOperations,
    pub(crate) account_recovery: crate::account::recovery::AccountRecovery,
    pub(crate) credentials: Box<dyn crate::account::credentials::CredentialStore>,
    pub(crate) session_provider: Box<dyn crate::account::provider::SessionProvider>,
}

impl AppState {
    pub(crate) fn new(paths: AppPaths) -> Result<Self, reqwest::Error> {
        let http = reqwest::Client::builder()
            .user_agent(crate::USER_AGENT)
            .build()?;
        Ok(Self {
            instances: crate::minecraft::metadata::MetadataRepository::new(paths.instances_file()),
            server_configs: crate::minecraft::metadata::MetadataRepository::new(
                paths.servers_file(),
            ),
            settings: SettingsRepository::new(paths.settings_file()),
            games: ProcessManager::new(),
            servers: ServerProcessManager::new(),
            supervisor: ServerSupervisorManager::new(),
            playit: PlayitManager::new(),
            session_provider: Box::new(crate::auth::session_provider::NativeSessionProvider::new(
                http.clone(),
            )),
            http,
            lifecycle: LifecycleState::default(),
            account_login: crate::account::login::LoginAttempts::default(),
            pending_login: crate::account::pending_login::PendingLoginGrants::new(
                &paths.accounts_file(),
            ),
            accounts: crate::account::repository::AccountRepository::new(paths.accounts_file()),
            account_operations: crate::account::operations::AccountOperations::default(),
            account_recovery: crate::account::recovery::AccountRecovery::new(
                paths.accounts_file().with_extension("recovery"),
            ),
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
