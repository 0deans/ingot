//! IPC procedures for server world map rendering.
use crate::app::server_config::get_server_dir;
use crate::ipc::ApiError;
use ingot_server_map::map::{dimensions, tile_typed, MapDimension};
use tauri::Runtime;

#[taurpc::procedures(path = "servers.map")]
pub trait ServerMapApi {
    async fn get_map_dimensions(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<MapDimension>, ApiError>;

    /// A region tile as a PNG data URL (None if the region doesn't exist).
    async fn get_map_tile(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        dimension: String,
        x: i32,
        z: i32,
    ) -> Result<Option<String>, ApiError>;
}

#[derive(Clone)]
pub struct ServerMapApiImpl;

#[taurpc::resolvers]
impl ServerMapApi for ServerMapApiImpl {
    async fn get_map_dimensions(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<MapDimension>, ApiError> {
        let dir = get_server_dir(&app_handle, &server_id).map_err(ApiError::from)?;
        tauri::async_runtime::spawn_blocking(move || dimensions(&dir))
            .await
            .map_err(|e| ApiError::internal(e.to_string()))
    }

    async fn get_map_tile(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        dimension: String,
        x: i32,
        z: i32,
    ) -> Result<Option<String>, ApiError> {
        let dir = get_server_dir(&app_handle, &server_id).map_err(ApiError::from)?;
        tauri::async_runtime::spawn_blocking(move || tile_typed(&dir, &dimension, x, z))
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?
            .map_err(ApiError::from)
    }
}
