use crate::ipc::ApiError;
use crate::system::{self, MemorySettings, SyncSettings, SystemMemoryInfo, WindowSettings};
use tauri::Runtime;

#[taurpc::procedures(path = "settings")]
pub trait SettingsApi {
    async fn get_system_memory() -> Result<SystemMemoryInfo, ApiError>;

    async fn get_memory_settings(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<MemorySettings, ApiError>;

    async fn set_memory_settings(
        app_handle: tauri::AppHandle<impl Runtime>,
        min_ram_mb: u32,
        max_ram_mb: u32,
    ) -> Result<MemorySettings, ApiError>;

    async fn get_launcher_behavior(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<String, ApiError>;

    async fn set_launcher_behavior(
        app_handle: tauri::AppHandle<impl Runtime>,
        behavior: String,
    ) -> Result<String, ApiError>;

    async fn get_window_settings(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<WindowSettings, ApiError>;

    async fn set_window_settings(
        app_handle: tauri::AppHandle<impl Runtime>,
        settings: WindowSettings,
    ) -> Result<WindowSettings, ApiError>;

    async fn get_sync_settings(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<SyncSettings, ApiError>;

    async fn set_sync_settings(
        app_handle: tauri::AppHandle<impl Runtime>,
        settings: SyncSettings,
    ) -> Result<SyncSettings, ApiError>;
}

#[derive(Clone)]
pub struct SettingsApiImpl;

#[taurpc::resolvers]
impl SettingsApi for SettingsApiImpl {
    async fn get_system_memory(self) -> Result<SystemMemoryInfo, ApiError> {
        Ok(system::get_memory_info())
    }

    async fn get_memory_settings(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<MemorySettings, ApiError> {
        system::get_memory_settings(&app_handle).map_err(ApiError::from)
    }

    async fn set_memory_settings(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        min_ram_mb: u32,
        max_ram_mb: u32,
    ) -> Result<MemorySettings, ApiError> {
        system::set_memory_settings(&app_handle, min_ram_mb, max_ram_mb).map_err(ApiError::from)
    }

    async fn get_launcher_behavior(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<String, ApiError> {
        system::get_launcher_behavior(&app_handle).map_err(ApiError::from)
    }

    async fn set_launcher_behavior(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        behavior: String,
    ) -> Result<String, ApiError> {
        system::set_launcher_behavior(&app_handle, behavior).map_err(ApiError::from)
    }

    async fn get_window_settings(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<WindowSettings, ApiError> {
        system::get_window_settings(&app_handle).map_err(ApiError::from)
    }

    async fn set_window_settings(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        settings: WindowSettings,
    ) -> Result<WindowSettings, ApiError> {
        system::set_window_settings(&app_handle, settings).map_err(ApiError::from)
    }

    async fn get_sync_settings(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<SyncSettings, ApiError> {
        system::get_sync_settings(&app_handle).map_err(ApiError::from)
    }

    async fn set_sync_settings(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        settings: SyncSettings,
    ) -> Result<SyncSettings, ApiError> {
        system::set_sync_settings(&app_handle, settings).map_err(ApiError::from)
    }
}
