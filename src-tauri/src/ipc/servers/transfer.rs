//! IPC procedures for server export, import, and duplication.
use super::{get_server_process_manager, import_upload_path, is_server_running, server_with_config};
use crate::ipc::ApiError;
use crate::server::transfer::{ExportMode, ImportedServer};
use crate::server::{self, ServerConfig};
use tauri::{Manager, Runtime};

#[taurpc::procedures(path = "servers.transfer")]
pub trait ServerTransferApi {
    /// Zips the server; saved to `dest`, or to the app cache when None.
    async fn export_server(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        mode: ExportMode,
        dest: Option<String>,
    ) -> Result<String, ApiError>;

    /// Appends a base64 chunk of an archive being imported.
    async fn import_upload_chunk(
        app_handle: tauri::AppHandle<impl Runtime>,
        upload_id: String,
        data_base64: String,
        first: bool,
    ) -> Result<(), ApiError>;

    /// Creates a new server from an uploaded archive.
    async fn import_server(
        app_handle: tauri::AppHandle<impl Runtime>,
        upload_id: String,
    ) -> Result<ImportedServer, ApiError>;

    /// Copies a server, optionally switching the copy to another version.
    async fn duplicate_server(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        name: String,
        game_version: Option<String>,
        build_number: Option<String>,
    ) -> Result<ServerConfig, ApiError>;
}

#[derive(Clone)]
pub struct ServerTransferApiImpl;

#[taurpc::resolvers]
impl ServerTransferApi for ServerTransferApiImpl {
    async fn export_server(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        mode: ExportMode,
        dest: Option<String>,
    ) -> Result<String, ApiError> {
        let (dir, config) = server_with_config(&app_handle, &server_id)?;
        let dest = match dest {
            Some(path) => std::path::PathBuf::from(path),
            None => app_handle
                .path()
                .app_cache_dir()
                .map_err(|e| ApiError::storage_failure(e.to_string()))?
                .join("exports")
                .join(server::transfer::export_file_name(&config, mode)),
        };
        let path = tauri::async_runtime::spawn_blocking(move || {
            server::transfer::export(&dir, &config, mode, &dest)
        })
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .map_err(ApiError::from)?;
        Ok(path.to_string_lossy().into_owned())
    }

    async fn import_upload_chunk(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        upload_id: String,
        data_base64: String,
        first: bool,
    ) -> Result<(), ApiError> {
        use base64::Engine;
        use std::io::Write;
        let path = import_upload_path(&app_handle, &upload_id)?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data_base64)
            .map_err(|e| ApiError::invalid_input(format!("Invalid upload chunk: {e}")))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ApiError::storage_failure(e.to_string()))?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(!first)
            .truncate(first)
            .open(&path)
            .map_err(|e| ApiError::storage_failure(format!("Failed to store upload: {e}")))?;
        file.write_all(&bytes)
            .map_err(|e| ApiError::storage_failure(format!("Failed to store upload: {e}")))
    }

    async fn import_server(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        upload_id: String,
    ) -> Result<ImportedServer, ApiError> {
        let path = import_upload_path(&app_handle, &upload_id)?;
        let app = app_handle.clone();
        let archive = path.clone();
        let result =
            tauri::async_runtime::spawn_blocking(move || server::transfer::import(&app, &archive))
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?;
        let _ = std::fs::remove_file(&path);
        let created = result.map_err(ApiError::from)?;
        let dir = server::get_server_dir(&app_handle, &created.id).map_err(ApiError::from)?;
        let warnings = server::plugins::restore_missing(&dir, &created.core, &created.game_version).await;
        Ok(ImportedServer {
            server: created,
            warnings,
        })
    }

    async fn duplicate_server(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        name: String,
        game_version: Option<String>,
        build_number: Option<String>,
    ) -> Result<ServerConfig, ApiError> {
        if is_server_running(get_server_process_manager(&app_handle), &server_id).await {
            return Err(ApiError::conflict("Stop the server before copying it"));
        }
        let (_, config) = server_with_config(&app_handle, &server_id)?;
        let app = app_handle.clone();
        tauri::async_runtime::spawn_blocking(move || {
            server::transfer::duplicate(&app, &config, &name, game_version.as_deref(), build_number)
        })
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .map_err(ApiError::from)
    }
}
