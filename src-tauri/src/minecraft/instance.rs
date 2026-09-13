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
