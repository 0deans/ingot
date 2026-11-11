//! IPC procedures for Minecraft server lifecycle and management.
pub(crate) mod config;
pub(crate) mod helpers;
pub(crate) mod map;
pub(crate) mod metrics;
pub(crate) mod packs;
pub(crate) mod players;
pub(crate) mod plugins;
pub(crate) mod transfer;
pub(crate) mod tunnel;
pub(crate) mod version_change;

#[cfg(test)]
mod tests;

pub(crate) use helpers::*;

use crate::app::server_config::{
    create_server, delete_server, get_server_dir, load_servers, update_server,
};
use crate::ipc::events::TauRpcEventsApiEventTrigger;
use crate::ipc::ApiError;
use ingot_server::{RunningServerSummary, ServerConfig, ServerCoreType, ServerPingResponse};
use tauri::Runtime;

#[taurpc::procedures(path = "servers")]
#[allow(clippy::too_many_arguments)]
pub trait ServersApi {
    async fn get_servers(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<ServerConfig>, ApiError>;

    async fn create_server(
        app_handle: tauri::AppHandle<impl Runtime>,
        name: String,
        core: ServerCoreType,
        game_version: String,
        build_number: Option<String>,
        port: Option<u16>,
        memory_min_mb: Option<u32>,
        memory_max_mb: Option<u32>,
    ) -> Result<ServerConfig, ApiError>;

    async fn delete_server(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        delete_files: bool,
    ) -> Result<(), ApiError>;

    async fn update_server(
        app_handle: tauri::AppHandle<impl Runtime>,
        server: ServerConfig,
    ) -> Result<(), ApiError>;

    async fn start_server(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<u32, ApiError>;

    async fn put_server_to_sleep(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), ApiError>;

    async fn stop_server(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), ApiError>;

    async fn kill_server(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), ApiError>;

    async fn get_running_servers(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<RunningServerSummary>, ApiError>;

    async fn send_server_command(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        command: String,
    ) -> Result<(), ApiError>;

    async fn get_server_logs(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<String>, ApiError>;

    async fn open_server_folder(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), ApiError>;

    async fn get_server_icon(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Option<String>, ApiError>;

    async fn set_server_icon(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        base64_data: String,
    ) -> Result<(), ApiError>;

    async fn save_server_world(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        flush: bool,
    ) -> Result<(), ApiError>;

    async fn ping_server(port: u16) -> Result<ServerPingResponse, ApiError>;

    async fn install_companion(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), ApiError>;

    async fn get_companion_status(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<ingot_server::companion::CompanionStatus, ApiError>;
}

#[derive(Clone)]
pub struct ServersApiImpl;

#[taurpc::resolvers]
impl ServersApi for ServersApiImpl {
    async fn get_servers(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<ServerConfig>, ApiError> {
        load_servers(&app_handle).map_err(ApiError::from)
    }

    async fn create_server(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        name: String,
        core: ServerCoreType,
        game_version: String,
        build_number: Option<String>,
        port: Option<u16>,
        memory_min_mb: Option<u32>,
        memory_max_mb: Option<u32>,
    ) -> Result<ServerConfig, ApiError> {
        let created = create_server(
            &app_handle,
            name,
            core,
            game_version,
            build_number,
            port,
            memory_min_mb,
            memory_max_mb,
        )?;
        if ingot_server::companion::supported(&created.core, &created.game_version) {
            if let Ok((dir, _)) = server_with_config(&app_handle, &created.id) {
                if let Err(e) =
                    ingot_server::companion::install(&dir, &created.core, &created.game_version)
                {
                    eprintln!("[Server] Couldn't install the Ingot plugin: {e}");
                }
            }
        }
        Ok(created)
    }

    async fn delete_server(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        delete_files: bool,
    ) -> Result<(), ApiError> {
        let ownership = get_server_process_manager(&app_handle)
            .reserve_maintenance(&server_id)
            .await?;
        tokio::task::spawn_blocking(move || {
            let _ownership = ownership;
            delete_server(&app_handle, &server_id, delete_files)
        })
        .await
        .map_err(ApiError::from_display)?
        .map_err(ApiError::from)
    }

    async fn update_server(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server: ServerConfig,
    ) -> Result<(), ApiError> {
        let ownership = get_server_process_manager(&app_handle)
            .reserve_maintenance(&server.id)
            .await?;
        tokio::task::spawn_blocking(move || {
            let _ownership = ownership;
            update_server(&app_handle, server)
        })
        .await
        .map_err(ApiError::from_display)?
        .map_err(ApiError::from)
    }

    async fn start_server(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<u32, ApiError> {
        use tauri::Manager;

        let servers = load_servers(&app_handle)?;
        let config = servers
            .into_iter()
            .find(|s| s.id == server_id)
            .ok_or_else(|| ApiError::not_found(format!("Server not found: {}", server_id)))?;

        let (dir, _) = server_with_config(&app_handle, &server_id)?;
        validate_pre_launch(&app_handle, &config, &dir).await?;

        let data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| ApiError::storage_failure(e.to_string()))?;

        let pm = get_server_process_manager(&app_handle).clone();
        let client = get_http_client(&app_handle).clone();

        let claimed = ingot_server::process::claim_server_launch(
            &crate::app::state(&app_handle).server_configs,
            &pm,
            &server_id,
        )
        .await
        .map_err(ApiError::from)?;

        let hooks = std::sync::Arc::new(crate::app::TauriServerLifecycleHooks::new(
            app_handle.clone(),
        ));

        let app_log = app_handle.clone();
        let on_log = move |ev: ingot_server::ServerLogEvent| {
            let trigger = TauRpcEventsApiEventTrigger::new(app_log.clone());
            if let Err(e) = trigger.on_server_log(ev) {
                eprintln!("[IPC] Failed to emit on_server_log: {e}");
            }
        };

        let app_status = app_handle.clone();
        let on_status = move |ev: ingot_server::ServerStatusEvent| {
            let trigger = TauRpcEventsApiEventTrigger::new(app_status.clone());
            if let Err(e) = trigger.on_server_status_changed(ev) {
                eprintln!("[IPC] Failed to emit on_server_status_changed: {e}");
            }
        };

        let has_tps = core_has_tps(&config);
        let pid = get_server_supervisor_manager(&app_handle)
            .supervise_and_start(
                ingot_server::process::ServerLaunchContext {
                    repository: crate::app::state(&app_handle).server_configs.clone(),
                    server_dir: dir,
                    data_dir,
                    processes: pm,
                    client,
                    hooks,
                    command_builder: std::sync::Arc::new(
                        crate::app::server_lifecycle::DefaultServerCommandBuilder::new(
                            crate::app::state(&app_handle).sandbox_setup.clone(),
                        ),
                    ),
                    on_log: std::sync::Arc::new(on_log),
                    on_status: std::sync::Arc::new(on_status),
                    publication: Default::default(),
                },
                claimed,
            )
            .await
            .map_err(ApiError::from)?;
        ingot_server::stats::ensure_sampler(
            get_server_process_manager(&app_handle).clone(),
            &server_id,
            has_tps,
        );
        Ok(pid)
    }

    async fn put_server_to_sleep(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), ApiError> {
        let app_status = app_handle.clone();
        let on_status = move |ev: ingot_server::ServerStatusEvent| {
            let trigger = TauRpcEventsApiEventTrigger::new(app_status.clone());
            if let Err(e) = trigger.on_server_status_changed(ev) {
                eprintln!("[IPC] Failed to emit on_server_status_changed: {e}");
            }
        };
        get_server_supervisor_manager(&app_handle)
            .put_to_sleep(
                get_server_process_manager(&app_handle),
                &server_id,
                on_status,
            )
            .await
            .map_err(ApiError::from)
    }

    async fn stop_server(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), ApiError> {
        get_server_supervisor_manager(&app_handle)
            .stop_supervised(get_server_process_manager(&app_handle), &server_id)
            .await
            .map_err(ApiError::from)
    }

    async fn kill_server(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), ApiError> {
        get_server_process_manager(&app_handle)
            .kill_server_typed(&server_id)
            .await
            .map_err(ApiError::from)
    }

    async fn get_running_servers(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<RunningServerSummary>, ApiError> {
        Ok(get_server_process_manager(&app_handle)
            .get_running_servers()
            .await)
    }

    async fn send_server_command(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        command: String,
    ) -> Result<(), ApiError> {
        get_server_process_manager(&app_handle)
            .send_command_typed(&server_id, &command)
            .await
            .map_err(ApiError::from)
    }

    async fn get_server_logs(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<String>, ApiError> {
        Ok(get_server_process_manager(&app_handle)
            .get_server_logs(&server_id)
            .await)
    }

    async fn open_server_folder(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), ApiError> {
        let dir = get_server_dir(&app_handle, &server_id)?;
        crate::app::platform::open_directory(&dir).map_err(ApiError::from)
    }

    async fn get_server_icon(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Option<String>, ApiError> {
        let dir = get_server_dir(&app_handle, &server_id)?;
        Ok(ingot_server::get_server_icon_base64(&dir))
    }

    async fn set_server_icon(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        base64_data: String,
    ) -> Result<(), ApiError> {
        let dir = get_server_dir(&app_handle, &server_id)?;
        ingot_server::save_server_icon(&dir, &base64_data).map_err(ApiError::from)
    }

    async fn save_server_world(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        flush: bool,
    ) -> Result<(), ApiError> {
        get_server_process_manager(&app_handle)
            .query_typed(
                &server_id,
                if flush { "save-all flush" } else { "save-all" },
                |line| line.contains("Saved the game"),
                std::time::Duration::from_secs(30),
            )
            .await
            .map(|_| ())
            .map_err(ApiError::from)
    }

    async fn ping_server(self, port: u16) -> Result<ServerPingResponse, ApiError> {
        ingot_server::ping_server("127.0.0.1", port)
            .await
            .map_err(ApiError::from)
    }

    async fn install_companion(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), ApiError> {
        let (dir, config) = server_with_config(&app_handle, &server_id)?;
        ingot_server::companion::install(&dir, &config.core, &config.game_version)
            .map_err(ApiError::from)
    }

    async fn get_companion_status(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<ingot_server::companion::CompanionStatus, ApiError> {
        let (dir, config) = server_with_config(&app_handle, &server_id)?;
        Ok(ingot_server::companion::status(
            &dir,
            &config.core,
            &config.game_version,
        ))
    }
}
