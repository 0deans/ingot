use super::{find_instance, get_http_client, is_downgrade};
use crate::ipc::ApiError;
use crate::minecraft::instance::{self, InstanceConfig, ModLoaderType};
use crate::version_change::targets::{self as change_targets, loader_id};
use crate::version_change::{self, PlanRequest, TargetKind, VersionBackup, VersionPlan};
use tauri::Runtime;

#[taurpc::procedures(path = "instances.version_change")]
pub trait InstanceVersionChangeApi {
    /// Works out what moving an instance to another Minecraft version means.
    async fn check_instance_version_change(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        game_version: String,
        loader: ModLoaderType,
        loader_version: Option<String>,
    ) -> Result<VersionPlan, ApiError>;

    /// Carries out a reviewed plan, all or nothing, keeping a backup for undo.
    async fn apply_instance_version_change(
        app_handle: tauri::AppHandle<impl Runtime>,
        plan: VersionPlan,
        backup_worlds: bool,
    ) -> Result<InstanceConfig, ApiError>;

    /// The last version change of an instance, if it can be undone.
    async fn get_instance_version_backup(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<Option<VersionBackup>, ApiError>;

    /// Restores the instance exactly as it was before its last version change.
    async fn undo_instance_version_change(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<InstanceConfig, ApiError>;

    /// Deletes the backup of the last version change.
    async fn discard_instance_version_backup(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<(), ApiError>;

    /// Recovers the instance from an interrupted or rollback-failed version change transaction.
    async fn recover_instance_version_change(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<InstanceConfig, ApiError>;

    /// Gets the current version change transaction status, exposing incomplete and rollback-failed states.
    async fn get_instance_version_transaction_status(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<version_change::VersionTransactionStatus, ApiError>;
}

#[derive(Clone)]
pub struct InstanceVersionChangeApiImpl;

#[taurpc::resolvers]
impl InstanceVersionChangeApi for InstanceVersionChangeApiImpl {
    async fn check_instance_version_change(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        game_version: String,
        loader: ModLoaderType,
        loader_version: Option<String>,
    ) -> Result<VersionPlan, ApiError> {
        let instance = find_instance(&app_handle, &instance_id)?;
        if loader != ModLoaderType::Vanilla && loader_version.is_none() {
            let name = version_change::loader_label(&loader_id(&loader));
            return Err(ApiError::invalid_input(format!(
                "Pick a {name} version for Minecraft {game_version}"
            )));
        }
        let downgrade = is_downgrade(&app_handle, &instance.game_version, &game_version).await?;
        let instance_dir =
            instance::get_instance_dir(&app_handle, &instance_id).map_err(ApiError::from)?;
        let (from, to) = (loader_id(&instance.loader), loader_id(&loader));
        let target = change_targets::instance(&instance_dir, &from, &to);
        let request = PlanRequest {
            target_kind: TargetKind::Instance,
            target_id: instance_id,
            from_game_version: instance.game_version.clone(),
            from_loader: from,
            from_loader_version: instance.loader_version.clone(),
            to_game_version: game_version,
            to_loader: to,
            to_loader_version: loader_version.filter(|_| loader != ModLoaderType::Vanilla),
            downgrade,
        };
        version_change::check(get_http_client(&app_handle), &target, request)
            .await
            .map_err(ApiError::from)
    }

    async fn apply_instance_version_change(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        plan: VersionPlan,
        backup_worlds: bool,
    ) -> Result<InstanceConfig, ApiError> {
        let directory =
            instance::get_instance_dir(&app_handle, &plan.target_id).map_err(ApiError::from)?;
        let state = crate::app::state(&app_handle);
        version_change::service::apply_instance(
            &state.instances,
            &state.games,
            &state.http,
            &directory,
            &plan,
            backup_worlds,
        )
        .await
        .map_err(ApiError::from)
    }

    async fn get_instance_version_backup(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<Option<VersionBackup>, ApiError> {
        let instance_dir =
            instance::get_instance_dir(&app_handle, &instance_id).map_err(ApiError::from)?;
        Ok(version_change::last_backup(&instance_dir))
    }

    async fn undo_instance_version_change(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<InstanceConfig, ApiError> {
        let directory =
            instance::get_instance_dir(&app_handle, &instance_id).map_err(ApiError::from)?;
        let state = crate::app::state(&app_handle);
        version_change::service::restore_instance(
            &state.instances,
            &state.games,
            &directory,
            &instance_id,
            version_change::service::Restoration::Undo,
        )
        .map_err(ApiError::from)
    }

    async fn discard_instance_version_backup(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<(), ApiError> {
        let state = crate::app::state(&app_handle);
        let _ownership = state
            .games
            .reserve_maintenance(&instance_id)
            .map_err(|_| ApiError::conflict("Instance is busy"))?;
        find_instance(&app_handle, &instance_id)?;
        let directory =
            instance::get_instance_dir(&app_handle, &instance_id).map_err(ApiError::from)?;
        version_change::discard_backup(&directory).map_err(ApiError::from)
    }

    async fn recover_instance_version_change(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<InstanceConfig, ApiError> {
        let directory =
            instance::get_instance_dir(&app_handle, &instance_id).map_err(ApiError::from)?;
        let state = crate::app::state(&app_handle);
        version_change::service::restore_instance(
            &state.instances,
            &state.games,
            &directory,
            &instance_id,
            version_change::service::Restoration::Recover,
        )
        .map_err(ApiError::from)
    }

    async fn get_instance_version_transaction_status(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<version_change::VersionTransactionStatus, ApiError> {
        let instance_dir =
            instance::get_instance_dir(&app_handle, &instance_id).map_err(ApiError::from)?;
        Ok(version_change::transaction_status(&instance_dir))
    }
}
