use crate::server::config::creation::{prepare_server_config, ServerCreation};
use crate::server::config::deletion::delete_server_record;
use crate::server::config::error::ConfigError;
use crate::server::config::models::{ServerConfig, ServerCoreType, ServerProperties};
use crate::server::config::ports::check_port;
use crate::server::config::properties::write_server_properties_to_dir;
use crate::server::config::service::update_server_record;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Manager, Runtime};
use uuid::Uuid;

pub fn get_servers_dir<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, ConfigError> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| ConfigError::Directory(format!("Failed to get app data directory: {e}")))?;

    let servers_dir = data_dir.join("servers");
    crate::app::state(app)
        .server_configs
        .with_serialized(|| fs::create_dir_all(&servers_dir))??;
    Ok(servers_dir)
}

pub fn get_server_dir<R: Runtime>(
    app: &tauri::AppHandle<R>,
    server_id: &str,
) -> Result<PathBuf, ConfigError> {
    let servers_dir = get_servers_dir(app)?;
    ensure_server_directory(
        &crate::app::state(app).server_configs,
        &servers_dir,
        server_id,
    )
}

fn ensure_server_directory(
    repository: &crate::minecraft::metadata::MetadataRepository<ServerConfig>,
    parent: &std::path::Path,
    server_id: &str,
) -> Result<PathBuf, ConfigError> {
    repository.with_serialized(|| {
        let directory =
            crate::app::safe_enclosed_filename(parent, server_id).ok_or(ConfigError::InvalidId)?;
        fs::create_dir_all(&directory)?;
        Ok(directory)
    })?
}

pub fn load_servers<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<ServerConfig>, ConfigError> {
    Ok(crate::app::state(app).server_configs.load()?)
}

pub(crate) fn publish_staged_server<R: Runtime>(
    app: &tauri::AppHandle<R>,
    config: ServerConfig,
    staging: &std::path::Path,
) -> Result<ServerConfig, ConfigError> {
    let parent = get_servers_dir(app)?;
    let destination =
        crate::app::safe_enclosed_filename(&parent, &config.id).ok_or(ConfigError::InvalidId)?;
    crate::app::state(app).server_configs.publish(
        staging,
        &destination,
        config.clone(),
        |records, candidate| {
            if records.iter().any(|record| record.id == candidate.id) {
                return Err(crate::minecraft::metadata::MetadataError::Conflict(
                    "Server already exists".into(),
                ));
            }
            check_port(records, &candidate.id, candidate.port)
                .map_err(|e| crate::minecraft::metadata::MetadataError::Conflict(e.to_string()))?;
            Ok(())
        },
    )?;
    Ok(config)
}

fn prepare_server<R: Runtime>(
    app: &tauri::AppHandle<R>,
    request: ServerCreation,
) -> Result<ServerConfig, ConfigError> {
    let servers = crate::app::state(app).server_configs.load()?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| ConfigError::Io(std::io::Error::other(e)))?
        .as_secs();
    prepare_server_config(&servers, request, Uuid::new_v4().to_string(), now)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_unpublished_server<R: Runtime>(
    app: &tauri::AppHandle<R>,
    name: String,
    core: ServerCoreType,
    game_version: String,
    build_number: Option<String>,
    memory_min_mb: Option<u32>,
    memory_max_mb: Option<u32>,
) -> Result<ServerConfig, ConfigError> {
    prepare_server(
        app,
        ServerCreation {
            name,
            core,
            game_version,
            build_number,
            port: None,
            memory_min_mb,
            memory_max_mb,
        },
    )
}

#[allow(clippy::too_many_arguments)]
pub fn create_server<R: Runtime>(
    app: &tauri::AppHandle<R>,
    name: String,
    core: ServerCoreType,
    game_version: String,
    build_number: Option<String>,
    port: Option<u16>,
    memory_min_mb: Option<u32>,
    memory_max_mb: Option<u32>,
) -> Result<ServerConfig, ConfigError> {
    let config = prepare_server(
        app,
        ServerCreation {
            name,
            core,
            game_version,
            build_number,
            port,
            memory_min_mb,
            memory_max_mb,
        },
    )?;
    let parent = get_servers_dir(app)?;
    let stage = tempfile::tempdir_in(&parent)?;
    for directory in ["plugins", "mods", "logs"] {
        fs::create_dir_all(stage.path().join(directory))?;
    }
    fs::write(stage.path().join("eula.txt"), "eula=true\n")?;
    let properties = ServerProperties {
        server_port: config.port,
        motd: format!(
            "\u{00a7}6\u{00a7}l{}\u{00a7}r \u{00a7}7- Ingot Minecraft Server",
            config.name
        ),
        ..Default::default()
    };
    write_server_properties_to_dir(stage.path(), &properties)?;
    publish_staged_server(app, config, stage.path())
}

pub(crate) fn delete_server<R: Runtime>(
    app: &tauri::AppHandle<R>,
    server_id: &str,
    delete_files: bool,
) -> Result<(), ConfigError> {
    let parent = get_servers_dir(app)?;
    let directory =
        crate::app::safe_enclosed_filename(&parent, server_id).ok_or(ConfigError::InvalidId)?;
    delete_server_record(
        &crate::app::state(app).server_configs,
        server_id,
        delete_files.then_some(directory.as_path()),
    )
}

pub(crate) fn update_server<R: Runtime>(
    app: &tauri::AppHandle<R>,
    config: ServerConfig,
) -> Result<(), ConfigError> {
    let parent = get_servers_dir(app)?;
    let directory =
        crate::app::safe_enclosed_filename(&parent, &config.id).ok_or(ConfigError::InvalidId)?;
    update_server_record(&crate::app::state(app).server_configs, config, &directory)
}

#[cfg(test)]
mod tests;
