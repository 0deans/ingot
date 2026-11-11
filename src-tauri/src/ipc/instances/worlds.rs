use crate::app::worlds;
use crate::ipc::ApiError;
use ingot_minecraft::worlds::models::{
    WorldBackupInfo, WorldDimension, WorldResetResult, WorldSummary,
};
use tauri::Runtime;

#[taurpc::procedures(path = "instances.worlds")]
pub trait InstanceWorldsApi {
    async fn get_all_worlds(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<WorldSummary>, ApiError>;

    async fn get_instance_worlds_full(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<Vec<WorldSummary>, ApiError>;

    async fn reset_world_dimension(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        folder_name: String,
        dimension: WorldDimension,
    ) -> Result<WorldResetResult, ApiError>;

    async fn export_world(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        folder_name: String,
        dest: Option<String>,
    ) -> Result<String, ApiError>;

    async fn backup_world(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        folder_name: String,
    ) -> Result<WorldBackupInfo, ApiError>;

    async fn list_world_backups(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        folder_name: Option<String>,
    ) -> Result<Vec<WorldBackupInfo>, ApiError>;

    async fn delete_world_backup(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        file_name: String,
    ) -> Result<(), ApiError>;

    async fn import_world(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        zip_path: String,
    ) -> Result<WorldSummary, ApiError>;

    async fn open_world_folder(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        folder_name: String,
    ) -> Result<(), ApiError>;
}

#[derive(Clone)]
pub struct InstanceWorldsApiImpl;

#[taurpc::resolvers]
impl InstanceWorldsApi for InstanceWorldsApiImpl {
    async fn get_all_worlds(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<WorldSummary>, ApiError> {
        worlds::get_all_worlds(&app_handle)
            .await
            .map_err(ApiError::from)
    }

    async fn get_instance_worlds_full(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<Vec<WorldSummary>, ApiError> {
        worlds::get_instance_worlds_full(&app_handle, instance_id)
            .await
            .map_err(ApiError::from)
    }

    async fn reset_world_dimension(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        folder_name: String,
        dimension: WorldDimension,
    ) -> Result<WorldResetResult, ApiError> {
        worlds::reset_world_dimension(&app_handle, instance_id, folder_name, dimension)
            .await
            .map_err(ApiError::from)
    }

    async fn export_world(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        folder_name: String,
        dest: Option<String>,
    ) -> Result<String, ApiError> {
        worlds::export_world(&app_handle, instance_id, folder_name, dest)
            .await
            .map_err(ApiError::from)
    }

    async fn backup_world(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        folder_name: String,
    ) -> Result<WorldBackupInfo, ApiError> {
        worlds::backup_world(&app_handle, instance_id, folder_name)
            .await
            .map_err(ApiError::from)
    }

    async fn list_world_backups(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        folder_name: Option<String>,
    ) -> Result<Vec<WorldBackupInfo>, ApiError> {
        worlds::list_world_backups(&app_handle, instance_id, folder_name)
            .await
            .map_err(ApiError::from)
    }

    async fn delete_world_backup(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        file_name: String,
    ) -> Result<(), ApiError> {
        worlds::delete_world_backup(&app_handle, instance_id, file_name)
            .await
            .map_err(ApiError::from)
    }

    async fn import_world(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        zip_path: String,
    ) -> Result<WorldSummary, ApiError> {
        worlds::import_world(&app_handle, instance_id, zip_path)
            .await
            .map_err(ApiError::from)
    }

    async fn open_world_folder(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        folder_name: String,
    ) -> Result<(), ApiError> {
        let dir = worlds::resolve_world_dir(&app_handle, &instance_id, &folder_name)?;
        crate::app::platform::open_directory(&dir).map_err(ApiError::from)
    }
}
