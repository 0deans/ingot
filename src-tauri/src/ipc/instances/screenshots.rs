use crate::ipc::ApiError;
use crate::minecraft::screenshots::{self, ScreenshotInfo};
use tauri::Runtime;

#[taurpc::procedures(path = "instances.screenshots")]
pub trait InstanceScreenshotsApi {
    async fn get_all_screenshots(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<ScreenshotInfo>, ApiError>;

    async fn get_screenshot_thumbnail(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        file_name: String,
        file_path: String,
        modified_at: u64,
    ) -> Result<String, ApiError>;

    async fn delete_screenshot(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        file_name: String,
    ) -> Result<(), ApiError>;

    async fn open_screenshots_folder(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: Option<String>,
    ) -> Result<(), ApiError>;

    async fn reveal_screenshot_file(file_path: String) -> Result<(), ApiError>;
}

#[derive(Clone)]
pub struct InstanceScreenshotsApiImpl;

#[taurpc::resolvers]
impl InstanceScreenshotsApi for InstanceScreenshotsApiImpl {
    async fn get_all_screenshots(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<ScreenshotInfo>, ApiError> {
        screenshots::get_all_screenshots(&app_handle).map_err(ApiError::from)
    }

    async fn get_screenshot_thumbnail(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        file_name: String,
        file_path: String,
        modified_at: u64,
    ) -> Result<String, ApiError> {
        tokio::task::spawn_blocking(move || {
            screenshots::get_or_create_thumbnail(
                &app_handle,
                &instance_id,
                &file_name,
                &file_path,
                modified_at,
            )
        })
        .await
        .map_err(ApiError::from_display)?
        .map_err(ApiError::from)
    }

    async fn delete_screenshot(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        file_name: String,
    ) -> Result<(), ApiError> {
        screenshots::delete_screenshot(&app_handle, &instance_id, &file_name).map_err(ApiError::from)
    }

    async fn open_screenshots_folder(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: Option<String>,
    ) -> Result<(), ApiError> {
        screenshots::open_screenshots_folder(&app_handle, instance_id.as_deref())
            .map_err(ApiError::from)
    }

    async fn reveal_screenshot_file(self, file_path: String) -> Result<(), ApiError> {
        screenshots::reveal_screenshot_file(&file_path).map_err(ApiError::from)
    }
}
