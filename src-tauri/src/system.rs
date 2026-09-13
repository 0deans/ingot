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

pub const BEHAVIOR_KEEP_OPEN: &str = "keepOpen";
pub const BEHAVIOR_HIDE_TO_TRAY: &str = "hideToTray";
pub const BEHAVIOR_CLOSE: &str = "close";

fn default_launcher_behavior() -> String {
    BEHAVIOR_KEEP_OPEN.to_string()
}

#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct WindowSettings {
    pub fullscreen: bool,
    pub width: u32,
    pub height: u32,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            fullscreen: false,
            width: 854,
            height: 480,
        }
    }
}

#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct SyncSettings {
    pub sync_options: bool,
    pub sync_servers: bool,
    pub sync_resource_packs: bool,
    pub sync_command_history: bool,
    pub sync_creative_hotbars: bool,
    #[serde(default)]
    pub initialized_categories: Vec<String>,
}

impl Default for SyncSettings {
    fn default() -> Self {
        Self {
            sync_options: false,
            sync_servers: false,
            sync_resource_packs: false,
            sync_command_history: false,
            sync_creative_hotbars: false,
            initialized_categories: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherSettings {
    pub memory: MemorySettings,
    #[serde(default = "default_launcher_behavior")]
    pub launcher_behavior: String,
    #[serde(default)]
    pub window: WindowSettings,
    #[serde(default)]
    pub sync: SyncSettings,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            memory: MemorySettings::default(),
            launcher_behavior: default_launcher_behavior(),
            window: WindowSettings::default(),
            sync: SyncSettings::default(),
        }
    }
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

pub fn get_launcher_behavior<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> String {
    load_settings(app).launcher_behavior
}

pub fn set_launcher_behavior<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    behavior: String,
) -> Result<String, String> {
    let valid = match behavior.as_str() {
        BEHAVIOR_KEEP_OPEN | BEHAVIOR_HIDE_TO_TRAY | BEHAVIOR_CLOSE => true,
        _ => false,
    };
    if !valid {
        return Err(format!("Invalid launcher behavior: {behavior}"));
    }
    let mut settings = load_settings(app);
    settings.launcher_behavior = behavior.clone();
    save_settings(app, &settings)?;
    Ok(behavior)
}

pub fn get_window_settings<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> WindowSettings {
    load_settings(app).window
}

pub fn set_window_settings<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    window: WindowSettings,
) -> Result<WindowSettings, String> {
    let mut settings = load_settings(app);
    settings.window = window.clone();
    save_settings(app, &settings)?;
    Ok(window)
}

pub fn get_sync_settings<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> SyncSettings {
    load_settings(app).sync
}

pub fn set_sync_settings<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    sync: SyncSettings,
) -> Result<SyncSettings, String> {
    let mut settings = load_settings(app);
    settings.sync = sync.clone();
    save_settings(app, &settings)?;
    Ok(sync)
}
