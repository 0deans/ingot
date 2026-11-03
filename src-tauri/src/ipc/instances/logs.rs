//! IPC procedures for client instance logs, crash reports, and paste uploads.
use super::{find_instance, get_http_client, get_process_manager};
use crate::app::instances::ensure_instance_dir;
use crate::ipc::ApiError;
use crate::minecraft::logs::{
    list_instance_log_files, read_instance_log_file, sanitize_log_content, upload_sanitized_log,
    InstanceLogFileContent, InstanceLogFileSummary, InstanceLogLine, LogError, LogFileKind,
    PasteEndpoints, PasteProvider, PasteUploadResult, SanitizeContext,
};
use tauri::Runtime;

fn build_sanitize_context<R: Runtime>(app: &tauri::AppHandle<R>) -> SanitizeContext {
    let (usernames, uuids) = match crate::app::accounts::load_accounts_file(app) {
        Ok(accounts) => {
            let names = accounts.iter().map(|a| a.username.clone()).collect();
            let ids = accounts.iter().map(|a| a.uuid.clone()).collect();
            (names, ids)
        }
        Err(_) => (Vec::new(), Vec::new()),
    };
    SanitizeContext::from_system(usernames, uuids)
}

#[taurpc::procedures(path = "instances.logs")]
pub trait InstanceLogsApi {
    async fn get_live_instance_logs(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<Vec<InstanceLogLine>, ApiError>;

    async fn clear_live_instance_logs(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<(), ApiError>;

    async fn list_instance_log_files(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<Vec<InstanceLogFileSummary>, ApiError>;

    async fn read_instance_log_file(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        kind: LogFileKind,
        file_name: String,
    ) -> Result<InstanceLogFileContent, ApiError>;

    async fn sanitize_log_text(
        app_handle: tauri::AppHandle<impl Runtime>,
        content: String,
    ) -> Result<String, ApiError>;

    async fn upload_log_paste(
        app_handle: tauri::AppHandle<impl Runtime>,
        content: String,
        provider: PasteProvider,
    ) -> Result<PasteUploadResult, ApiError>;

    async fn open_instance_log_location(
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        kind: Option<LogFileKind>,
        file_name: Option<String>,
    ) -> Result<(), ApiError>;
}

#[derive(Clone)]
pub struct InstanceLogsApiImpl;

#[taurpc::resolvers]
impl InstanceLogsApi for InstanceLogsApiImpl {
    async fn get_live_instance_logs(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<Vec<InstanceLogLine>, ApiError> {
        find_instance(&app_handle, &instance_id)?;
        Ok(get_process_manager(&app_handle).get_instance_logs(&instance_id))
    }

    async fn clear_live_instance_logs(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<(), ApiError> {
        find_instance(&app_handle, &instance_id)?;
        get_process_manager(&app_handle).clear_instance_logs(&instance_id);
        Ok(())
    }

    async fn list_instance_log_files(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
    ) -> Result<Vec<InstanceLogFileSummary>, ApiError> {
        find_instance(&app_handle, &instance_id)?;
        let dir = ensure_instance_dir(&app_handle, &instance_id).map_err(ApiError::from)?;
        tokio::task::spawn_blocking(move || list_instance_log_files(&dir))
            .await
            .map_err(LogError::Task)
            .and_then(|r| r)
            .map_err(ApiError::from)
    }

    async fn read_instance_log_file(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        kind: LogFileKind,
        file_name: String,
    ) -> Result<InstanceLogFileContent, ApiError> {
        find_instance(&app_handle, &instance_id)?;
        let dir = ensure_instance_dir(&app_handle, &instance_id).map_err(ApiError::from)?;
        tokio::task::spawn_blocking(move || read_instance_log_file(&dir, kind, &file_name))
            .await
            .map_err(LogError::Task)
            .and_then(|r| r)
            .map_err(ApiError::from)
    }

    async fn sanitize_log_text(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        content: String,
    ) -> Result<String, ApiError> {
        let ctx = build_sanitize_context(&app_handle);
        Ok(sanitize_log_content(&content, &ctx))
    }

    async fn upload_log_paste(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        content: String,
        provider: PasteProvider,
    ) -> Result<PasteUploadResult, ApiError> {
        let ctx = build_sanitize_context(&app_handle);
        let client = get_http_client(&app_handle).clone();
        let endpoints = PasteEndpoints::default();
        upload_sanitized_log(&client, provider, &content, &ctx, &endpoints)
            .await
            .map_err(ApiError::from)
    }

    async fn open_instance_log_location(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        instance_id: String,
        kind: Option<LogFileKind>,
        file_name: Option<String>,
    ) -> Result<(), ApiError> {
        find_instance(&app_handle, &instance_id)?;
        let instance_dir =
            ensure_instance_dir(&app_handle, &instance_id).map_err(ApiError::from)?;

        let target_path = match (kind, file_name) {
            (Some(LogFileKind::Log), Some(name)) => {
                crate::minecraft::logs::reader::validate_safe_file_name(&name)
                    .map_err(ApiError::from)?;
                let file_path = instance_dir.join("logs").join(&name);
                if file_path.exists() {
                    Some(file_path)
                } else {
                    Some(instance_dir.join("logs"))
                }
            }
            (Some(LogFileKind::CrashReport), Some(name)) => {
                crate::minecraft::logs::reader::validate_safe_file_name(&name)
                    .map_err(ApiError::from)?;
                let file_path = instance_dir.join("crash-reports").join(&name);
                if file_path.exists() {
                    Some(file_path)
                } else {
                    Some(instance_dir.join("crash-reports"))
                }
            }
            (Some(LogFileKind::Log), None) => Some(instance_dir.join("logs")),
            (Some(LogFileKind::CrashReport), None) => Some(instance_dir.join("crash-reports")),
            (None, _) => {
                let latest = instance_dir.join("logs").join("latest.log");
                if latest.exists() {
                    Some(latest)
                } else {
                    let logs = instance_dir.join("logs");
                    if logs.exists() {
                        Some(logs)
                    } else {
                        Some(instance_dir.clone())
                    }
                }
            }
        };

        if let Some(target) = target_path {
            if target.is_file() {
                crate::app::platform::reveal_file(&target).map_err(ApiError::from)?;
            } else {
                let _ = std::fs::create_dir_all(&target);
                if target.is_dir() {
                    crate::app::platform::open_directory(&target).map_err(ApiError::from)?;
                } else {
                    crate::app::platform::open_directory(&instance_dir).map_err(ApiError::from)?;
                }
            }
        }
        Ok(())
    }
}
