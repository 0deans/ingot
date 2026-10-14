//! IPC procedures for server configuration, properties, and access control.
use super::{get_http_client, get_server_process_manager, is_server_running, resolve_profile};
use crate::ipc::ApiError;
use crate::server::files::{AccessEntry, AccessListKind, ConfigFile, PropertyEntry};
use crate::server::{self, ServerProperties, WhitelistEntry};
use tauri::Runtime;

#[taurpc::procedures(path = "servers.config")]
pub trait ServerConfigApi {
    async fn get_server_properties(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<ServerProperties, ApiError>;

    async fn set_server_properties(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        properties: ServerProperties,
    ) -> Result<(), ApiError>;

    async fn get_server_properties_all(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<PropertyEntry>, ApiError>;

    async fn set_server_properties_all(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        entries: Vec<PropertyEntry>,
    ) -> Result<(), ApiError>;

    async fn get_server_whitelist(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<WhitelistEntry>, ApiError>;

    async fn add_to_server_whitelist(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        username: String,
    ) -> Result<(), ApiError>;

    async fn remove_from_server_whitelist(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        username: String,
    ) -> Result<(), ApiError>;

    async fn get_access_list(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        kind: AccessListKind,
    ) -> Result<Vec<AccessEntry>, ApiError>;

    async fn add_access_entry(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        kind: AccessListKind,
        name: String,
    ) -> Result<Vec<AccessEntry>, ApiError>;

    async fn remove_access_entry(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        kind: AccessListKind,
        name: String,
    ) -> Result<Vec<AccessEntry>, ApiError>;

    async fn list_server_config_files(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<ConfigFile>, ApiError>;

    async fn read_server_config_file(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        path: String,
    ) -> Result<String, ApiError>;

    async fn write_server_config_file(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        path: String,
        content: String,
    ) -> Result<(), ApiError>;
}

#[derive(Clone)]
pub struct ServerConfigApiImpl;

#[taurpc::resolvers]
impl ServerConfigApi for ServerConfigApiImpl {
    async fn get_server_properties(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<ServerProperties, ApiError> {
        let dir = server::get_server_dir(&app_handle, &server_id).map_err(ApiError::from)?;
        server::read_server_properties_from_dir(&dir).map_err(ApiError::from)
    }

    async fn set_server_properties(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        properties: ServerProperties,
    ) -> Result<(), ApiError> {
        let dir = server::get_server_dir(&app_handle, &server_id).map_err(ApiError::from)?;
        server::write_server_properties_to_dir(&dir, &properties).map_err(ApiError::from)
    }

    async fn get_server_properties_all(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<PropertyEntry>, ApiError> {
        let dir = server::get_server_dir(&app_handle, &server_id).map_err(ApiError::from)?;
        let mut props = server::files::read_properties(&dir).map_err(ApiError::from)?;
        if let Ok(servers) = server::load_servers(&app_handle) {
            if let Some(cfg) = servers.iter().find(|s| s.id == server_id) {
                for p in &mut props {
                    if p.key == "server-port" {
                        p.value = cfg.port.to_string();
                    }
                }
            }
        }
        Ok(props)
    }

    async fn set_server_properties_all(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        entries: Vec<PropertyEntry>,
    ) -> Result<(), ApiError> {
        let dir = server::get_server_dir(&app_handle, &server_id).map_err(ApiError::from)?;
        let new_port = entries
            .iter()
            .find(|e| e.key == "server-port")
            .and_then(|e| e.value.trim().parse::<u16>().ok());
        let mut servers = server::load_servers(&app_handle).map_err(ApiError::from)?;
        if let Some(port) = new_port {
            if servers.iter().any(|s| s.id == server_id && s.port != port) {
                server::config::check_port(&servers, &server_id, port).map_err(ApiError::from)?;
            }
        }
        server::files::write_properties(&dir, &entries).map_err(ApiError::from)?;
        if let Some(port) = new_port {
            if let Some(config) = servers.iter_mut().find(|s| s.id == server_id) {
                if config.port != port {
                    config.port = port;
                    config.internal_port = Some(server::config::internal_port_for(port));
                    server::save_servers(&app_handle, &servers).map_err(ApiError::from)?;
                }
            }
        }
        Ok(())
    }

    async fn get_server_whitelist(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<WhitelistEntry>, ApiError> {
        let dir = server::get_server_dir(&app_handle, &server_id).map_err(ApiError::from)?;
        server::read_server_whitelist(&dir).map_err(ApiError::from)
    }

    async fn add_to_server_whitelist(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        username: String,
    ) -> Result<(), ApiError> {
        let dir = server::get_server_dir(&app_handle, &server_id).map_err(ApiError::from)?;
        server::add_to_server_whitelist(&dir, &username).map_err(ApiError::from)
    }

    async fn remove_from_server_whitelist(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        username: String,
    ) -> Result<(), ApiError> {
        let dir = server::get_server_dir(&app_handle, &server_id).map_err(ApiError::from)?;
        server::remove_from_server_whitelist(&dir, &username).map_err(ApiError::from)
    }

    async fn get_access_list(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        kind: AccessListKind,
    ) -> Result<Vec<AccessEntry>, ApiError> {
        let dir = server::get_server_dir(&app_handle, &server_id).map_err(ApiError::from)?;
        Ok(server::files::read_access_list(&dir, kind))
    }

    async fn add_access_entry(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        kind: AccessListKind,
        name: String,
    ) -> Result<Vec<AccessEntry>, ApiError> {
        let name = name.trim().to_string();
        if !server::live::is_valid_player_name(&name) {
            return Err(ApiError::invalid_input(format!("\"{name}\" is not a valid Minecraft username")));
        }
        let dir = server::get_server_dir(&app_handle, &server_id).map_err(ApiError::from)?;
        let running = is_server_running(get_server_process_manager(&app_handle), &server_id).await;
        if kind == AccessListKind::Whitelist || !running {
            let (name, uuid) = resolve_profile(get_http_client(&app_handle), &dir, &name).await;
            server::files::add_access_entry(&dir, kind, &name, &uuid).map_err(ApiError::from)?;
            if running {
                let _ = get_server_process_manager(&app_handle)
                    .send_command(&server_id, "whitelist reload")
                    .await;
            }
        } else {
            get_server_process_manager(&app_handle)
                .send_command(&server_id, &kind.add_command(&name))
                .await
                .map_err(ApiError::from)?;
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        Ok(server::files::read_access_list(&dir, kind))
    }

    async fn remove_access_entry(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        kind: AccessListKind,
        name: String,
    ) -> Result<Vec<AccessEntry>, ApiError> {
        let dir = server::get_server_dir(&app_handle, &server_id).map_err(ApiError::from)?;
        if is_server_running(get_server_process_manager(&app_handle), &server_id).await {
            if !server::live::is_valid_player_name(&name) {
                return Err(ApiError::invalid_input(format!("\"{name}\" is not a valid Minecraft username")));
            }
            get_server_process_manager(&app_handle)
                .send_command(&server_id, &kind.remove_command(&name))
                .await
                .map_err(ApiError::from)?;
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        } else {
            server::files::remove_access_entry(&dir, kind, &name).map_err(ApiError::from)?;
        }
        Ok(server::files::read_access_list(&dir, kind))
    }

    async fn list_server_config_files(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<ConfigFile>, ApiError> {
        let dir = server::get_server_dir(&app_handle, &server_id).map_err(ApiError::from)?;
        Ok(server::files::list_config_files(&dir))
    }

    async fn read_server_config_file(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        path: String,
    ) -> Result<String, ApiError> {
        let dir = server::get_server_dir(&app_handle, &server_id).map_err(ApiError::from)?;
        server::files::read_config_file(&dir, &path).map_err(ApiError::from)
    }

    async fn write_server_config_file(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        path: String,
        content: String,
    ) -> Result<(), ApiError> {
        let dir = server::get_server_dir(&app_handle, &server_id).map_err(ApiError::from)?;
        server::files::write_config_file(&dir, &path, &content).map_err(ApiError::from)
    }
}
