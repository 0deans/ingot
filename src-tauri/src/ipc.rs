use crate::account::{self, AccountProfile};
use crate::minecraft::importer::{self, DetectedLauncher, ImportInstanceOptions, ImportReport, ImportableInstance};
use crate::minecraft::instance::{self, InstanceConfig, ModLoaderType};
use crate::minecraft::launcher::{
    self, InstanceStatusEvent, LaunchProgressEvent, ProcessManager, RunningInstanceSummary,
};
use crate::minecraft::loader;
use crate::minecraft::screenshots::{self, ScreenshotInfo};
use crate::minecraft::sync::{self, SharedSyncStatus, SyncConflictInfo, SyncReport};
use crate::minecraft::version::{self, VersionManifestEntry};
use crate::system::{self, MemorySettings, SyncSettings, SystemMemoryInfo, WindowSettings};
use std::sync::OnceLock;
use tauri::{Manager, Runtime};

static PROCESS_MANAGER: OnceLock<ProcessManager> = OnceLock::new();
static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn get_process_manager() -> &'static ProcessManager {
    PROCESS_MANAGER.get_or_init(ProcessManager::new)
}

fn get_http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("IngotLauncher/0.1.0")
            .build()
            .unwrap_or_default()
    })
}

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
    async fn get_launcher_behavior(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<String, String>;
    async fn set_launcher_behavior(
        app_handle: tauri::AppHandle<impl Runtime>,
        behavior: String,
    ) -> Result<String, String>;
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
    async fn get_ely_skins(
        page: u32,
        query: Option<String>,
        sort: Option<String>,
        model: Option<String>,
    ) -> Result<crate::auth::ely::ElySkinsCatalogResponse, String>;
    async fn apply_ely_skin(
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
        skin_id: u64,
        password: Option<String>,
    ) -> Result<(), String>;
    async fn upload_ely_skin(
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
        image_base64: String,
        password: Option<String>,
    ) -> Result<(), String>;
    async fn has_ely_web_credentials(account_id: String) -> Result<bool, String>;

    // Minecraft Instance Management
    async fn get_instances(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<InstanceConfig>, String>;

    async fn create_instance(
        app_handle: tauri::AppHandle<impl Runtime>,
        name: String,
        game_version: String,
        loader: ModLoaderType,
        loader_version: Option<String>,
    ) -> Result<InstanceConfig, String>;

    async fn delete_instance(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<(), String>;

    async fn update_instance(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance: InstanceConfig,
    ) -> Result<(), String>;

    async fn open_instance_folder(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<(), String>;

    async fn launch_instance(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<u32, String>;

    async fn kill_instance(instance_id: String) -> Result<(), String>;

    async fn get_running_instances() -> Result<Vec<RunningInstanceSummary>, String>;

    async fn get_available_game_versions(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<VersionManifestEntry>, String>;

    async fn get_available_loader_versions(
        game_version: String,
        loader: ModLoaderType,
    ) -> Result<Vec<String>, String>;

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

    async fn push_instance_sync(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<SyncReport, String>;

    async fn export_instance_category_to_shared(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        category: String,
    ) -> Result<SyncReport, String>;

    async fn pull_instance_sync(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<SyncReport, String>;

    async fn get_shared_sync_status(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<SharedSyncStatus, String>;

    async fn check_sync_conflict(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<Option<SyncConflictInfo>, String>;

    async fn resolve_sync_conflict(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        resolution: String,
    ) -> Result<SyncReport, String>;

    async fn get_detected_launchers(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<DetectedLauncher>, String>;

    async fn get_launcher_instances(
        app_handle: tauri::AppHandle<impl Runtime>,
        launcher_id: String,
        custom_path: Option<String>,
    ) -> Result<Vec<ImportableInstance>, String>;

    async fn detect_custom_instance(
        app_handle: tauri::AppHandle<impl Runtime>,
        path: String,
    ) -> Result<ImportableInstance, String>;

    async fn import_instance(
        app_handle: tauri::AppHandle<impl Runtime>,
        options: ImportInstanceOptions,
    ) -> Result<ImportReport, String>;

    // Screenshot Management
    async fn get_all_screenshots(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<ScreenshotInfo>, String>;

    async fn delete_screenshot(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        file_name: String,
    ) -> Result<(), String>;

    async fn open_screenshots_folder(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: Option<String>,
    ) -> Result<(), String>;

    async fn reveal_screenshot_file(
        file_path: String,
    ) -> Result<(), String>;

    #[taurpc(event)]
    async fn on_memory_changed(settings: MemorySettings);

    #[taurpc(event)]
    async fn on_instance_status_changed(event: InstanceStatusEvent);

    #[taurpc(event)]
    async fn on_launch_progress(event: LaunchProgressEvent);
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

    async fn get_launcher_behavior(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<String, String> {
        Ok(system::get_launcher_behavior(&app_handle))
    }

    async fn set_launcher_behavior(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        behavior: String,
    ) -> Result<String, String> {
        system::set_launcher_behavior(&app_handle, behavior)
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

    async fn get_ely_skins(
        self,
        page: u32,
        query: Option<String>,
        sort: Option<String>,
        model: Option<String>,
    ) -> Result<crate::auth::ely::ElySkinsCatalogResponse, String> {
        account::get_ely_skins_catalog(page, query, sort, model).await
    }

    async fn apply_ely_skin(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
        skin_id: u64,
        password: Option<String>,
    ) -> Result<(), String> {
        account::apply_ely_skin(&app_handle, &account_id, skin_id, password).await
    }

    async fn upload_ely_skin(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
        image_base64: String,
        password: Option<String>,
    ) -> Result<(), String> {
        account::upload_ely_skin(&app_handle, &account_id, &image_base64, password).await
    }

    async fn has_ely_web_credentials(self, account_id: String) -> Result<bool, String> {
        Ok(account::has_ely_web_credentials(&account_id))
    }

    async fn get_instances(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<InstanceConfig>, String> {
        instance::load_instances(&app_handle)
    }

    async fn create_instance(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        name: String,
        game_version: String,
        loader: ModLoaderType,
        loader_version: Option<String>,
    ) -> Result<InstanceConfig, String> {
        instance::create_instance(&app_handle, name, game_version, loader, loader_version)
    }

    async fn delete_instance(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<(), String> {
        instance::delete_instance(&app_handle, &instance_id)
    }

    async fn update_instance(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance: InstanceConfig,
    ) -> Result<(), String> {
        instance::update_instance(&app_handle, instance)
    }

    async fn open_instance_folder(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<(), String> {
        let dir = instance::get_instance_dir(&app_handle, &instance_id)?;
        let path_str = dir.to_string_lossy().to_string();
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("explorer").arg(&path_str).spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(&path_str).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(&path_str).spawn();
        }
        Ok(())
    }

    async fn launch_instance(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<u32, String> {
        let instances = instance::load_instances(&app_handle)?;
        let inst = instances
            .into_iter()
            .find(|i| i.id == instance_id)
            .ok_or_else(|| format!("Instance not found: {}", instance_id))?;

        let pm = get_process_manager().clone();
        let client = get_http_client().clone();
        let app = app_handle.clone();

        let app_prog = app_handle.clone();
        let on_prog = move |ev: LaunchProgressEvent| {
            let trigger = TauRpcAppApiEventTrigger::new(app_prog.clone());
            if let Err(e) = trigger.on_launch_progress(ev) {
                eprintln!("[IPC] Failed to emit on_launch_progress: {e}");
            }
        };

        let app_stat = app_handle.clone();
        let on_status = move |ev: InstanceStatusEvent| {
            let trigger = TauRpcAppApiEventTrigger::new(app_stat.clone());
            if let Err(e) = trigger.on_instance_status_changed(ev) {
                eprintln!("[IPC] Failed to emit on_instance_status_changed: {e}");
            }
        };

        launcher::launch_minecraft(app, pm, client, inst, on_prog, on_status).await
    }

    async fn kill_instance(self, instance_id: String) -> Result<(), String> {
        get_process_manager().kill_instance(&instance_id).await
    }

    async fn get_running_instances(self) -> Result<Vec<RunningInstanceSummary>, String> {
        Ok(get_process_manager().get_running_instances().await)
    }

    async fn get_available_game_versions(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<VersionManifestEntry>, String> {
        let data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data directory: {e}"))?;
        let cache_dir = data_dir.join("cache");
        let manifest = version::fetch_version_manifest(get_http_client(), &cache_dir).await?;
        Ok(manifest.versions)
    }

    async fn get_available_loader_versions(
        self,
        game_version: String,
        loader: ModLoaderType,
    ) -> Result<Vec<String>, String> {
        loader::fetch_loader_versions(get_http_client(), &loader, &game_version).await
    }

    async fn get_window_settings(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<WindowSettings, String> {
        Ok(system::get_window_settings(&app_handle))
    }

    async fn set_window_settings(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        settings: WindowSettings,
    ) -> Result<WindowSettings, String> {
        system::set_window_settings(&app_handle, settings)
    }

    async fn get_sync_settings(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<SyncSettings, String> {
        Ok(system::get_sync_settings(&app_handle))
    }

    async fn set_sync_settings(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        settings: SyncSettings,
    ) -> Result<SyncSettings, String> {
        system::set_sync_settings(&app_handle, settings)
    }

    async fn push_instance_sync(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<SyncReport, String> {
        sync::export_instance_to_shared(&app_handle, &instance_id)
    }

    async fn export_instance_category_to_shared(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        category: String,
    ) -> Result<SyncReport, String> {
        sync::export_instance_category_to_shared(&app_handle, &instance_id, &category)
    }

    async fn pull_instance_sync(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<SyncReport, String> {
        sync::import_shared_to_instance(&app_handle, &instance_id)
    }

    async fn get_shared_sync_status(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<SharedSyncStatus, String> {
        sync::get_shared_sync_status(&app_handle)
    }

    async fn check_sync_conflict(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<Option<SyncConflictInfo>, String> {
        sync::check_sync_conflict(&app_handle, &instance_id)
    }

    async fn resolve_sync_conflict(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        resolution: String,
    ) -> Result<SyncReport, String> {
        sync::resolve_sync_conflict(&app_handle, &instance_id, &resolution)
    }

    async fn get_detected_launchers(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<DetectedLauncher>, String> {
        Ok(importer::get_detected_launchers(&app_handle))
    }

    async fn get_launcher_instances(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        launcher_id: String,
        custom_path: Option<String>,
    ) -> Result<Vec<ImportableInstance>, String> {
        importer::get_launcher_instances(&app_handle, &launcher_id, custom_path.as_deref())
    }

    async fn detect_custom_instance(
        self,
        _app_handle: tauri::AppHandle<impl Runtime>,
        path: String,
    ) -> Result<ImportableInstance, String> {
        importer::detect_custom_instance(&path)
    }

    async fn import_instance(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        options: ImportInstanceOptions,
    ) -> Result<ImportReport, String> {
        importer::import_instance(&app_handle, options)
    }

    async fn get_all_screenshots(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<ScreenshotInfo>, String> {
        screenshots::get_all_screenshots(&app_handle)
    }

    async fn delete_screenshot(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        file_name: String,
    ) -> Result<(), String> {
        screenshots::delete_screenshot(&app_handle, &instance_id, &file_name)
    }

    async fn open_screenshots_folder(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: Option<String>,
    ) -> Result<(), String> {
        screenshots::open_screenshots_folder(&app_handle, instance_id.as_deref())
    }

    async fn reveal_screenshot_file(
        self,
        file_path: String,
    ) -> Result<(), String> {
        screenshots::reveal_screenshot_file(&file_path)
    }
}
