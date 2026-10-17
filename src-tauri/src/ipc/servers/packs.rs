//! IPC procedures for Bedrock server pack management.
use crate::ipc::ApiError;
use crate::server::{self, bedrock_packs::BedrockPackInfo};
use tauri::Runtime;

#[taurpc::procedures(path = "servers.packs")]
pub trait ServerPacksApi {
    async fn list_bedrock_packs(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<BedrockPackInfo>, ApiError>;

    async fn set_bedrock_pack_enabled(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        pack_id: String,
        pack_type: String,
        enabled: bool,
    ) -> Result<(), ApiError>;

    async fn install_bedrock_pack(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        download_url: String,
        filename: String,
    ) -> Result<String, ApiError>;

    async fn delete_bedrock_pack(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        folder_name: String,
        pack_type: String,
    ) -> Result<(), ApiError>;

    async fn open_bedrock_server_subfolder(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        subfolder: String,
    ) -> Result<(), ApiError>;
}

#[derive(Clone)]
pub struct ServerPacksApiImpl;

#[taurpc::resolvers]
impl ServerPacksApi for ServerPacksApiImpl {
    async fn list_bedrock_packs(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<BedrockPackInfo>, ApiError> {
        let dir = server::get_server_dir(&app_handle, &server_id).map_err(ApiError::from)?;
        server::bedrock_packs::list_installed_packs(&dir).map_err(ApiError::from)
    }

    async fn set_bedrock_pack_enabled(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        pack_id: String,
        pack_type: String,
        enabled: bool,
    ) -> Result<(), ApiError> {
        let dir = server::get_server_dir(&app_handle, &server_id).map_err(ApiError::from)?;
        server::bedrock_packs::set_pack_enabled(&dir, &pack_id, &pack_type, enabled)
            .map_err(ApiError::from)
    }

    async fn install_bedrock_pack(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        download_url: String,
        filename: String,
    ) -> Result<String, ApiError> {
        let dir = server::get_server_dir(&app_handle, &server_id).map_err(ApiError::from)?;
        server::bedrock_packs::install_bedrock_pack(&dir, &download_url, &filename)
            .await
            .map_err(ApiError::from)
    }

    async fn delete_bedrock_pack(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        folder_name: String,
        pack_type: String,
    ) -> Result<(), ApiError> {
        let dir = server::get_server_dir(&app_handle, &server_id).map_err(ApiError::from)?;
        server::bedrock_packs::delete_pack(&dir, &folder_name, &pack_type).map_err(ApiError::from)
    }

    async fn open_bedrock_server_subfolder(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        subfolder: String,
    ) -> Result<(), ApiError> {
        let server_directory =
            server::get_server_dir(&app_handle, &server_id).map_err(ApiError::from)?;
        let directory = crate::app::safe_enclosed_path(&server_directory, &subfolder)
            .ok_or_else(|| ApiError::invalid_input("Invalid server subfolder"))?;
        std::fs::create_dir_all(&directory)
            .map_err(|error| ApiError::storage_failure(error.to_string()))?;
        crate::app::platform::open_directory(&directory).map_err(ApiError::from_display)
    }
}
