use tauri::Runtime;

use crate::app::instance_content;
use crate::ipc::ApiError;
use crate::minecraft::instance_content::{ContentCategory, ContentListResult};

#[taurpc::procedures(path = "instances.content")]
pub trait InstanceContentApi {
    async fn list_instance_content(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        category: ContentCategory,
    ) -> Result<ContentListResult, ApiError>;

    async fn set_content_enabled(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        category: ContentCategory,
        file_name: String,
        enabled: bool,
    ) -> Result<String, ApiError>;

    async fn set_content_enabled_bulk(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        category: ContentCategory,
        file_names: Vec<String>,
        enabled: bool,
    ) -> Result<Vec<String>, ApiError>;

    async fn delete_content(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        category: ContentCategory,
        file_names: Vec<String>,
    ) -> Result<Vec<String>, ApiError>;

    async fn open_content_folder(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        category: ContentCategory,
        world: Option<String>,
    ) -> Result<(), ApiError>;
}

#[derive(Clone)]
pub struct InstanceContentApiImpl;

#[taurpc::resolvers]
impl InstanceContentApi for InstanceContentApiImpl {
    async fn list_instance_content(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        category: ContentCategory,
    ) -> Result<ContentListResult, ApiError> {
        instance_content::list_content(&app_handle, &instance_id, category)
            .await
            .map_err(ApiError::from)
    }

    async fn set_content_enabled(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        category: ContentCategory,
        file_name: String,
        enabled: bool,
    ) -> Result<String, ApiError> {
        instance_content::set_content_enabled(
            &app_handle,
            &instance_id,
            category,
            file_name,
            enabled,
        )
        .await
        .map_err(ApiError::from)
    }

    async fn set_content_enabled_bulk(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        category: ContentCategory,
        file_names: Vec<String>,
        enabled: bool,
    ) -> Result<Vec<String>, ApiError> {
        instance_content::set_content_enabled_bulk(
            &app_handle,
            &instance_id,
            category,
            file_names,
            enabled,
        )
        .await
        .map_err(ApiError::from)
    }

    async fn delete_content(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        category: ContentCategory,
        file_names: Vec<String>,
    ) -> Result<Vec<String>, ApiError> {
        instance_content::delete_content(&app_handle, &instance_id, category, file_names)
            .await
            .map_err(ApiError::from)
    }

    async fn open_content_folder(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        category: ContentCategory,
        world: Option<String>,
    ) -> Result<(), ApiError> {
        instance_content::open_content_folder(&app_handle, &instance_id, category, world)
            .await
            .map_err(ApiError::from)
    }
}
