//! Shared resolution and state helpers for server IPC handlers.
use crate::app::server_config::{get_server_dir, load_servers};
use crate::ipc::ApiError;
use ingot_server::{
    ServerConfig, ServerCoreType, ServerProcessManager, ServerStatus, ServerSupervisorManager,
};
use tauri::Runtime;

pub(crate) fn get_server_process_manager<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> &ServerProcessManager {
    &crate::app::state(app).servers
}

pub(crate) fn get_server_supervisor_manager<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> &ServerSupervisorManager {
    &crate::app::state(app).supervisor
}

pub(crate) fn get_http_client<R: Runtime>(app: &tauri::AppHandle<R>) -> &reqwest::Client {
    &crate::app::state(app).http
}

pub(crate) fn core_has_tps(config: &ServerConfig) -> bool {
    matches!(
        config.core,
        ServerCoreType::Paper | ServerCoreType::Purpur | ServerCoreType::Folia
    )
}

pub(crate) async fn is_server_running(pm: &ServerProcessManager, server_id: &str) -> bool {
    pm.get_server_status(server_id).await == ServerStatus::Running
}

pub(crate) fn server_with_config<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
    server_id: &str,
) -> Result<(std::path::PathBuf, ServerConfig), ApiError> {
    let config = load_servers(app_handle)
        .map_err(ApiError::from)?
        .into_iter()
        .find(|s| s.id == server_id)
        .ok_or_else(|| ApiError::not_found(format!("Server not found: {server_id}")))?;
    Ok((
        get_server_dir(app_handle, server_id).map_err(ApiError::from)?,
        config,
    ))
}

pub(crate) async fn resolve_profile(
    client: &reqwest::Client,
    server_dir: &std::path::Path,
    name: &str,
) -> (String, String) {
    let online_mode = ingot_server::read_server_properties_from_dir(server_dir)
        .map(|p| p.online_mode)
        .unwrap_or(false);
    if online_mode {
        #[derive(serde::Deserialize)]
        struct Profile {
            id: String,
            name: String,
        }
        let url = format!("https://api.mojang.com/users/profiles/minecraft/{name}");
        if let Ok(res) = client.get(url).send().await {
            if let Ok(profile) = res.json::<Profile>().await {
                if profile.id.len() == 32 {
                    let id = &profile.id;
                    let uuid = format!(
                        "{}-{}-{}-{}-{}",
                        &id[0..8],
                        &id[8..12],
                        &id[12..16],
                        &id[16..20],
                        &id[20..32]
                    );
                    return (profile.name, uuid);
                }
            }
        }
    }
    (
        name.to_string(),
        ingot_server::config::offline_uuid_for(name),
    )
}

pub(crate) async fn validate_pre_launch<R: Runtime>(
    app: &tauri::AppHandle<R>,
    config: &ServerConfig,
    dir: &std::path::Path,
) -> Result<(), ApiError> {
    match ingot_version_change::detect_incomplete_transaction(dir) {
        Ok(Some(incomplete)) => {
            return Err(ApiError::incomplete_recovery(format!(
                "Cannot start server: version change transaction is in incomplete state ({:?}). Please recover or revert it first.",
                incomplete.phase
            )));
        }
        Err(err) => {
            return Err(ApiError::incomplete_recovery(format!(
                "Cannot start server: {err}. Please recover or inspect the server backup first."
            )));
        }
        Ok(None) => {}
    }

    let leftovers = crate::app::lifecycle::leftover_servers(app).await;
    if leftovers.iter().any(|l| l.server_id == config.id) {
        if let Err(e) = crate::ipc::events::TauRpcEventsApiEventTrigger::new(app.clone())
            .on_leftover_servers(leftovers)
        {
            eprintln!("[IPC] Failed to emit on_leftover_servers: {e}");
        }
        return Err(ApiError::conflict(format!(
            "\"{}\" is still running from before Ingot closed. Stop it first, then start it again.",
            config.name
        )));
    }

    let pm = get_server_process_manager(app);
    for other in load_servers(app)?.iter().filter(|s| s.id != config.id) {
        let other_ports = ingot_server::config::ports_of(other);
        let Some(port) = ingot_server::config::ports_of(config)
            .into_iter()
            .find(|p| other_ports.contains(p))
        else {
            continue;
        };
        if pm.get_server_status(&other.id).await != ServerStatus::Stopped {
            return Err(ApiError::conflict(format!(
                "Port {port} is used by \"{}\", which is running. Stop it, or give this server another port in Settings.",
                other.name
            )));
        }
    }
    if matches!(
        pm.get_server_status(&config.id).await,
        ServerStatus::Stopped | ServerStatus::Sleeping
    ) {
        get_server_supervisor_manager(app).release(&config.id).await;
    }
    let port_free = || std::net::TcpListener::bind(("0.0.0.0", config.port)).is_ok();
    let mut free = port_free();
    for _ in 0..10 {
        if free {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        free = port_free();
    }
    if !free {
        return Err(ApiError::conflict(format!(
            "Port {} is already in use by another app. Close it, or give this server another port in Settings.",
            config.port
        )));
    }
    Ok(())
}
