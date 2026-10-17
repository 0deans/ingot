//! IPC procedures for server version changes.
use super::{get_http_client, is_downgrade, server_with_config};
use crate::ipc::ApiError;
use crate::server::{ServerConfig, ServerCoreType};
use crate::version_change::targets::{self as change_targets, loader_id};
use crate::version_change::{self, PlanRequest, TargetKind, VersionBackup, VersionPlan};
use tauri::Runtime;

#[taurpc::procedures(path = "servers.version_change")]
pub trait ServerVersionChangeApi {
    /// Works out what moving a server to another Minecraft version means.
    async fn check_server_version_change(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        game_version: String,
        core: ServerCoreType,
    ) -> Result<VersionPlan, ApiError>;

    /// Carries out a reviewed plan, all or nothing, keeping a backup for undo.
    async fn apply_server_version_change(
        app_handle: tauri::AppHandle<impl Runtime>,
        plan: VersionPlan,
        backup_worlds: bool,
    ) -> Result<ServerConfig, ApiError>;

    /// The last version change of a server, if it can be undone.
    async fn get_server_version_backup(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Option<VersionBackup>, ApiError>;

    /// Restores the server exactly as it was before its last version change.
    async fn undo_server_version_change(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<ServerConfig, ApiError>;

    /// Deletes the backup of the server's last version change.
    async fn discard_server_version_backup(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), ApiError>;

    /// Recovers a server from an interrupted or rollback-failed version change transaction.
    async fn recover_server_version_change(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<ServerConfig, ApiError>;

    /// Gets the current version change transaction status, exposing incomplete and rollback-failed states.
    async fn get_server_version_transaction_status(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<version_change::VersionTransactionStatus, ApiError>;
}

#[derive(Clone)]
pub struct ServerVersionChangeApiImpl;

#[taurpc::resolvers]
impl ServerVersionChangeApi for ServerVersionChangeApiImpl {
    async fn check_server_version_change(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        game_version: String,
        core: ServerCoreType,
    ) -> Result<VersionPlan, ApiError> {
        let (dir, config) = server_with_config(&app_handle, &server_id)?;
        if !change_targets::can_switch(&config.core, &core) {
            return Err(ApiError::invalid_input(format!(
                "A {} server can't become a {core} server; create a new one instead",
                config.core
            )));
        }
        let downgrade = is_downgrade(&app_handle, &config.game_version, &game_version).await?;
        let target = change_targets::server(&dir, &core)
            .map_err(|error| ApiError::storage_failure(error.to_string()))?;
        let request = PlanRequest {
            target_kind: TargetKind::Server,
            target_id: server_id,
            from_game_version: config.game_version.clone(),
            from_loader: loader_id(&config.core),
            from_loader_version: config.build_number.clone(),
            to_game_version: game_version,
            to_loader: loader_id(&core),
            to_loader_version: None,
            downgrade,
        };
        version_change::check_typed(get_http_client(&app_handle), &target, request)
            .await
            .map_err(ApiError::from)
    }

    async fn apply_server_version_change(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        plan: VersionPlan,
        backup_worlds: bool,
    ) -> Result<ServerConfig, ApiError> {
        let (directory, _) = server_with_config(&app_handle, &plan.target_id)?;
        let state = crate::app::state(&app_handle);
        version_change::service::apply_server(
            &state.server_configs,
            &state.servers,
            &state.http,
            &directory,
            &plan,
            backup_worlds,
        )
        .await
        .map_err(ApiError::from)
    }

    async fn get_server_version_backup(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Option<VersionBackup>, ApiError> {
        let (dir, _) = server_with_config(&app_handle, &server_id)?;
        Ok(version_change::last_backup(&dir))
    }

    async fn undo_server_version_change(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<ServerConfig, ApiError> {
        let (directory, _) = server_with_config(&app_handle, &server_id)?;
        let state = crate::app::state(&app_handle);
        version_change::service::restore_server(
            &state.server_configs,
            &state.servers,
            &directory,
            &server_id,
            version_change::service::Restoration::Undo,
        )
        .await
        .map_err(ApiError::from)
    }

    async fn discard_server_version_backup(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), ApiError> {
        let state = crate::app::state(&app_handle);
        let _ownership = state
            .servers
            .reserve_maintenance(&server_id)
            .await
            .map_err(|_| ApiError::conflict("Server is busy"))?;
        let (directory, _) = server_with_config(&app_handle, &server_id)?;
        version_change::discard_backup_typed(&directory).map_err(ApiError::from)
    }

    async fn recover_server_version_change(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<ServerConfig, ApiError> {
        let (directory, _) = server_with_config(&app_handle, &server_id)?;
        let state = crate::app::state(&app_handle);
        version_change::service::restore_server(
            &state.server_configs,
            &state.servers,
            &directory,
            &server_id,
            version_change::service::Restoration::Recover,
        )
        .await
        .map_err(ApiError::from)
    }

    async fn get_server_version_transaction_status(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<version_change::VersionTransactionStatus, ApiError> {
        let (dir, _) = server_with_config(&app_handle, &server_id)?;
        Ok(version_change::transaction_status(&dir))
    }
}
