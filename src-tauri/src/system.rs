pub(crate) use crate::settings::LauncherSettings;
pub use crate::settings::{MemorySettings, SyncSettings, WindowSettings};
use crate::{
    app::AppState,
    settings::{service, SettingsError},
};
use tauri::Manager;

pub const BEHAVIOR_KEEP_OPEN: &str = "keepOpen";
pub const BEHAVIOR_HIDE_TO_TRAY: &str = "hideToTray";
pub const BEHAVIOR_CLOSE: &str = "close";

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

fn state<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<tauri::State<'_, AppState>, SettingsError> {
    app.try_state::<AppState>()
        .ok_or(SettingsError::StateUnavailable)
}

pub(crate) fn load_settings<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<LauncherSettings, SettingsError> {
    state(app)?.settings.load()
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

pub(crate) fn get_memory_settings<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<MemorySettings, SettingsError> {
    Ok(load_settings(app)?.memory)
}
pub(crate) fn set_memory_settings<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    min_ram_mb: u32,
    max_ram_mb: u32,
) -> Result<MemorySettings, SettingsError> {
    service::set_memory(&state(app)?.settings, min_ram_mb, max_ram_mb)
}
pub(crate) fn get_launcher_behavior<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<String, SettingsError> {
    Ok(load_settings(app)?.launcher_behavior.as_str().to_owned())
}
pub(crate) fn set_launcher_behavior<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    behavior: String,
) -> Result<String, SettingsError> {
    service::set_behavior(&state(app)?.settings, &behavior).map(|value| value.as_str().to_owned())
}
pub(crate) fn get_window_settings<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<WindowSettings, SettingsError> {
    Ok(load_settings(app)?.window)
}
pub(crate) fn set_window_settings<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    window: WindowSettings,
) -> Result<WindowSettings, SettingsError> {
    service::set_window(&state(app)?.settings, window)
}
pub(crate) fn get_sync_settings<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<SyncSettings, SettingsError> {
    Ok(load_settings(app)?.sync)
}
pub(crate) fn set_sync_settings<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    sync: SyncSettings,
) -> Result<SyncSettings, SettingsError> {
    service::set_sync(&state(app)?.settings, sync)
}
