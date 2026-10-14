use crate::ipc::ApiError;
use crate::minecraft::sync::{self, SharedSyncStatus, SyncConflictInfo, SyncReport};
use tauri::Runtime;

#[taurpc::procedures(path = "instances.sync")]
pub trait InstanceSyncApi {
    async fn push_instance_sync(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<SyncReport, ApiError>;

    async fn export_instance_category_to_shared(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        category: String,
    ) -> Result<SyncReport, ApiError>;

    async fn pull_instance_sync(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<SyncReport, ApiError>;

    async fn get_shared_sync_status(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<SharedSyncStatus, ApiError>;

    async fn check_sync_conflict(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<Option<SyncConflictInfo>, ApiError>;

    async fn resolve_sync_conflict(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        resolution: String,
    ) -> Result<SyncReport, ApiError>;
}

#[derive(Clone)]
pub struct InstanceSyncApiImpl;

#[taurpc::resolvers]
impl InstanceSyncApi for InstanceSyncApiImpl {
    async fn push_instance_sync(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<SyncReport, ApiError> {
        sync::export_instance_to_shared(&app_handle, &instance_id).map_err(ApiError::from)
    }

    async fn export_instance_category_to_shared(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        category: String,
    ) -> Result<SyncReport, ApiError> {
        sync::export_instance_category_to_shared(&app_handle, &instance_id, &category)
            .map_err(ApiError::from)
    }

    async fn pull_instance_sync(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<SyncReport, ApiError> {
        sync::import_shared_to_instance(&app_handle, &instance_id).map_err(ApiError::from)
    }

    async fn get_shared_sync_status(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<SharedSyncStatus, ApiError> {
        sync::get_shared_sync_status(&app_handle).map_err(ApiError::from)
    }

    async fn check_sync_conflict(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<Option<SyncConflictInfo>, ApiError> {
        sync::check_sync_conflict(&app_handle, &instance_id).map_err(ApiError::from)
    }

    async fn resolve_sync_conflict(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        resolution: String,
    ) -> Result<SyncReport, ApiError> {
        sync::resolve_sync_conflict(&app_handle, &instance_id, &resolution).map_err(ApiError::from)
    }
}
