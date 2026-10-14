use crate::ipc::ApiError;
use crate::minecraft::instance::ModLoaderType;
use crate::minecraft::loader;
use crate::minecraft::version::{self, VersionManifestEntry};
use crate::server::{self, ServerCoreType};
use tauri::{Manager, Runtime};

fn get_http_client<R: Runtime>(app: &tauri::AppHandle<R>) -> &reqwest::Client {
    &crate::app::state(app).http
}

#[taurpc::procedures(path = "versions")]
pub trait VersionsApi {
    async fn get_available_game_versions(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<VersionManifestEntry>, ApiError>;

    async fn get_available_loader_versions(
        app_handle: tauri::AppHandle<impl Runtime>,
        game_version: String,
        loader: ModLoaderType,
    ) -> Result<Vec<String>, ApiError>;

    async fn get_available_server_core_versions(
        app_handle: tauri::AppHandle<impl Runtime>,
        core: ServerCoreType,
    ) -> Result<Vec<String>, ApiError>;
}

#[derive(Clone)]
pub struct VersionsApiImpl;

#[taurpc::resolvers]
impl VersionsApi for VersionsApiImpl {
    async fn get_available_game_versions(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<VersionManifestEntry>, ApiError> {
        let data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| ApiError::storage_failure(format!("Failed to get app data directory: {e}")))?;
        let cache_dir = data_dir.join("cache");
        let manifest = version::fetch_version_manifest(get_http_client(&app_handle), &cache_dir)
            .await
            .map_err(ApiError::from)?;
        Ok(manifest.versions)
    }

    async fn get_available_loader_versions(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        game_version: String,
        loader: ModLoaderType,
    ) -> Result<Vec<String>, ApiError> {
        loader::fetch_loader_versions(get_http_client(&app_handle), &loader, &game_version)
            .await
            .map_err(ApiError::from)
    }

    async fn get_available_server_core_versions(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        core: ServerCoreType,
    ) -> Result<Vec<String>, ApiError> {
        server::fetch_core_versions(get_http_client(&app_handle), &core)
            .await
            .map_err(ApiError::from)
    }
}
