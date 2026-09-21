use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Manager, Runtime};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModLoaderType {
    Vanilla,
    Fabric,
    Quilt,
    NeoForge,
    Forge,
}

impl Default for ModLoaderType {
    fn default() -> Self {
        Self::Vanilla
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstanceConfig {
    pub id: String,
    pub name: String,
    pub game_version: String,
    pub loader: ModLoaderType,
    pub loader_version: Option<String>,
    pub java_path: Option<String>,
    pub memory_min_mb: Option<u32>,
    pub memory_max_mb: Option<u32>,
    pub icon: Option<String>,
    pub created_at: u64,
    pub last_played: Option<u64>,
    pub total_play_time_seconds: u64,
    pub jvm_args: Option<Vec<String>>,
    #[serde(default)]
    pub fullscreen: Option<bool>,
    #[serde(default)]
    pub window_width: Option<u32>,
    #[serde(default)]
    pub window_height: Option<u32>,
    #[serde(default)]
    pub sync_options: Option<bool>,
    #[serde(default)]
    pub sync_servers: Option<bool>,
    #[serde(default)]
    pub sync_resource_packs: Option<bool>,
    #[serde(default)]
    pub sync_command_history: Option<bool>,
    #[serde(default)]
    pub sync_creative_hotbars: Option<bool>,
    #[serde(default)]
    pub last_synced_at: Option<u64>,
}

pub fn get_instances_dir<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {e}"))?;

    let instances_dir = data_dir.join("instances");
    if !instances_dir.exists() {
        fs::create_dir_all(&instances_dir)
            .map_err(|e| format!("Failed to create instances directory: {e}"))?;
    }
    Ok(instances_dir)
}

pub fn get_instance_dir<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<PathBuf, String> {
    let instances_dir = get_instances_dir(app)?;
    let instance_dir = instances_dir.join(instance_id);
    if !instance_dir.exists() {
        fs::create_dir_all(&instance_dir)
            .map_err(|e| format!("Failed to create instance directory: {e}"))?;
        let _ = fs::create_dir_all(instance_dir.join("mods"));
        let _ = fs::create_dir_all(instance_dir.join("config"));
        let _ = fs::create_dir_all(instance_dir.join("saves"));
        let _ = fs::create_dir_all(instance_dir.join("resourcepacks"));
    }
    Ok(instance_dir)
}

fn get_instances_file<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {e}"))?;
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create app data directory: {e}"))?;
    }
    Ok(data_dir.join("instances.json"))
}

pub fn load_instances<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<Vec<InstanceConfig>, String> {
    let file_path = get_instances_file(app)?;
    if !file_path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read instances file: {e}"))?;
    serde_json::from_str(&data).map_err(|e| format!("Failed to parse instances file: {e}"))
}

pub fn get_instances<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<Vec<InstanceConfig>, String> {
    load_instances(app)
}

pub fn save_instances<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instances: &[InstanceConfig],
) -> Result<(), String> {
    let file_path = get_instances_file(app)?;
    let data = serde_json::to_string_pretty(instances)
        .map_err(|e| format!("Failed to serialize instances: {e}"))?;
    fs::write(&file_path, data).map_err(|e| format!("Failed to write instances file: {e}"))
}

pub fn create_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    name: String,
    game_version: String,
    loader: ModLoaderType,
    loader_version: Option<String>,
) -> Result<InstanceConfig, String> {
    let mut instances = load_instances(app)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let id = uuid::Uuid::new_v4().to_string();
    let instance = InstanceConfig {
        id: id.clone(),
        name,
        game_version,
        loader,
        loader_version,
        java_path: None,
        memory_min_mb: None,
        memory_max_mb: None,
        icon: None,
        created_at: now,
        last_played: None,
        total_play_time_seconds: 0,
        jvm_args: None,
        fullscreen: None,
        window_width: None,
        window_height: None,
        sync_options: None,
        sync_servers: None,
        sync_resource_packs: None,
        sync_command_history: None,
        sync_creative_hotbars: None,
        last_synced_at: None,
    };

    // Ensure instance dir exists
    get_instance_dir(app, &id)?;

    instances.push(instance.clone());
    save_instances(app, &instances)?;

    Ok(instance)
}

pub fn delete_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<(), String> {
    let mut instances = load_instances(app)?;
    instances.retain(|i| i.id != instance_id);
    save_instances(app, &instances)?;

    // Delete instance directory
    let instance_dir = get_instances_dir(app)?.join(instance_id);
    if instance_dir.exists() {
        let _ = fs::remove_dir_all(instance_dir);
    }

    Ok(())
}

pub fn update_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance: InstanceConfig,
) -> Result<(), String> {
    let mut instances = load_instances(app)?;
    if let Some(existing) = instances.iter_mut().find(|i| i.id == instance.id) {
        *existing = instance;
        save_instances(app, &instances)?;
        Ok(())
    } else {
        Err(format!("Instance not found: {}", instance.id))
    }
}

pub fn update_last_played<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    played_seconds: u64,
) -> Result<(), String> {
    let mut instances = load_instances(app)?;
    if let Some(existing) = instances.iter_mut().find(|i| i.id == instance_id) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        existing.last_played = Some(now);
        existing.total_play_time_seconds += played_seconds;
        save_instances(app, &instances)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstanceWorldSummary {
    pub folder_name: String,
    pub display_name: String,
    pub icon: Option<String>,
    pub last_played: Option<u64>,
}

pub fn get_instance_worlds(instance_dir: &std::path::Path) -> Result<Vec<InstanceWorldSummary>, String> {
    use std::io::Read;
    let saves_dir = instance_dir.join("saves");
    if !saves_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&saves_dir).map_err(|e| format!("Failed to read saves dir: {e}"))?;
    let mut worlds = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let folder_name = entry.file_name().to_string_lossy().to_string();
        if folder_name == "qp_world" || folder_name.starts_with(".qp_") {
            continue;
        }
        let level_dat = path.join("level.dat");
        if !level_dat.is_file() {
            continue;
        }

        let mut display_name = folder_name.clone();
        let mut last_played = None;

        if let Ok(metadata) = level_dat.metadata() {
            if let Ok(modified) = metadata.modified() {
                if let Ok(dur) = modified.duration_since(UNIX_EPOCH) {
                    last_played = Some(dur.as_secs());
                }
            }
        }

        if let Ok(compressed_bytes) = fs::read(&level_dat) {
            let mut gz = flate2::read::GzDecoder::new(&compressed_bytes[..]);
            let mut decompressed = Vec::new();
            if gz.read_to_end(&mut decompressed).is_ok() {
                // Search for LevelName string tag in NBT: Tag ID (0x08), Name Len (0x00, 0x09), "LevelName"
                let needle = b"\x08\x00\x09LevelName";
                if let Some(pos) = decompressed.windows(needle.len()).position(|w| w == needle) {
                    let val_start = pos + needle.len();
                    if val_start + 2 <= decompressed.len() {
                        let str_len = u16::from_be_bytes([decompressed[val_start], decompressed[val_start + 1]]) as usize;
                        let str_end = val_start + 2 + str_len;
                        if str_end <= decompressed.len() {
                            if let Ok(name) = std::str::from_utf8(&decompressed[val_start + 2..str_end]) {
                                if !name.trim().is_empty() {
                                    display_name = name.to_string();
                                }
                            }
                        }
                    }
                }

                // Also check for LastPlayed tag in NBT: Tag ID (0x04), Name Len (0x00, 0x0A), "LastPlayed"
                let lp_needle = b"\x04\x00\x0aLastPlayed";
                if let Some(pos) = decompressed.windows(lp_needle.len()).position(|w| w == lp_needle) {
                    let val_start = pos + lp_needle.len();
                    if val_start + 8 <= decompressed.len() {
                        if let Ok(bytes) = decompressed[val_start..val_start + 8].try_into() {
                            let lp_ms = i64::from_be_bytes(bytes);
                            if lp_ms > 0 {
                                last_played = Some((lp_ms / 1000) as u64);
                            }
                        }
                    }
                }
            }
        }

        let mut icon = None;
        let icon_path = path.join("icon.png");
        if icon_path.is_file() {
            if let Ok(icon_bytes) = fs::read(&icon_path) {
                use base64::Engine;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&icon_bytes);
                icon = Some(format!("data:image/png;base64,{b64}"));
            }
        }

        worlds.push(InstanceWorldSummary {
            folder_name,
            display_name,
            icon,
            last_played,
        });
    }

    worlds.sort_by(|a, b| b.last_played.cmp(&a.last_played));
    Ok(worlds)
}
