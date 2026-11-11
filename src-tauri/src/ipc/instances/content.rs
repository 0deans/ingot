use tauri::Runtime;

use crate::app::instance_content;
use crate::ipc::ApiError;
use ingot_minecraft::instance_content::{
    ContentApplyReport, ContentCategory, ContentListResult, ContentUpdateCandidate, IdentifyResult,
    UpdateCheckResult,
};

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

    async fn identify_instance_content(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        category: ContentCategory,
    ) -> Result<IdentifyResult, ApiError>;

    async fn check_content_updates(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        category: ContentCategory,
    ) -> Result<UpdateCheckResult, ApiError>;

    async fn apply_content_updates(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        category: ContentCategory,
        selections: Vec<ContentUpdateCandidate>,
    ) -> Result<ContentApplyReport, ApiError>;
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

    async fn identify_instance_content(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        category: ContentCategory,
    ) -> Result<IdentifyResult, ApiError> {
        instance_content::identify_content(&app_handle, &instance_id, category)
            .await
            .map_err(ApiError::from)
    }

    async fn check_content_updates(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        category: ContentCategory,
    ) -> Result<UpdateCheckResult, ApiError> {
        instance_content::check_content_updates(&app_handle, &instance_id, category)
            .await
            .map_err(ApiError::from)
    }

    async fn apply_content_updates(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        category: ContentCategory,
        selections: Vec<ContentUpdateCandidate>,
    ) -> Result<ContentApplyReport, ApiError> {
        instance_content::apply_content_updates(&app_handle, &instance_id, category, selections)
            .await
            .map_err(ApiError::from)
    }
}
