use crate::account::{self, AccountProfile};
use crate::minecraft::content::{self, ContentSearchResult, UnifiedContentDetails};
use crate::minecraft::importer::{self, DetectedLauncher, ImportInstanceOptions, ImportReport, ImportableInstance};
use crate::minecraft::instance::{self, InstanceConfig, ModLoaderType};
use crate::minecraft::launcher::{
    self, InstanceStatusEvent, LaunchProgressEvent, ProcessManager, RunningInstanceSummary,
};
use crate::minecraft::loader;
use crate::minecraft::screenshots::{self, ScreenshotInfo};
use crate::minecraft::sync::{self, SharedSyncStatus, SyncConflictInfo, SyncReport};
use crate::minecraft::version::{self, VersionManifestEntry};
use crate::server::{
    self, PlayitTunnelStatus, RunningServerSummary, ServerConfig, ServerCoreType, ServerLogEvent,
    ServerPingResponse, ServerProperties, ServerProcessManager, ServerStatusEvent, WhitelistEntry,
};
use crate::system::{self, MemorySettings, SyncSettings, SystemMemoryInfo, WindowSettings};
use std::sync::OnceLock;
use tauri::{Manager, Runtime};

static PROCESS_MANAGER: OnceLock<ProcessManager> = OnceLock::new();
static SERVER_PROCESS_MANAGER: OnceLock<ServerProcessManager> = OnceLock::new();
static SERVER_SUPERVISOR_MANAGER: OnceLock<server::ServerSupervisorManager> = OnceLock::new();
static PLAYIT_MANAGER: OnceLock<server::PlayitManager> = OnceLock::new();
static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn get_process_manager() -> &'static ProcessManager {
    PROCESS_MANAGER.get_or_init(ProcessManager::new)
}

fn get_server_process_manager() -> &'static ServerProcessManager {
    SERVER_PROCESS_MANAGER.get_or_init(ServerProcessManager::new)
}

fn get_server_supervisor_manager() -> &'static server::ServerSupervisorManager {
    SERVER_SUPERVISOR_MANAGER.get_or_init(server::ServerSupervisorManager::new)
}

fn get_playit_manager() -> &'static server::PlayitManager {
    PLAYIT_MANAGER.get_or_init(server::PlayitManager::new)
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
        uploader: Option<String>,
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
        quick_play: Option<launcher::QuickPlayOptions>,
    ) -> Result<u32, String>;

    async fn get_instance_worlds(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<Vec<instance::InstanceWorldSummary>, String>;

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

    // Content Discovery (CurseForge & Modrinth)
    async fn search_content(
        source: String,
        project_type: String,
        query: Option<String>,
        game_version: Option<String>,
        loader: Option<String>,
        sort: Option<String>,
        page: u32,
        page_size: u32,
    ) -> Result<ContentSearchResult, String>;

    async fn get_content_details(
        source: String,
        project_id: String,
    ) -> Result<UnifiedContentDetails, String>;

    async fn install_content_file(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        project_type: String,
        download_url: String,
        filename: String,
    ) -> Result<String, String>;

    async fn install_modpack_instance(
        app_handle: tauri::AppHandle<impl Runtime>,
        name: String,
        source: String,
        download_url: String,
        filename: String,
    ) -> Result<InstanceConfig, String>;

    // Dedicated Server Management
    async fn get_servers(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<ServerConfig>, String>;

    async fn create_server(
        app_handle: tauri::AppHandle<impl Runtime>,
        name: String,
        core: ServerCoreType,
        game_version: String,
        build_number: Option<String>,
        port: Option<u16>,
        memory_min_mb: Option<u32>,
        memory_max_mb: Option<u32>,
    ) -> Result<ServerConfig, String>;

    async fn delete_server(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        delete_files: bool,
    ) -> Result<(), String>;

    async fn update_server(
        app_handle: tauri::AppHandle<impl Runtime>,
        server: ServerConfig,
    ) -> Result<(), String>;

    async fn get_server_properties(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<ServerProperties, String>;

    async fn set_server_properties(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        properties: ServerProperties,
    ) -> Result<(), String>;

    async fn open_server_folder(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), String>;

    async fn start_server(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<u32, String>;

    async fn put_server_to_sleep(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), String>;

    async fn stop_server(server_id: String) -> Result<(), String>;

    async fn kill_server(server_id: String) -> Result<(), String>;

    async fn send_server_command(server_id: String, command: String) -> Result<(), String>;

    async fn start_playit_tunnel(
        app_handle: tauri::AppHandle<impl Runtime>,
        secret_key: Option<String>,
    ) -> Result<PlayitTunnelStatus, String>;

    async fn stop_playit_tunnel() -> Result<(), String>;

    async fn get_playit_status() -> Result<PlayitTunnelStatus, String>;

    async fn get_running_servers() -> Result<Vec<RunningServerSummary>, String>;

    async fn get_server_logs(server_id: String) -> Result<Vec<String>, String>;

    async fn get_server_online_players(server_id: String) -> Result<Vec<String>, String>;

    async fn get_server_whitelist(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<WhitelistEntry>, String>;

    async fn add_to_server_whitelist(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        username: String,
    ) -> Result<(), String>;

    async fn remove_from_server_whitelist(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        username: String,
    ) -> Result<(), String>;

    async fn ping_server(port: u16) -> Result<ServerPingResponse, String>;

    async fn get_server_icon(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Option<String>, String>;

    async fn set_server_icon(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        base64_data: String,
    ) -> Result<(), String>;

    async fn get_available_server_core_versions(
        core: ServerCoreType,
    ) -> Result<Vec<String>, String>;

    #[taurpc(event)]
    async fn on_memory_changed(settings: MemorySettings);

    #[taurpc(event)]
    async fn on_instance_status_changed(event: InstanceStatusEvent);

    #[taurpc(event)]
    async fn on_launch_progress(event: LaunchProgressEvent);

    #[taurpc(event)]
    async fn on_server_log(event: ServerLogEvent);

    #[taurpc(event)]
    async fn on_server_status_changed(event: ServerStatusEvent);
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
        uploader: Option<String>,
    ) -> Result<crate::auth::ely::ElySkinsCatalogResponse, String> {
        account::get_ely_skins_catalog(page, query, sort, model, uploader).await
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
        let _path_str = dir.to_string_lossy().to_string();
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("explorer").arg(&_path_str).spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(&_path_str).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(&_path_str).spawn();
        }
        Ok(())
    }

    async fn launch_instance(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        quick_play: Option<launcher::QuickPlayOptions>,
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

        launcher::launch_minecraft(app, pm, client, inst, quick_play, on_prog, on_status).await
    }

    async fn get_instance_worlds(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<Vec<instance::InstanceWorldSummary>, String> {
        let instance_dir = instance::get_instance_dir(&app_handle, &instance_id)?;
        instance::get_instance_worlds(&instance_dir)
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

    async fn search_content(
        self,
        source: String,
        project_type: String,
        query: Option<String>,
        game_version: Option<String>,
        loader: Option<String>,
        sort: Option<String>,
        page: u32,
        page_size: u32,
    ) -> Result<ContentSearchResult, String> {
        content::search_content(
            source,
            project_type,
            query,
            game_version,
            loader,
            sort,
            page,
            page_size,
        )
        .await
    }

    async fn get_content_details(
        self,
        source: String,
        project_id: String,
    ) -> Result<UnifiedContentDetails, String> {
        content::get_content_details(&source, &project_id).await
    }

    async fn install_content_file(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        project_type: String,
        download_url: String,
        filename: String,
    ) -> Result<String, String> {
        content::install_content_file(&app_handle, &instance_id, &project_type, &download_url, &filename).await
    }

    async fn install_modpack_instance(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        name: String,
        source: String,
        download_url: String,
        filename: String,
    ) -> Result<InstanceConfig, String> {
        content::install_modpack_instance(&app_handle, &name, &source, &download_url, &filename).await
    }

    async fn get_servers(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<ServerConfig>, String> {
        server::load_servers(&app_handle)
    }

    async fn create_server(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        name: String,
        core: ServerCoreType,
        game_version: String,
        build_number: Option<String>,
        port: Option<u16>,
        memory_min_mb: Option<u32>,
        memory_max_mb: Option<u32>,
    ) -> Result<ServerConfig, String> {
        server::create_server(
            &app_handle,
            name,
            core,
            game_version,
            build_number,
            port,
            memory_min_mb,
            memory_max_mb,
        )
    }

    async fn delete_server(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        delete_files: bool,
    ) -> Result<(), String> {
        server::delete_server(&app_handle, &server_id, delete_files)
    }

    async fn update_server(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server: ServerConfig,
    ) -> Result<(), String> {
        server::update_server(&app_handle, server)
    }

    async fn get_server_properties(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<ServerProperties, String> {
        let dir = server::get_server_dir(&app_handle, &server_id)?;
        server::read_server_properties_from_dir(&dir)
    }

    async fn set_server_properties(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        properties: ServerProperties,
    ) -> Result<(), String> {
        let dir = server::get_server_dir(&app_handle, &server_id)?;
        server::write_server_properties_to_dir(&dir, &properties)
    }

    async fn open_server_folder(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), String> {
        let dir = server::get_server_dir(&app_handle, &server_id)?;
        let _path_str = dir.to_string_lossy().to_string();
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("explorer").arg(&_path_str).spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(&_path_str).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(&_path_str).spawn();
        }
        Ok(())
    }

    async fn start_server(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<u32, String> {
        let servers = server::load_servers(&app_handle)?;
        let config = servers
            .into_iter()
            .find(|s| s.id == server_id)
            .ok_or_else(|| format!("Server not found: {}", server_id))?;

        let pm = get_server_process_manager().clone();
        let client = get_http_client().clone();
        let app = app_handle.clone();

        let app_log = app_handle.clone();
        let on_log = move |ev: ServerLogEvent| {
            let trigger = TauRpcAppApiEventTrigger::new(app_log.clone());
            if let Err(e) = trigger.on_server_log(ev) {
                eprintln!("[IPC] Failed to emit on_server_log: {e}");
            }
        };

        let app_status = app_handle.clone();
        let on_status = move |ev: ServerStatusEvent| {
            let trigger = TauRpcAppApiEventTrigger::new(app_status.clone());
            if let Err(e) = trigger.on_server_status_changed(ev) {
                eprintln!("[IPC] Failed to emit on_server_status_changed: {e}");
            }
        };

        get_server_supervisor_manager()
            .supervise_and_start(app, pm, client, config, on_log, on_status)
            .await
    }

    async fn put_server_to_sleep(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<(), String> {
        let app_status = app_handle.clone();
        let on_status = move |ev: ServerStatusEvent| {
            let trigger = TauRpcAppApiEventTrigger::new(app_status.clone());
            if let Err(e) = trigger.on_server_status_changed(ev) {
                eprintln!("[IPC] Failed to emit on_server_status_changed: {e}");
            }
        };
        get_server_supervisor_manager()
            .put_to_sleep(get_server_process_manager(), &server_id, on_status)
            .await
    }

    async fn stop_server(self, server_id: String) -> Result<(), String> {
        get_server_supervisor_manager()
            .stop_supervised(get_server_process_manager(), &server_id)
            .await
    }

    async fn kill_server(self, server_id: String) -> Result<(), String> {
        get_server_process_manager().kill_server(&server_id).await
    }

    async fn start_playit_tunnel(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        secret_key: Option<String>,
    ) -> Result<PlayitTunnelStatus, String> {
        get_playit_manager()
            .start_tunnel(&app_handle, get_http_client(), secret_key)
            .await
    }

    async fn stop_playit_tunnel(self) -> Result<(), String> {
        get_playit_manager().stop_tunnel().await
    }

    async fn get_playit_status(self) -> Result<PlayitTunnelStatus, String> {
        Ok(get_playit_manager().get_status().await)
    }

    async fn send_server_command(self, server_id: String, command: String) -> Result<(), String> {
        get_server_process_manager()
            .send_command(&server_id, &command)
            .await
    }

    async fn get_running_servers(self) -> Result<Vec<RunningServerSummary>, String> {
        Ok(get_server_process_manager().get_running_servers().await)
    }

    async fn get_server_logs(self, server_id: String) -> Result<Vec<String>, String> {
        Ok(get_server_process_manager().get_server_logs(&server_id).await)
    }

    async fn get_server_online_players(self, server_id: String) -> Result<Vec<String>, String> {
        Ok(get_server_process_manager().get_server_online_players(&server_id).await)
    }

    async fn get_server_whitelist(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<WhitelistEntry>, String> {
        let dir = server::get_server_dir(&app_handle, &server_id)?;
        server::read_server_whitelist(&dir)
    }

    async fn add_to_server_whitelist(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        username: String,
    ) -> Result<(), String> {
        let dir = server::get_server_dir(&app_handle, &server_id)?;
        server::add_to_server_whitelist(&dir, &username)
    }

    async fn remove_from_server_whitelist(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        username: String,
    ) -> Result<(), String> {
        let dir = server::get_server_dir(&app_handle, &server_id)?;
        server::remove_from_server_whitelist(&dir, &username)
    }

    async fn ping_server(self, port: u16) -> Result<ServerPingResponse, String> {
        server::ping_server("127.0.0.1", port).await
    }

    async fn get_server_icon(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Option<String>, String> {
        let dir = server::get_server_dir(&app_handle, &server_id)?;
        Ok(server::get_server_icon_base64(&dir))
    }

    async fn set_server_icon(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        base64_data: String,
    ) -> Result<(), String> {
        let dir = server::get_server_dir(&app_handle, &server_id)?;
        server::save_server_icon(&dir, &base64_data)
    }

    async fn get_available_server_core_versions(
        self,
        core: ServerCoreType,
    ) -> Result<Vec<String>, String> {
        server::fetch_core_versions(get_http_client(), &core).await
    }
}
