use crate::ipc::ApiError;
use ingot_minecraft::instance::ModLoaderType;
use ingot_minecraft::loader;
use ingot_minecraft::version::{self, VersionManifestEntry};
use ingot_server::ServerCoreType;
use tauri::{Manager, Runtime};

fn get_http_client<R: Runtime>(app: &tauri::AppHandle<R>) -> &reqwest::Client {
    &crate::app::state(app).http
}

pub(crate) async fn is_downgrade<R: Runtime>(
    app: &tauri::AppHandle<R>,
    from: &str,
    to: &str,
) -> Result<bool, ApiError> {
    let cache_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| ApiError::storage_failure(e.to_string()))?
        .join("cache");
    let manifest = version::fetch_version_manifest(get_http_client(app), &cache_dir)
        .await
        .map_err(ApiError::from)?;
    let released = |id: &str| {
        manifest
            .versions
            .iter()
            .find(|v| v.id == id)
            .map(|v| v.release_time.clone())
    };
    Ok(matches!(
        (released(from), released(to)),
        (Some(from), Some(to)) if to < from
    ))
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

    async fn get_available_horizon_releases(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<ingot_minecraft::horizon::HorizonRelease>, ApiError>;
}

#[derive(Clone)]
pub struct VersionsApiImpl;

#[taurpc::resolvers]
impl VersionsApi for VersionsApiImpl {
    async fn get_available_game_versions(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<VersionManifestEntry>, ApiError> {
        let data_dir = app_handle.path().app_data_dir().map_err(|e| {
            ApiError::storage_failure(format!("Failed to get app data directory: {e}"))
        })?;
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
        if loader == ModLoaderType::Horizon {
            let data_dir = app_handle.path().app_data_dir().map_err(|e| {
                ApiError::storage_failure(format!("Failed to get app data directory: {e}"))
            })?;
            let cache_dir = data_dir.join("cache");
            let releases = ingot_minecraft::horizon::fetch_horizon_releases(
                get_http_client(&app_handle),
                &cache_dir,
            )
            .await
            .map_err(ApiError::from)?;
            return Ok(releases.into_iter().map(|r| r.tag).collect());
        }
        loader::fetch_loader_versions(get_http_client(&app_handle), &loader, &game_version)
            .await
            .map_err(ApiError::from)
    }

    async fn get_available_server_core_versions(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        core: ServerCoreType,
    ) -> Result<Vec<String>, ApiError> {
        ingot_server::fetch_core_versions(get_http_client(&app_handle), &core)
            .await
            .map_err(ApiError::from)
    }

    async fn get_available_horizon_releases(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<ingot_minecraft::horizon::HorizonRelease>, ApiError> {
        let data_dir = app_handle.path().app_data_dir().map_err(|e| {
            ApiError::storage_failure(format!("Failed to get app data directory: {e}"))
        })?;
        let cache_dir = data_dir.join("cache");
        ingot_minecraft::horizon::fetch_horizon_releases(get_http_client(&app_handle), &cache_dir)
            .await
            .map_err(ApiError::from)
    }
}
