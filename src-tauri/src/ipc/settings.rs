//! Native RPC adapters for global preferences; persistence stays in the feature.
use crate::system::{self, MemorySettings, SyncSettings, SystemMemoryInfo, WindowSettings};
use tauri::Runtime;

#[taurpc::procedures(path = "settings")]
pub trait SettingsApi {
    async fn get_system_memory() -> Result<SystemMemoryInfo, String>;

    async fn get_memory_settings(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<MemorySettings, String>;

    async fn set_memory_settings(
        app_handle: tauri::AppHandle<impl Runtime>,
        min_ram_mb: u32,
        max_ram_mb: u32,
    ) -> Result<MemorySettings, String>;

    async fn get_launcher_behavior(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<String, String>;

    async fn set_launcher_behavior(
        app_handle: tauri::AppHandle<impl Runtime>,
        behavior: String,
    ) -> Result<String, String>;

    async fn get_window_settings(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<WindowSettings, String>;

    async fn set_window_settings(
        app_handle: tauri::AppHandle<impl Runtime>,
        settings: WindowSettings,
    ) -> Result<WindowSettings, String>;

    async fn get_sync_settings(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<SyncSettings, String>;

    async fn set_sync_settings(
        app_handle: tauri::AppHandle<impl Runtime>,
        settings: SyncSettings,
    ) -> Result<SyncSettings, String>;
}

#[derive(Clone)]
pub struct SettingsApiImpl;

#[taurpc::resolvers]
impl SettingsApi for SettingsApiImpl {
    async fn get_system_memory(self) -> Result<SystemMemoryInfo, String> {
        Ok(system::get_memory_info())
    }

    async fn get_memory_settings(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<MemorySettings, String> {
        system::get_memory_settings(&app_handle).map_err(|error| error.to_string())
    }

    async fn set_memory_settings(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        min_ram_mb: u32,
        max_ram_mb: u32,
    ) -> Result<MemorySettings, String> {
        system::set_memory_settings(&app_handle, min_ram_mb, max_ram_mb)
            .map_err(|error| error.to_string())
    }

    async fn get_launcher_behavior(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<String, String> {
        system::get_launcher_behavior(&app_handle).map_err(|error| error.to_string())
    }

    async fn set_launcher_behavior(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        behavior: String,
    ) -> Result<String, String> {
        system::set_launcher_behavior(&app_handle, behavior).map_err(|error| error.to_string())
    }

    async fn get_window_settings(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<WindowSettings, String> {
        system::get_window_settings(&app_handle).map_err(|error| error.to_string())
    }

    async fn set_window_settings(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        settings: WindowSettings,
    ) -> Result<WindowSettings, String> {
        system::set_window_settings(&app_handle, settings).map_err(|error| error.to_string())
    }

    async fn get_sync_settings(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<SyncSettings, String> {
        system::get_sync_settings(&app_handle).map_err(|error| error.to_string())
    }

    async fn set_sync_settings(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        settings: SyncSettings,
    ) -> Result<SyncSettings, String> {
        system::set_sync_settings(&app_handle, settings).map_err(|error| error.to_string())
    }
}
