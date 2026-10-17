//! IPC procedures for server plugin management.
use super::server_with_config;
use crate::ipc::ApiError;
use crate::server;
use crate::server::plugins::{
    InstallReport, InstalledPlugin, PluginSearchResult, PluginSource, PluginUpdate, PluginVersion,
};
use tauri::Runtime;

#[taurpc::procedures(path = "servers.plugins")]
#[allow(clippy::too_many_arguments)]
pub trait ServerPluginsApi {
    /// Search plugins or server-side mods for this server.
    async fn search_server_plugins(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        source: PluginSource,
        query: String,
        sort: String,
        compatible_only: bool,
        page: u32,
    ) -> Result<PluginSearchResult, ApiError>;

    /// Project description as Markdown.
    async fn get_server_plugin_page(
        source: PluginSource,
        project_id: String,
    ) -> Result<String, ApiError>;

    async fn get_server_plugin_versions(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        source: PluginSource,
        project_id: String,
        compatible_only: bool,
    ) -> Result<Vec<PluginVersion>, ApiError>;

    /// Installs a plugin together with its required dependencies.
    async fn install_server_plugin(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        source: PluginSource,
        project_id: String,
        version_id: Option<String>,
    ) -> Result<InstallReport, ApiError>;

    async fn list_server_plugins(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<InstalledPlugin>, ApiError>;

    async fn set_server_plugin_enabled(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        file_name: String,
        enabled: bool,
    ) -> Result<(), ApiError>;

    async fn remove_server_plugin(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        file_name: String,
    ) -> Result<(), ApiError>;

    async fn check_server_plugin_updates(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<PluginUpdate>, ApiError>;
}

#[derive(Clone)]
pub struct ServerPluginsApiImpl;

#[taurpc::resolvers]
impl ServerPluginsApi for ServerPluginsApiImpl {
    async fn search_server_plugins(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        source: PluginSource,
        query: String,
        sort: String,
        compatible_only: bool,
        page: u32,
    ) -> Result<PluginSearchResult, ApiError> {
        let (_, config) = server_with_config(&app_handle, &server_id)?;
        server::plugins::search(
            &config.core,
            &config.game_version,
            source,
            &query,
            &sort,
            compatible_only,
            page,
            20,
        )
        .await
        .map_err(ApiError::from)
    }

    async fn get_server_plugin_page(
        self,
        source: PluginSource,
        project_id: String,
    ) -> Result<String, ApiError> {
        server::plugins::page(source, &project_id)
            .await
            .map_err(ApiError::from)
    }

    async fn get_server_plugin_versions(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        source: PluginSource,
        project_id: String,
        compatible_only: bool,
    ) -> Result<Vec<PluginVersion>, ApiError> {
        let (_, config) = server_with_config(&app_handle, &server_id)?;
        server::plugins::versions(
            &config.core,
            &config.game_version,
            source,
            &project_id,
            compatible_only,
        )
        .await
        .map_err(ApiError::from)
    }

    async fn install_server_plugin(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        source: PluginSource,
        project_id: String,
        version_id: Option<String>,
    ) -> Result<InstallReport, ApiError> {
        let _operation = crate::app::state(&app_handle)
            .servers
            .reserve_maintenance(&server_id)
            .await
            .map_err(ApiError::from)?;
        let (dir, config) = server_with_config(&app_handle, &server_id)?;
        server::plugins::install(
            &dir,
            &config.core,
            &config.game_version,
            source,
            &project_id,
            version_id.as_deref(),
        )
        .await
        .map_err(ApiError::from)
    }

    async fn list_server_plugins(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<InstalledPlugin>, ApiError> {
        let (dir, config) = server_with_config(&app_handle, &server_id)?;
        tauri::async_runtime::spawn_blocking(move || {
            server::plugins::list_installed(&dir, &config.core)
        })
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .map_err(ApiError::from)
    }

    async fn set_server_plugin_enabled(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        file_name: String,
        enabled: bool,
    ) -> Result<(), ApiError> {
        let _operation = crate::app::state(&app_handle)
            .servers
            .reserve_maintenance(&server_id)
            .await
            .map_err(ApiError::from)?;
        let (dir, config) = server_with_config(&app_handle, &server_id)?;
        server::plugins::set_enabled(&dir, &config.core, &file_name, enabled)
            .map_err(ApiError::from)
    }

    async fn remove_server_plugin(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        file_name: String,
    ) -> Result<(), ApiError> {
        let _operation = crate::app::state(&app_handle)
            .servers
            .reserve_maintenance(&server_id)
            .await
            .map_err(ApiError::from)?;
        let (dir, config) = server_with_config(&app_handle, &server_id)?;
        server::plugins::remove(&dir, &config.core, &file_name).map_err(ApiError::from)
    }

    async fn check_server_plugin_updates(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<PluginUpdate>, ApiError> {
        let (dir, config) = server_with_config(&app_handle, &server_id)?;
        server::plugins::check_updates(&dir, &config.core, &config.game_version)
            .await
            .map_err(ApiError::from)
    }
}
