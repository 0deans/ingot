use crate::app::importer;
use crate::ipc::ApiError;
use crate::minecraft::importer::{
    DetectedLauncher, ImportInstanceOptions, ImportReport, ImportableInstance,
};
use tauri::Runtime;

#[taurpc::procedures(path = "instances.imports")]
pub trait InstanceImportsApi {
    async fn get_detected_launchers(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<DetectedLauncher>, ApiError>;

    async fn get_launcher_instances(
        app_handle: tauri::AppHandle<impl Runtime>,
        launcher_id: String,
        custom_path: Option<String>,
    ) -> Result<Vec<ImportableInstance>, ApiError>;

    async fn detect_custom_instance(
        app_handle: tauri::AppHandle<impl Runtime>,
        path: String,
    ) -> Result<ImportableInstance, ApiError>;

    async fn import_instance(
        app_handle: tauri::AppHandle<impl Runtime>,
        options: ImportInstanceOptions,
    ) -> Result<ImportReport, ApiError>;
}

#[derive(Clone)]
pub struct InstanceImportsApiImpl;

#[taurpc::resolvers]
impl InstanceImportsApi for InstanceImportsApiImpl {
    async fn get_detected_launchers(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<DetectedLauncher>, ApiError> {
        importer::get_detected_launchers(&app_handle)
            .await
            .map_err(ApiError::from)
    }

    async fn get_launcher_instances(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        launcher_id: String,
        custom_path: Option<String>,
    ) -> Result<Vec<ImportableInstance>, ApiError> {
        importer::get_launcher_instances(&app_handle, launcher_id, custom_path)
            .await
            .map_err(ApiError::from)
    }

    async fn detect_custom_instance(
        self,
        _app_handle: tauri::AppHandle<impl Runtime>,
        path: String,
    ) -> Result<ImportableInstance, ApiError> {
        importer::detect_custom_instance(path)
            .await
            .map_err(ApiError::from)
    }

    async fn import_instance(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        options: ImportInstanceOptions,
    ) -> Result<ImportReport, ApiError> {
        importer::import_instance(&app_handle, options)
            .await
            .map_err(ApiError::from)
    }
}
