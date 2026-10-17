use crate::ipc::ApiError;
use crate::minecraft::content::{self, ContentSearchResult, UnifiedContentDetails};
use crate::minecraft::instance::InstanceConfig;
use tauri::Runtime;

#[taurpc::procedures(path = "content")]
#[allow(clippy::too_many_arguments)]
pub trait ContentApi {
    #[allow(clippy::too_many_arguments)]
    async fn search_content(
        source: String,
        project_type: String,
        query: Option<String>,
        game_version: Option<String>,
        loader: Option<String>,
        sort: Option<String>,
        page: u32,
        page_size: u32,
    ) -> Result<ContentSearchResult, ApiError>;

    async fn get_content_details(
        source: String,
        project_id: String,
    ) -> Result<UnifiedContentDetails, ApiError>;

    async fn install_content_file(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        project_type: String,
        download_url: String,
        filename: String,
    ) -> Result<String, ApiError>;

    async fn install_modpack_instance(
        app_handle: tauri::AppHandle<impl Runtime>,
        name: String,
        source: String,
        download_url: String,
        filename: String,
    ) -> Result<InstanceConfig, ApiError>;
}

#[derive(Clone)]
pub struct ContentApiImpl;

#[taurpc::resolvers]
impl ContentApi for ContentApiImpl {
    async fn search_content(
        self,
        source: String,
        project_type: String,
        query: Option<String>,
        game_version: Option<String>,
        loader: Option<String>,
        sort: Option<String>,
        page: u32,
        page_size: u32,
    ) -> Result<ContentSearchResult, ApiError> {
        content::search_content(
            source,
            project_type,
            query,
            game_version,
            loader,
            sort,
            page,
            page_size,
        )
        .await
        .map_err(ApiError::from)
    }

    async fn get_content_details(
        self,
        source: String,
        project_id: String,
    ) -> Result<UnifiedContentDetails, ApiError> {
        content::get_content_details(&source, &project_id)
            .await
            .map_err(ApiError::from)
    }

    async fn install_content_file(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        project_type: String,
        download_url: String,
        filename: String,
    ) -> Result<String, ApiError> {
        content::install_content_file(
            &app_handle,
            &instance_id,
            &project_type,
            &download_url,
            &filename,
        )
        .await
        .map_err(ApiError::from)
    }

    async fn install_modpack_instance(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        name: String,
        source: String,
        download_url: String,
        filename: String,
    ) -> Result<InstanceConfig, ApiError> {
        content::install_modpack_instance(&app_handle, &name, &source, &download_url, &filename)
            .await
            .map_err(ApiError::from)
    }
}
