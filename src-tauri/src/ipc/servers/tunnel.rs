//! IPC procedures for Playit tunnel management.
use super::get_http_client;
use crate::ipc::ApiError;
use crate::tunnel::{self, PlayitTunnelStatus};
use tauri::{Manager, Runtime};

fn get_playit_manager<R: Runtime>(app: &tauri::AppHandle<R>) -> &tunnel::PlayitManager {
    &crate::app::state(app).playit
}

#[taurpc::procedures(path = "servers.tunnel")]
pub trait ServerTunnelApi {
    async fn start_playit_tunnel(
        app_handle: tauri::AppHandle<impl Runtime>,
        secret_key: Option<String>,
    ) -> Result<PlayitTunnelStatus, ApiError>;

    async fn stop_playit_tunnel(app_handle: tauri::AppHandle<impl Runtime>)
        -> Result<(), ApiError>;

    async fn get_playit_status(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<PlayitTunnelStatus, ApiError>;
}

#[derive(Clone)]
pub struct ServerTunnelApiImpl;

#[taurpc::resolvers]
impl ServerTunnelApi for ServerTunnelApiImpl {
    async fn start_playit_tunnel(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        secret_key: Option<String>,
    ) -> Result<PlayitTunnelStatus, ApiError> {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| ApiError::storage_failure(e.to_string()))?;
        get_playit_manager(&app_handle)
            .start_tunnel(&app_data_dir, get_http_client(&app_handle), secret_key)
            .await
            .map_err(ApiError::from)
    }

    async fn stop_playit_tunnel(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<(), ApiError> {
        get_playit_manager(&app_handle)
            .stop_tunnel()
            .await
            .map_err(ApiError::from)
    }

    async fn get_playit_status(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<PlayitTunnelStatus, ApiError> {
        Ok(get_playit_manager(&app_handle).get_status().await)
    }
}
