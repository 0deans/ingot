//! Application commands delegate lifecycle work to the native application layer.
use crate::app::lifecycle;
use crate::running::{LeftoverServer, QuitRequest};
use tauri::Runtime;

#[taurpc::procedures(path = "app")]
pub(super) trait ApplicationApi {
    async fn greet(name: String) -> String;
    async fn get_quit_blockers(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<QuitRequest, String>;
    async fn stop_all_servers(app_handle: tauri::AppHandle<impl Runtime>) -> Result<(), String>;
    async fn quit_app(app_handle: tauri::AppHandle<impl Runtime>) -> Result<(), String>;
    async fn get_leftover_servers(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<LeftoverServer>, String>;
    async fn stop_leftover_server(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), String>;
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
    ) -> Result<QuitRequest, String> {
        Ok(lifecycle::quit_request(&app_handle).await)
    }

    async fn stop_all_servers(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<(), String> {
        lifecycle::stop_all_servers(&app_handle).await;
        Ok(())
    }

    async fn quit_app(self, app_handle: tauri::AppHandle<impl Runtime>) -> Result<(), String> {
        lifecycle::quit_now(&app_handle);
        Ok(())
    }

    async fn get_leftover_servers(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<LeftoverServer>, String> {
        Ok(lifecycle::leftover_servers(&app_handle).await)
    }

    async fn stop_leftover_server(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), String> {
        lifecycle::stop_leftover_server(&app_handle, server_id)
            .await
            .map_err(|error| error.to_string())
    }
}
