use super::models::{ServerConfig, ServerCoreType, ServerProperties};
use super::ports::{check_port, free_port_for_core, internal_port_for};
use super::properties::{read_server_properties_from_dir, write_server_properties_to_dir};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Manager, Runtime};
use uuid::Uuid;

pub fn get_servers_dir<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {e}"))?;

    let servers_dir = data_dir.join("servers");
    if !servers_dir.exists() {
        fs::create_dir_all(&servers_dir)
            .map_err(|e| format!("Failed to create servers directory: {e}"))?;
    }
    Ok(servers_dir)
}

pub fn get_server_dir<R: Runtime>(
    app: &tauri::AppHandle<R>,
    server_id: &str,
) -> Result<PathBuf, String> {
    let servers_dir = get_servers_dir(app)?;
    let server_dir = servers_dir.join(server_id);
    if !server_dir.exists() {
        fs::create_dir_all(&server_dir)
            .map_err(|e| format!("Failed to create server directory: {e}"))?;
    }
    Ok(server_dir)
}

fn get_servers_file<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {e}"))?;
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create app data directory: {e}"))?;
    }
    Ok(data_dir.join("servers.json"))
}

pub fn load_servers<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<Vec<ServerConfig>, String> {
    let file = get_servers_file(app)?;
    if !file.exists() {
        return Ok(Vec::new());
    }

    let contents =
        fs::read_to_string(&file).map_err(|e| format!("Failed to read servers.json: {e}"))?;

    if contents.trim().is_empty() {
        return Ok(Vec::new());
    }

    serde_json::from_str(&contents).map_err(|e| format!("Failed to parse servers.json: {e}"))
}

pub fn save_servers<R: Runtime>(
    app: &tauri::AppHandle<R>,
    servers: &[ServerConfig],
) -> Result<(), String> {
    let file = get_servers_file(app)?;
    let json = serde_json::to_string_pretty(servers)
        .map_err(|e| format!("Failed to serialize servers: {e}"))?;
    fs::write(file, json).map_err(|e| format!("Failed to write servers.json: {e}"))
}

#[allow(clippy::too_many_arguments)]
pub fn create_server<R: Runtime>(
    app: &tauri::AppHandle<R>,
    name: String,
    core: ServerCoreType,
    game_version: String,
    build_number: Option<String>,
    port: Option<u16>,
    memory_min_mb: Option<u32>,
    memory_max_mb: Option<u32>,
) -> Result<ServerConfig, String> {
    let mut servers = load_servers(app)?;
    let id = Uuid::new_v4().to_string();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let selected_port = match port {
        Some(p) => {
            check_port(&servers, &id, p)?;
            p
        }
        None => free_port_for_core(&servers, &id, &core),
    };

    let server_dir = get_server_dir(app, &id)?;
    let _ = fs::create_dir_all(server_dir.join("plugins"));
    let _ = fs::create_dir_all(server_dir.join("mods"));
    let _ = fs::create_dir_all(server_dir.join("logs"));

    let eula_file = server_dir.join("eula.txt");
    let _ = fs::write(
        eula_file,
        "# By changing the setting below to TRUE you are indicating your agreement to our EULA (https://aka.ms/MinecraftEULA).\neula=true\n",
    );

    let props = ServerProperties {
        server_port: selected_port,
        motd: format!("§6§l{}§r §7- Ingot Minecraft Server", name),
        ..Default::default()
    };
    let _ = write_server_properties_to_dir(&server_dir, &props);

    let is_bedrock = core == ServerCoreType::Bedrock;
    let config = ServerConfig {
        id: id.clone(),
        name,
        core,
        game_version,
        build_number,
        port: selected_port,
        memory_min_mb: memory_min_mb.unwrap_or(if is_bedrock { 1024 } else { 2048 }),
        memory_max_mb: memory_max_mb.unwrap_or(if is_bedrock { 2048 } else { 4096 }),
        icon: None,
        java_path: None,
        jvm_args: None,
        auto_start: Some(false),
        sleep_enabled: Some(!is_bedrock),
        idle_timeout_seconds: Some(600),
        internal_port: Some(internal_port_for(selected_port)),
        playit_enabled: Some(false),
        playit_secret_key: None,
        created_at: now,
        last_run_at: None,
    };

    servers.push(config.clone());
    save_servers(app, &servers)?;

    Ok(config)
}

pub fn delete_server<R: Runtime>(
    app: &tauri::AppHandle<R>,
    server_id: &str,
    delete_files: bool,
) -> Result<(), String> {
    let mut servers = load_servers(app)?;
    let index = servers
        .iter()
        .position(|s| s.id == server_id)
        .ok_or_else(|| format!("Server not found: {server_id}"))?;

    servers.remove(index);
    save_servers(app, &servers)?;

    if delete_files {
        let server_dir = get_server_dir(app, server_id)?;
        if server_dir.exists() {
            let _ = fs::remove_dir_all(server_dir);
        }
    }

    Ok(())
}

pub fn update_server<R: Runtime>(
    app: &tauri::AppHandle<R>,
    mut config: ServerConfig,
) -> Result<(), String> {
    let mut servers = load_servers(app)?;
    let index = servers
        .iter()
        .position(|s| s.id == config.id)
        .ok_or_else(|| format!("Server not found: {}", config.id))?;

    if servers[index].port != config.port {
        check_port(&servers, &config.id, config.port)?;
        config.internal_port = Some(internal_port_for(config.port));
        if let Ok(server_dir) = get_server_dir(app, &config.id) {
            if let Ok(mut props) = read_server_properties_from_dir(&server_dir) {
                props.server_port = config.port;
                let _ = write_server_properties_to_dir(&server_dir, &props);
            }
        }
    }

    servers[index] = config;
    save_servers(app, &servers)
}
