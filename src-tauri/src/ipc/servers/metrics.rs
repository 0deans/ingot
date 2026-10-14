//! IPC procedures for server metrics, storage, and networking.
use super::{core_has_tps, get_server_process_manager, is_server_running, server_with_config};
use crate::ipc::ApiError;
use crate::server;
use crate::server::stats::ServerStats;
use crate::server::storage::ServerStorage;
use tauri::Runtime;

#[taurpc::procedures(path = "servers.metrics")]
pub trait ServerMetricsApi {
    /// CPU/RAM/TPS samples of the last two minutes.
    async fn get_server_stats(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<ServerStats>, ApiError>;

    /// This device's LAN IP, for inviting players on the same network.
    async fn get_lan_address() -> Result<Option<String>, ApiError>;

    /// Disk space used by the server folder, by category, and space left on the device.
    async fn get_server_storage(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<ServerStorage, ApiError>;
}

#[derive(Clone)]
pub struct ServerMetricsApiImpl;

#[taurpc::resolvers]
impl ServerMetricsApi for ServerMetricsApiImpl {
    async fn get_server_stats(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<ServerStats>, ApiError> {
        let (_, config) = server_with_config(&app_handle, &server_id)?;
        if is_server_running(get_server_process_manager(&app_handle), &server_id).await {
            server::stats::ensure_sampler(
                get_server_process_manager(&app_handle).clone(),
                &server_id,
                core_has_tps(&config),
            );
        }
        Ok(server::stats::history(&server_id))
    }

    async fn get_lan_address(self) -> Result<Option<String>, ApiError> {
        Ok(server::stats::lan_address())
    }

    async fn get_server_storage(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<ServerStorage, ApiError> {
        let (dir, _) = server_with_config(&app_handle, &server_id)?;
        tauri::async_runtime::spawn_blocking(move || server::storage::storage(&dir))
            .await
            .map_err(|e| ApiError::internal(e.to_string()))
    }
}
