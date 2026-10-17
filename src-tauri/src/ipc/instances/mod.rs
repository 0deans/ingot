//! IPC procedures for Minecraft instance management.
pub(crate) mod imports;
pub(crate) mod screenshots;
pub(crate) mod sync;
pub(crate) mod version_change;

use crate::app::lifecycle::adopt_running_games;
use crate::ipc::events::TauRpcEventsApiEventTrigger;
use crate::ipc::ApiError;
use crate::minecraft::instance::{self, InstanceConfig, ModLoaderType};
use crate::minecraft::launcher::{
    self, InstanceStatusEvent, LaunchProgressEvent, ProcessManager, RunningInstanceSummary,
};
use crate::minecraft::version;
use tauri::{Manager, Runtime};

pub(super) fn get_process_manager<R: Runtime>(app: &tauri::AppHandle<R>) -> &ProcessManager {
    &crate::app::state(app).games
}

pub(super) fn get_http_client<R: Runtime>(app: &tauri::AppHandle<R>) -> &reqwest::Client {
    &crate::app::state(app).http
}

pub(super) fn find_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<InstanceConfig, ApiError> {
    instance::load_instances(app)
        .map_err(ApiError::from)?
        .into_iter()
        .find(|i| i.id == instance_id)
        .ok_or_else(|| ApiError::not_found(format!("Instance not found: {instance_id}")))
}

pub(super) async fn is_downgrade<R: Runtime>(
    app: &tauri::AppHandle<R>,
    from: &str,
    to: &str,
) -> Result<bool, ApiError> {
    let cache_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| ApiError::storage_failure(e.to_string()))?
        .join("cache");
    let manifest = version::fetch_version_manifest(get_http_client(app), &cache_dir)
        .await
        .map_err(ApiError::from)?;
    let released = |id: &str| {
        manifest
            .versions
            .iter()
            .find(|v| v.id == id)
            .map(|v| v.release_time.clone())
    };
    Ok(matches!(
        (released(from), released(to)),
        (Some(from), Some(to)) if to < from
    ))
}

#[taurpc::procedures(path = "instances")]
pub trait InstancesApi {
    async fn get_instances(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<InstanceConfig>, ApiError>;

    async fn create_instance(
        app_handle: tauri::AppHandle<impl Runtime>,
        name: String,
        game_version: String,
        loader: ModLoaderType,
        loader_version: Option<String>,
    ) -> Result<InstanceConfig, ApiError>;

    async fn delete_instance(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<(), ApiError>;

    async fn update_instance(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance: InstanceConfig,
    ) -> Result<(), ApiError>;

    /// Copies an instance into a new, independent one (worlds optional)
    async fn duplicate_instance(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        name: String,
        include_worlds: bool,
    ) -> Result<InstanceConfig, ApiError>;

    async fn open_instance_folder(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<(), ApiError>;

    async fn launch_instance(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        quick_play: Option<launcher::QuickPlayOptions>,
    ) -> Result<u32, ApiError>;

    async fn get_instance_worlds(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<Vec<instance::InstanceWorldSummary>, ApiError>;

    async fn kill_instance(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<(), ApiError>;

    async fn get_running_instances(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<RunningInstanceSummary>, ApiError>;
}

#[derive(Clone)]
pub struct InstancesApiImpl;

#[taurpc::resolvers]
impl InstancesApi for InstancesApiImpl {
    async fn get_instances(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<InstanceConfig>, ApiError> {
        instance::load_instances(&app_handle).map_err(ApiError::from)
    }

    async fn create_instance(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        name: String,
        game_version: String,
        loader: ModLoaderType,
        loader_version: Option<String>,
    ) -> Result<InstanceConfig, ApiError> {
        instance::create_instance(&app_handle, name, game_version, loader, loader_version)
            .map_err(ApiError::from)
    }

    async fn delete_instance(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<(), ApiError> {
        adopt_running_games(&app_handle).await;
        let ownership = get_process_manager(&app_handle)
            .reserve_maintenance(&instance_id)
            .map_err(ApiError::from)?;
        tokio::task::spawn_blocking(move || {
            let _ownership = ownership;
            instance::delete_instance(&app_handle, &instance_id)
        })
        .await
        .map_err(ApiError::from_display)?
        .map_err(ApiError::from)
    }

    async fn update_instance(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance: InstanceConfig,
    ) -> Result<(), ApiError> {
        adopt_running_games(&app_handle).await;
        let ownership = get_process_manager(&app_handle)
            .reserve_maintenance(&instance.id)
            .map_err(ApiError::from)?;
        tokio::task::spawn_blocking(move || {
            let _ownership = ownership;
            instance::update_instance(&app_handle, instance)
        })
        .await
        .map_err(ApiError::from_display)?
        .map_err(ApiError::from)
    }

    async fn duplicate_instance(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        name: String,
        include_worlds: bool,
    ) -> Result<InstanceConfig, ApiError> {
        adopt_running_games(&app_handle).await;
        let ownership = get_process_manager(&app_handle)
            .reserve_maintenance(&instance_id)
            .map_err(ApiError::from)?;
        tauri::async_runtime::spawn_blocking(move || {
            let _ownership = ownership;
            instance::duplicate_instance(&app_handle, &instance_id, name, include_worlds)
        })
        .await
        .map_err(ApiError::from_display)?
        .map_err(ApiError::from)
    }

    async fn open_instance_folder(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<(), ApiError> {
        let dir = instance::get_instance_dir(&app_handle, &instance_id).map_err(ApiError::from)?;
        crate::app::platform::open_directory(&dir).map_err(ApiError::from)
    }

    async fn launch_instance(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        quick_play: Option<launcher::QuickPlayOptions>,
    ) -> Result<u32, ApiError> {
        adopt_running_games(&app_handle).await;
        let pm = get_process_manager(&app_handle).clone();
        let client = get_http_client(&app_handle).clone();
        let app = app_handle.clone();

        let app_prog = app_handle.clone();
        let on_prog = move |ev: LaunchProgressEvent| {
            let trigger = TauRpcEventsApiEventTrigger::new(app_prog.clone());
            if let Err(e) = trigger.on_launch_progress(ev) {
                eprintln!("[IPC] Failed to emit on_launch_progress: {e}");
            }
        };

        let app_stat = app_handle.clone();
        let on_status = move |ev: InstanceStatusEvent| {
            let trigger = TauRpcEventsApiEventTrigger::new(app_stat.clone());
            if let Err(e) = trigger.on_instance_status_changed(ev) {
                eprintln!("[IPC] Failed to emit on_instance_status_changed: {e}");
            }
        };

        crate::app::games::launch_minecraft(
            app,
            pm,
            client,
            instance_id,
            quick_play,
            on_prog,
            on_status,
        )
        .await
        .map_err(ApiError::from)
    }

    async fn get_instance_worlds(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<Vec<instance::InstanceWorldSummary>, ApiError> {
        let instance_dir =
            instance::get_instance_dir(&app_handle, &instance_id).map_err(ApiError::from)?;
        instance::get_instance_worlds(&instance_dir).map_err(ApiError::from)
    }

    async fn kill_instance(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<(), ApiError> {
        get_process_manager(&app_handle)
            .kill_instance(&instance_id)
            .await
            .map_err(ApiError::from)
    }

    async fn get_running_instances(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<RunningInstanceSummary>, ApiError> {
        adopt_running_games(&app_handle).await;
        Ok(get_process_manager(&app_handle)
            .get_running_instances()
            .await)
    }
}
