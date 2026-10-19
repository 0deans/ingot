use crate::app::lifecycle;
use crate::ipc::ApiError;
use crate::running::{LeftoverServer, QuitRequest};
use tauri::Runtime;

#[taurpc::procedures(path = "app")]
pub(super) trait ApplicationApi {
    async fn greet(name: String) -> String;
    async fn get_quit_blockers(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<QuitRequest, ApiError>;
    async fn stop_all_servers(app_handle: tauri::AppHandle<impl Runtime>) -> Result<(), ApiError>;
    async fn quit_app(app_handle: tauri::AppHandle<impl Runtime>) -> Result<(), ApiError>;
    async fn get_leftover_servers(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<LeftoverServer>, ApiError>;
    async fn stop_leftover_server(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), ApiError>;
    /// Turns off a mod or plugin blamed for crashing after a version change.
    async fn turn_off_after_version_change(
        app_handle: tauri::AppHandle<impl Runtime>,
        target_kind: crate::version_change::TargetKind,
        target_id: String,
        folder: String,
        file_name: String,
    ) -> Result<(), ApiError>;
}

#[derive(Clone)]
pub(super) struct ApplicationApiImpl;

#[taurpc::resolvers]
impl ApplicationApi for ApplicationApiImpl {
    async fn greet(self, name: String) -> String {
        format!("Hello, {}! You've been greeted from Rust via TauRPC!", name)
    }

    async fn get_quit_blockers(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<QuitRequest, ApiError> {
        Ok(lifecycle::quit_request(&app_handle).await)
    }

    async fn stop_all_servers(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<(), ApiError> {
        lifecycle::stop_all_servers(&app_handle).await;
        Ok(())
    }

    async fn quit_app(self, app_handle: tauri::AppHandle<impl Runtime>) -> Result<(), ApiError> {
        lifecycle::quit_now(&app_handle);
        Ok(())
    }

    async fn get_leftover_servers(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<LeftoverServer>, ApiError> {
        Ok(lifecycle::leftover_servers(&app_handle).await)
    }

    async fn stop_leftover_server(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), ApiError> {
        lifecycle::stop_leftover_server(&app_handle, server_id)
            .await
            .map_err(ApiError::from)
    }

    async fn turn_off_after_version_change(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        target_kind: crate::version_change::TargetKind,
        target_id: String,
        folder: String,
        file_name: String,
    ) -> Result<(), ApiError> {
        let root = match target_kind {
            crate::version_change::TargetKind::Instance => {
                if crate::app::state(&app_handle)
                    .games
                    .is_running(&target_id)
                    .await
                {
                    return Err(ApiError::conflict("Close the game first"));
                }
                crate::app::instances::ensure_instance_dir(&app_handle, &target_id)?
            }
            crate::version_change::TargetKind::Server => {
                if crate::app::state(&app_handle)
                    .servers
                    .get_server_status(&target_id)
                    .await
                    != crate::server::ServerStatus::Stopped
                {
                    return Err(ApiError::conflict("Stop the server first"));
                }
                crate::server::get_server_dir(&app_handle, &target_id)?
            }
        };
        crate::version_change::crash::disable(&root, &folder, &file_name).map_err(ApiError::from)
    }
}
