use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct SystemMemoryInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub total_mb: u64,
    pub available_mb: u64,
    pub used_mb: u64,
}

#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct MemorySettings {
    pub min_ram_mb: u32,
    pub max_ram_mb: u32,
}

impl Default for MemorySettings {
    fn default() -> Self {
        Self {
            min_ram_mb: 2048,
            max_ram_mb: 4096,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LauncherSettings {
    pub memory: MemorySettings,
}

fn get_settings_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {e}"))?;

    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create app data directory: {e}"))?;
    }

    Ok(data_dir.join("settings.json"))
}

pub fn load_settings<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> LauncherSettings {
    if let Ok(path) = get_settings_path(app) {
        if path.exists() {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str::<LauncherSettings>(&data) {
                    return settings;
                }
            }
        }
    }
    LauncherSettings::default()
}

pub fn save_settings<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    settings: &LauncherSettings,
) -> Result<(), String> {
    let path = get_settings_path(app)?;
    let data = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {e}"))?;
    fs::write(&path, data).map_err(|e| format!("Failed to write settings file: {e}"))?;
    Ok(())
}

pub fn get_memory_info() -> SystemMemoryInfo {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();

    let total_bytes = sys.total_memory();
    let available_bytes = sys.available_memory();
    let used_bytes = sys.used_memory();

    let total_mb = total_bytes / (1024 * 1024);
    let available_mb = available_bytes / (1024 * 1024);
    let used_mb = used_bytes / (1024 * 1024);

    SystemMemoryInfo {
        total_bytes,
        available_bytes,
        used_bytes,
        total_mb,
        available_mb,
        used_mb,
    }
}

pub fn get_memory_settings<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> MemorySettings {
    load_settings(app).memory
}

pub fn set_memory_settings<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    min_ram_mb: u32,
    max_ram_mb: u32,
) -> Result<MemorySettings, String> {
    let mut settings = load_settings(app);
    let min = min_ram_mb.min(max_ram_mb);
    let max = min_ram_mb.max(max_ram_mb);

    let mem = MemorySettings {
        min_ram_mb: min,
        max_ram_mb: max,
    };
    settings.memory = mem.clone();
    save_settings(app, &settings)?;
    Ok(mem)
}
