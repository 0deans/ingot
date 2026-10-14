//! IPC procedures for server version changes.
use super::{get_http_client, get_server_process_manager, is_downgrade, server_with_config};
use crate::ipc::ApiError;
use crate::server::{self, ServerConfig, ServerCoreType, ServerStatus};
use crate::version_change::targets::{self as change_targets, loader_id, parse_loader};
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
        let target = change_targets::server(&dir, &core);
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
        version_change::check(get_http_client(&app_handle), &target, request)
            .await
            .map_err(ApiError::from)
    }

    async fn apply_server_version_change(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        plan: VersionPlan,
        backup_worlds: bool,
    ) -> Result<ServerConfig, ApiError> {
        if get_server_process_manager(&app_handle).get_server_status(&plan.target_id).await != ServerStatus::Stopped {
            return Err(ApiError::conflict("Stop the server before changing its version"));
        }
        let (dir, config) = server_with_config(&app_handle, &plan.target_id)?;
        let core: ServerCoreType = parse_loader(&plan.to_loader).map_err(ApiError::invalid_input)?;
        if plan.target_kind != TargetKind::Server
            || plan.from_game_version != config.game_version
            || plan.from_loader != loader_id(&config.core)
            || !change_targets::can_switch(&config.core, &core)
        {
            return Err(ApiError::conflict("The server changed since it was checked. Check again."));
        }
        let target = change_targets::server(&dir, &core);
        match version_change::detect_incomplete_transaction(&dir) {
            Ok(Some(incomplete)) => {
                return Err(ApiError::incomplete_recovery(format!(
                    "Cannot apply version change: an incomplete transaction ({:?}) is pending recovery. Please recover or revert it first.",
                    incomplete.phase
                )));
            }
            Err(err) => {
                return Err(ApiError::incomplete_recovery(format!(
                    "Cannot apply version change: {err}"
                )));
            }
            Ok(None) => {}
        }

        let mut updated = config.clone();
        updated.game_version = plan.to_game_version.clone();
        updated.core = core;
        updated.build_number = None;

        let dir_clone = dir.clone();
        let plan_items = plan.items.clone();
        let app_h = app_handle.clone();
        let updated_to_save = updated.clone();

        version_change::apply_with_commit(
            get_http_client(&app_handle),
            &target,
            &plan,
            backup_worlds,
            |_| {},
            || {
                server::plugins::track_version_change(&dir_clone, &plan_items);
                server::update_server(&app_h, updated_to_save)
            },
        )
        .await
        .map_err(ApiError::from)?;

        Ok(updated)
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
        if get_server_process_manager(&app_handle).get_server_status(&server_id).await != ServerStatus::Stopped {
            return Err(ApiError::conflict("Stop the server first"));
        }
        let (dir, config) = server_with_config(&app_handle, &server_id)?;
        let app_h = app_handle.clone();
        let config_clone = config.clone();
        let restored = version_change::undo_with_commit(
            &dir,
            &config.game_version,
            &loader_id(&config.core),
            |restored| {
                let mut to_update = config_clone;
                to_update.game_version = restored.game_version.clone();
                to_update.core = parse_loader(&restored.loader)?;
                to_update.build_number = restored.loader_version.clone();
                server::update_server(&app_h, to_update)
            },
        )
        .map_err(ApiError::from)?;
        let mut updated = config.clone();
        updated.game_version = restored.game_version;
        updated.core = parse_loader(&restored.loader).map_err(ApiError::invalid_input)?;
        updated.build_number = restored.loader_version;
        Ok(updated)
    }

    async fn discard_server_version_backup(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), ApiError> {
        let (dir, _) = server_with_config(&app_handle, &server_id)?;
        version_change::discard_backup(&dir).map_err(ApiError::from)
    }

    async fn recover_server_version_change(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<ServerConfig, ApiError> {
        if get_server_process_manager(&app_handle).get_server_status(&server_id).await != ServerStatus::Stopped {
            return Err(ApiError::conflict("Stop the server first"));
        }
        let (dir, config) = server_with_config(&app_handle, &server_id)?;
        let app_h = app_handle.clone();
        let config_clone = config.clone();
        let restored = version_change::recover_interrupted_transaction_with_commit(
            &dir,
            |restored| {
                let mut to_update = config_clone;
                to_update.game_version = restored.game_version.clone();
                to_update.core = parse_loader(&restored.loader)?;
                to_update.build_number = restored.loader_version.clone();
                server::update_server(&app_h, to_update)
            },
        )
        .map_err(ApiError::from)?;
        let mut updated = config.clone();
        updated.game_version = restored.game_version;
        updated.core = parse_loader(&restored.loader).map_err(ApiError::invalid_input)?;
        updated.build_number = restored.loader_version;
        Ok(updated)
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
