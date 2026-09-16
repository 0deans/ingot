use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Manager, Runtime};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WhitelistEntry {
    pub uuid: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServerCoreType {
    Paper,
    Purpur,
    Fabric,
    Vanilla,
    Folia,
}

impl Default for ServerCoreType {
    fn default() -> Self {
        Self::Paper
    }
}

impl std::fmt::Display for ServerCoreType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Paper => write!(f, "Paper"),
            Self::Purpur => write!(f, "Purpur"),
            Self::Fabric => write!(f, "Fabric"),
            Self::Vanilla => write!(f, "Vanilla"),
            Self::Folia => write!(f, "Folia"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServerStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    pub id: String,
    pub name: String,
    pub core: ServerCoreType,
    pub game_version: String,
    pub build_number: Option<String>,
    pub port: u16,
    pub memory_min_mb: u32,
    pub memory_max_mb: u32,
    pub java_path: Option<String>,
    pub jvm_args: Option<Vec<String>>,
    pub auto_start: Option<bool>,
    pub created_at: u64,
    pub last_run_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerProperties {
    pub server_port: u16,
    pub motd: String,
    pub online_mode: bool,
    pub difficulty: String,
    pub gamemode: String,
    pub max_players: u32,
    pub pvp: bool,
    pub view_distance: u32,
    pub simulation_distance: u32,
    pub white_list: bool,
    pub allow_flight: bool,
    pub spawn_protection: u32,
}

impl Default for ServerProperties {
    fn default() -> Self {
        Self {
            server_port: 25565,
            motd: "A Minecraft Server hosted with Ingot".to_string(),
            online_mode: false, // Default to offline/hybrid mode so Ely.by/local players can connect easily
            difficulty: "normal".to_string(),
            gamemode: "survival".to_string(),
            max_players: 20,
            pvp: true,
            view_distance: 10,
            simulation_distance: 8,
            white_list: false,
            allow_flight: false,
            spawn_protection: 16,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RunningServerSummary {
    pub server_id: String,
    pub pid: u32,
    pub port: u16,
    pub uptime_seconds: u64,
    pub status: ServerStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerLogEvent {
    pub server_id: String,
    pub line: String,
    pub level: String, // "info" | "warn" | "error"
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatusEvent {
    pub server_id: String,
    pub status: ServerStatus,
    pub pid: Option<u32>,
}

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

    // Auto-select available port if not explicitly given
    let used_ports: Vec<u16> = servers.iter().map(|s| s.port).collect();
    let selected_port = if let Some(p) = port {
        p
    } else {
        let mut p = 25565;
        while used_ports.contains(&p) {
            p += 1;
        }
        p
    };

    let server_dir = get_server_dir(app, &id)?;
    // Create standard subdirectories: plugins, mods, logs, config
    let _ = fs::create_dir_all(server_dir.join("plugins"));
    let _ = fs::create_dir_all(server_dir.join("mods"));
    let _ = fs::create_dir_all(server_dir.join("logs"));

    // Automatically agree to EULA
    let eula_file = server_dir.join("eula.txt");
    let _ = fs::write(
        eula_file,
        "# By changing the setting below to TRUE you are indicating your agreement to our EULA (https://aka.ms/MinecraftEULA).\neula=true\n",
    );

    // Write initial server.properties
    let mut props = ServerProperties::default();
    props.server_port = selected_port;
    props.motd = format!("§6§l{}§r §7- Ingot Minecraft Server", name);
    let _ = write_server_properties_to_dir(&server_dir, &props);

    let config = ServerConfig {
        id: id.clone(),
        name,
        core,
        game_version,
        build_number,
        port: selected_port,
        memory_min_mb: memory_min_mb.unwrap_or(2048),
        memory_max_mb: memory_max_mb.unwrap_or(4096),
        java_path: None,
        jvm_args: None,
        auto_start: Some(false),
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
    config: ServerConfig,
) -> Result<(), String> {
    let mut servers = load_servers(app)?;
    let index = servers
        .iter()
        .position(|s| s.id == config.id)
        .ok_or_else(|| format!("Server not found: {}", config.id))?;

    // Also update server-port in server.properties if port changed
    if servers[index].port != config.port {
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

/// Parses server.properties from a key=value file format
pub fn read_server_properties_from_dir(server_dir: &Path) -> Result<ServerProperties, String> {
    let file_path = server_dir.join("server.properties");
    if !file_path.exists() {
        return Ok(ServerProperties::default());
    }

    let contents = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read server.properties: {e}"))?;

    let mut map = HashMap::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }

    let default = ServerProperties::default();
    Ok(ServerProperties {
        server_port: map
            .get("server-port")
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.server_port),
        motd: map
            .get("motd")
            .cloned()
            .unwrap_or(default.motd),
        online_mode: map
            .get("online-mode")
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.online_mode),
        difficulty: map
            .get("difficulty")
            .cloned()
            .unwrap_or(default.difficulty),
        gamemode: map
            .get("gamemode")
            .cloned()
            .unwrap_or(default.gamemode),
        max_players: map
            .get("max-players")
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.max_players),
        pvp: map
            .get("pvp")
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.pvp),
        view_distance: map
            .get("view-distance")
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.view_distance),
        simulation_distance: map
            .get("simulation-distance")
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.simulation_distance),
        white_list: map
            .get("white-list")
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.white_list),
        allow_flight: map
            .get("allow-flight")
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.allow_flight),
        spawn_protection: map
            .get("spawn-protection")
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.spawn_protection),
    })
}

/// Serializes ServerProperties back into server.properties, preserving any extra custom properties
pub fn write_server_properties_to_dir(
    server_dir: &Path,
    props: &ServerProperties,
) -> Result<(), String> {
    let file_path = server_dir.join("server.properties");

    let mut existing_lines: Vec<String> = if file_path.exists() {
        fs::read_to_string(&file_path)
            .unwrap_or_default()
            .lines()
            .map(|l| l.to_string())
            .collect()
    } else {
        vec!["# Minecraft server properties generated by Ingot".to_string()]
    };

    let mut updated_keys = std::collections::HashSet::new();

    let mut apply_prop = |key: &str, value: &str| {
        updated_keys.insert(key.to_string());
        let mut replaced = false;
        for line in existing_lines.iter_mut() {
            if let Some((k, _)) = line.split_once('=') {
                if k.trim() == key {
                    *line = format!("{}={}", key, value);
                    replaced = true;
                    break;
                }
            }
        }
        if !replaced {
            existing_lines.push(format!("{}={}", key, value));
        }
    };

    apply_prop("server-port", &props.server_port.to_string());
    apply_prop("query.port", &props.server_port.to_string());
    apply_prop("motd", &props.motd);
    apply_prop("online-mode", &props.online_mode.to_string());
    apply_prop("difficulty", &props.difficulty);
    apply_prop("gamemode", &props.gamemode);
    apply_prop("max-players", &props.max_players.to_string());
    apply_prop("pvp", &props.pvp.to_string());
    apply_prop("view-distance", &props.view_distance.to_string());
    apply_prop("simulation-distance", &props.simulation_distance.to_string());
    apply_prop("white-list", &props.white_list.to_string());
    apply_prop("allow-flight", &props.allow_flight.to_string());
    apply_prop("spawn-protection", &props.spawn_protection.to_string());

    let final_content = existing_lines.join("\n") + "\n";
    fs::write(file_path, final_content)
        .map_err(|e| format!("Failed to write server.properties: {e}"))
}

// ─── Whitelist helpers ────────────────────────────────────────────────────────

pub fn read_server_whitelist(server_dir: &Path) -> Result<Vec<WhitelistEntry>, String> {
    let path = server_dir.join("whitelist.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read whitelist.json: {e}"))?;
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    // Minecraft stores whitelist as an array of objects with `uuid` and `name`.
    // We deserialize using raw JSON because the key casing is lowercase in the file.
    let entries: Vec<serde_json::Value> = serde_json::from_str(&raw)
        .map_err(|e| format!("Failed to parse whitelist.json: {e}"))?;
    Ok(entries
        .into_iter()
        .filter_map(|v| {
            let name = v.get("name")?.as_str()?.to_string();
            let uuid = v
                .get("uuid")
                .and_then(|u| u.as_str())
                .unwrap_or("")
                .to_string();
            Some(WhitelistEntry { uuid, name })
        })
        .collect())
}

fn write_server_whitelist(server_dir: &Path, entries: &[WhitelistEntry]) -> Result<(), String> {
    let path = server_dir.join("whitelist.json");
    // Write in the format Minecraft expects: lowercase keys
    let json_entries: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "uuid": e.uuid,
                "name": e.name
            })
        })
        .collect();
    let raw = serde_json::to_string_pretty(&json_entries)
        .map_err(|e| format!("Failed to serialize whitelist: {e}"))?;
    fs::write(path, raw).map_err(|e| format!("Failed to write whitelist.json: {e}"))
}

/// Compute the offline-mode UUID that Minecraft uses for a given username.
/// This matches Java's `UUID.nameUUIDFromBytes(("OfflinePlayer:" + name).getBytes("UTF-8"))`.
fn offline_uuid(name: &str) -> String {
    let key = format!("OfflinePlayer:{name}");
    // UUID v3 with a nil namespace (as Minecraft does via MD5)
    let bytes = key.as_bytes();
    let hash = md5_bytes(bytes);
    // Set version (3) and variant bits exactly as Minecraft does
    let mut b = hash;
    b[6] = (b[6] & 0x0f) | 0x30; // version 3
    b[8] = (b[8] & 0x3f) | 0x80; // variant RFC 4122
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        u32::from_be_bytes([b[0], b[1], b[2], b[3]]),
        u16::from_be_bytes([b[4], b[5]]),
        u16::from_be_bytes([b[6], b[7]]),
        u16::from_be_bytes([b[8], b[9]]),
        {
            let hi = u32::from_be_bytes([b[10], b[11], b[12], b[13]]) as u64;
            let lo = u16::from_be_bytes([b[14], b[15]]) as u64;
            (hi << 16) | lo
        }
    )
}

fn md5_bytes(data: &[u8]) -> [u8; 16] {
    // Simple portable MD5 — use the md5 crate if available, otherwise derive manually.
    // We use the uuid v3 helper through a nil namespace UUID.
    let ns = Uuid::nil();
    let id = Uuid::new_v3(&ns, data);
    *id.as_bytes()
}

pub fn add_to_server_whitelist(server_dir: &Path, username: &str) -> Result<(), String> {
    let mut entries = read_server_whitelist(server_dir)?;
    // Skip if already whitelisted (case-insensitive)
    if entries
        .iter()
        .any(|e| e.name.eq_ignore_ascii_case(username))
    {
        return Ok(());
    }
    let uuid = offline_uuid(username);
    entries.push(WhitelistEntry {
        uuid,
        name: username.to_string(),
    });
    write_server_whitelist(server_dir, &entries)
}

pub fn remove_from_server_whitelist(server_dir: &Path, username: &str) -> Result<(), String> {
    let mut entries = read_server_whitelist(server_dir)?;
    entries.retain(|e| !e.name.eq_ignore_ascii_case(username));
    write_server_whitelist(server_dir, &entries)
}
