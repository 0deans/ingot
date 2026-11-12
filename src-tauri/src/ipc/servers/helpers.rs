//! Shared resolution and state helpers for server IPC handlers.
use crate::app::server_config::{get_server_dir, load_servers};
use crate::ipc::ApiError;
use ingot_server::{ServerConfig, ServerProcessManager, ServerStatus, ServerSupervisorManager};
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
