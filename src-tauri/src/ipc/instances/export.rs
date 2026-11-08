use tauri::Runtime;

use crate::app::instance_export;
use crate::ipc::ApiError;
use crate::minecraft::instance_export::{ExportOptions, ExportPreview, ExportReport};

#[taurpc::procedures(path = "instances.export")]
pub trait InstanceExportApi {
    async fn preview_instance_export(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<ExportPreview, ApiError>;

    async fn export_instance(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        options: ExportOptions,
        dest: Option<String>,
    ) -> Result<ExportReport, ApiError>;
}

#[derive(Clone)]
pub struct InstanceExportApiImpl;

#[taurpc::resolvers]
impl InstanceExportApi for InstanceExportApiImpl {
    async fn preview_instance_export(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<ExportPreview, ApiError> {
        instance_export::preview_instance_export(&app_handle, instance_id)
            .await
            .map_err(ApiError::from)
    }

    async fn export_instance(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        options: ExportOptions,
        dest: Option<String>,
    ) -> Result<ExportReport, ApiError> {
        instance_export::export_instance_archive(&app_handle, instance_id, options, dest)
            .await
            .map_err(ApiError::from)
    }
}
