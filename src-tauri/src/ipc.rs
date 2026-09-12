use crate::account::{self, AccountProfile};
use crate::system::{self, MemorySettings, SystemMemoryInfo};
use tauri::Runtime;

#[taurpc::procedures]
pub trait AppApi {
    async fn greet(name: String) -> String;
    async fn ely_login(
        app_handle: tauri::AppHandle<impl Runtime>,
        username: String,
        password: String,
    ) -> Result<AccountProfile, String>;
    async fn add_offline_account(
        app_handle: tauri::AppHandle<impl Runtime>,
        username: String,
    ) -> Result<AccountProfile, String>;
    async fn get_accounts(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<AccountProfile>, String>;
    async fn set_active_account(
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
    ) -> Result<(), String>;
    async fn remove_account(
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
    ) -> Result<(), String>;
    async fn get_active_account_token(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<String, String>;
    async fn get_system_memory() -> Result<SystemMemoryInfo, String>;
    async fn get_memory_settings(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<MemorySettings, String>;
    async fn set_memory_settings(
        app_handle: tauri::AppHandle<impl Runtime>,
        min_ram_mb: u32,
        max_ram_mb: u32,
    ) -> Result<MemorySettings, String>;
    async fn get_skin_data_url(
        app_handle: tauri::AppHandle<impl Runtime>,
        skin_url: String,
    ) -> Result<String, String>;
    async fn save_skin_to_downloads(
        app_handle: tauri::AppHandle<impl Runtime>,
        username: String,
        skin_url: String,
    ) -> Result<String, String>;
    async fn reorder_accounts(
        app_handle: tauri::AppHandle<impl Runtime>,
        account_ids: Vec<String>,
    ) -> Result<(), String>;
    #[taurpc(event)]
    async fn on_memory_changed(settings: MemorySettings);
}

#[derive(Clone)]
pub struct AppApiImpl;

#[taurpc::resolvers]
impl AppApi for AppApiImpl {
    async fn greet(self, name: String) -> String {
        format!("Hello, {}! You've been greeted from Rust via TauRPC!", name)
    }

    async fn ely_login(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        username: String,
        password: String,
    ) -> Result<AccountProfile, String> {
        account::ely_login(app_handle, username, password).await
    }

    async fn add_offline_account(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        username: String,
    ) -> Result<AccountProfile, String> {
        account::add_offline_account(app_handle, username)
    }

    async fn get_accounts(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<AccountProfile>, String> {
        account::get_accounts(app_handle)
    }

    async fn set_active_account(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
    ) -> Result<(), String> {
        account::set_active_account(app_handle, account_id)
    }

    async fn remove_account(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
    ) -> Result<(), String> {
        account::remove_account(app_handle, account_id)
    }

    async fn get_active_account_token(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<String, String> {
        account::get_active_account_token(app_handle).await
    }

    async fn get_system_memory(self) -> Result<SystemMemoryInfo, String> {
        Ok(system::get_memory_info())
    }

    async fn get_memory_settings(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<MemorySettings, String> {
        Ok(system::get_memory_settings(&app_handle))
    }

    async fn set_memory_settings(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        min_ram_mb: u32,
        max_ram_mb: u32,
    ) -> Result<MemorySettings, String> {
        system::set_memory_settings(&app_handle, min_ram_mb, max_ram_mb)
    }

    async fn get_skin_data_url(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        skin_url: String,
    ) -> Result<String, String> {
        account::get_skin_data_url(app_handle, skin_url).await
    }

    async fn save_skin_to_downloads(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        username: String,
        skin_url: String,
    ) -> Result<String, String> {
        account::save_skin_to_downloads(app_handle, username, skin_url).await
    }

    async fn reorder_accounts(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        account_ids: Vec<String>,
    ) -> Result<(), String> {
        account::reorder_accounts(app_handle, account_ids)
    }
}
