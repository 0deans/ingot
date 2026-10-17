use super::models::{ServerConfig, ServerCoreType, ServerProperties};
use super::ports::{check_port, free_port_for_core, internal_port_for};
use super::properties::write_server_properties_to_dir;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Manager, Runtime};
use uuid::Uuid;

pub fn get_servers_dir<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {e}"))?;

    let servers_dir = data_dir.join("servers");
    if !servers_dir.exists() {
        fs::create_dir_all(&servers_dir)
            .map_err(|e| format!("Failed to create servers directory: {e}"))?;
    }
    Ok(servers_dir)
}

pub fn get_server_dir<R: Runtime>(
    app: &tauri::AppHandle<R>,
    server_id: &str,
) -> Result<PathBuf, String> {
    let servers_dir = get_servers_dir(app)?;
    let server_dir = crate::app::safe_enclosed_filename(&servers_dir, server_id)
        .ok_or_else(|| "Invalid server identifier".to_string())?;
    if !server_dir.exists() {
        fs::create_dir_all(&server_dir)
            .map_err(|e| format!("Failed to create server directory: {e}"))?;
    }
    Ok(server_dir)
}

pub fn load_servers<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<Vec<ServerConfig>, String> {
    crate::app::state(app)
        .server_configs
        .load()
        .map_err(|error| error.to_string())
}

pub(crate) fn publish_staged_server<R: Runtime>(
    app: &tauri::AppHandle<R>,
    config: ServerConfig,
    staging: &std::path::Path,
) -> Result<ServerConfig, String> {
    let parent = get_servers_dir(app)?;
    let destination = crate::app::safe_enclosed_filename(&parent, &config.id)
        .ok_or_else(|| "Invalid server identifier".to_string())?;
    crate::app::state(app)
        .server_configs
        .publish(
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
                    .map_err(crate::minecraft::metadata::MetadataError::Conflict)?;
                Ok(())
            },
        )
        .map_err(|error| error.to_string())?;
    Ok(config)
}

#[allow(clippy::too_many_arguments)]
fn prepare_server<R: Runtime>(
    app: &tauri::AppHandle<R>,
    name: String,
    core: ServerCoreType,
    game_version: String,
    build_number: Option<String>,
    port: Option<u16>,
    memory_min_mb: Option<u32>,
    memory_max_mb: Option<u32>,
) -> Result<ServerConfig, String> {
    let servers = load_servers(app)?;
    let id = Uuid::new_v4().to_string();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let selected_port = match port {
        Some(p) => {
            check_port(&servers, &id, p)?;
            p
        }
        None => free_port_for_core(&servers, &id, &core),
    };

    let is_bedrock = core == ServerCoreType::Bedrock;
    let config = ServerConfig {
        id: id.clone(),
        name,
        core,
        game_version,
        build_number,
        port: selected_port,
        memory_min_mb: memory_min_mb.unwrap_or(if is_bedrock { 1024 } else { 2048 }),
        memory_max_mb: memory_max_mb.unwrap_or(if is_bedrock { 2048 } else { 4096 }),
        icon: None,
        java_path: None,
        jvm_args: None,
        auto_start: Some(false),
        sleep_enabled: Some(!is_bedrock),
        idle_timeout_seconds: Some(600),
        internal_port: Some(internal_port_for(selected_port)),
        playit_enabled: Some(false),
        playit_secret_key: None,
        created_at: now,
        last_run_at: None,
    };

    Ok(config)
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
) -> Result<ServerConfig, String> {
    prepare_server(
        app,
        name,
        core,
        game_version,
        build_number,
        None,
        memory_min_mb,
        memory_max_mb,
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
) -> Result<ServerConfig, String> {
    let config = prepare_server(
        app,
        name,
        core,
        game_version,
        build_number,
        port,
        memory_min_mb,
        memory_max_mb,
    )?;
    let parent = get_servers_dir(app)?;
    let stage = tempfile::tempdir_in(&parent).map_err(|error| error.to_string())?;
    for directory in ["plugins", "mods", "logs"] {
        fs::create_dir_all(stage.path().join(directory)).map_err(|error| error.to_string())?;
    }
    fs::write(stage.path().join("eula.txt"), "eula=true\n").map_err(|error| error.to_string())?;
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

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConfigError {
    #[error(transparent)]
    Metadata(#[from] crate::minecraft::metadata::MetadataError),
    #[error("Server file I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid server identifier")]
    InvalidId,
    #[error("Server metadata was deleted, but files remain at {path}: {source}")]
    FilesCleanupCommitted {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Server directory resolution failed: {0}")]
    Directory(String),
    #[error(transparent)]
    File(#[from] crate::server::files::ServerFileError),
    #[error("Invalid server port")]
    InvalidPort,
}
impl From<ConfigError> for String {
    fn from(error: ConfigError) -> Self {
        error.to_string()
    }
}

pub(crate) fn delete_server<R: Runtime>(
    app: &tauri::AppHandle<R>,
    server_id: &str,
    delete_files: bool,
) -> Result<(), ConfigError> {
    let parent = get_servers_dir(app).map_err(ConfigError::Directory)?;
    let directory =
        crate::app::safe_enclosed_filename(&parent, server_id).ok_or(ConfigError::InvalidId)?;
    delete_server_record(
        &crate::app::state(app).server_configs,
        server_id,
        delete_files.then_some(directory.as_path()),
    )
}

fn metadata_io(
    operation: &'static str,
    path: &std::path::Path,
    source: std::io::Error,
) -> crate::minecraft::metadata::MetadataError {
    crate::minecraft::metadata::MetadataError::Io {
        operation,
        path: path.to_owned(),
        source,
    }
}

fn delete_server_record(
    repository: &crate::minecraft::metadata::MetadataRepository<ServerConfig>,
    server_id: &str,
    directory: Option<&std::path::Path>,
) -> Result<(), ConfigError> {
    use crate::minecraft::metadata::MetadataError;
    let retained = repository.update_compensated(
        |records| {
            let index = records
                .iter()
                .position(|record| record.id == server_id)
                .ok_or_else(|| MetadataError::Conflict("Server not found".into()))?;
            let retained = if let Some(directory) = directory {
                if directory
                    .try_exists()
                    .map_err(|error| metadata_io("inspect server directory", directory, error))?
                {
                    let parent = directory.parent().ok_or_else(|| {
                        MetadataError::Conflict("Server directory has no parent".into())
                    })?;
                    let backup = parent.join(format!(".deleted-{server_id}-{}", Uuid::new_v4()));
                    fs::rename(directory, &backup).map_err(|error| {
                        metadata_io("preserve server directory", directory, error)
                    })?;
                    Some((directory.to_owned(), backup))
                } else {
                    None
                }
            } else {
                None
            };
            records.remove(index);
            Ok(retained)
        },
        |retained| {
            if let Some((original, backup)) = retained {
                fs::rename(backup, original).map_err(|error| {
                    metadata_io("restore deleted server directory", original, error)
                })?;
            }
            Ok(())
        },
    )?;
    if let Some((_, backup)) = retained {
        fs::remove_dir_all(&backup).map_err(|source| ConfigError::FilesCleanupCommitted {
            path: backup,
            source,
        })?;
    }
    Ok(())
}

pub(crate) fn update_server<R: Runtime>(
    app: &tauri::AppHandle<R>,
    config: ServerConfig,
) -> Result<(), ConfigError> {
    let parent = get_servers_dir(app).map_err(ConfigError::Directory)?;
    let directory =
        crate::app::safe_enclosed_filename(&parent, &config.id).ok_or(ConfigError::InvalidId)?;
    update_server_record(&crate::app::state(app).server_configs, config, &directory)
}

fn update_server_record(
    repository: &crate::minecraft::metadata::MetadataRepository<ServerConfig>,
    mut config: ServerConfig,
    directory: &std::path::Path,
) -> Result<(), ConfigError> {
    use crate::minecraft::metadata::MetadataError;
    repository.update_compensated(
        |records| {
            let index = records
                .iter()
                .position(|record| record.id == config.id)
                .ok_or_else(|| MetadataError::Conflict("Server not found".into()))?;
            let previous = if records[index].port != config.port {
                check_port(records, &config.id, config.port).map_err(MetadataError::Conflict)?;
                let path = directory.join("server.properties");
                let bytes = match fs::read(&path) {
                    Ok(bytes) => Some(bytes),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                    Err(error) => {
                        return Err(metadata_io("read previous properties", &path, error))
                    }
                };
                crate::server::files::write_properties(
                    directory,
                    &[crate::server::files::PropertyEntry {
                        key: "server-port".into(),
                        value: config.port.to_string(),
                    }],
                )
                .map_err(|error| {
                    metadata_io("publish properties", &path, std::io::Error::other(error))
                })?;
                config.internal_port = Some(internal_port_for(config.port));
                Some((path, bytes))
            } else {
                None
            };
            records[index] = config;
            Ok(previous)
        },
        restore_properties,
    )?;
    Ok(())
}

fn restore_properties(
    previous: &Option<(std::path::PathBuf, Option<Vec<u8>>)>,
) -> Result<(), crate::minecraft::metadata::MetadataError> {
    use std::io::Write;
    if let Some((path, bytes)) = previous {
        if let Some(bytes) = bytes {
            let directory = path.parent().ok_or_else(|| {
                crate::minecraft::metadata::MetadataError::Conflict(
                    "Missing properties directory".into(),
                )
            })?;
            let mut stage = tempfile::NamedTempFile::new_in(directory)
                .map_err(|error| metadata_io("stage previous properties", path, error))?;
            stage
                .write_all(bytes)
                .map_err(|error| metadata_io("restore previous properties", path, error))?;
            stage
                .as_file()
                .sync_all()
                .map_err(|error| metadata_io("sync previous properties", path, error))?;
            stage
                .persist(path)
                .map_err(|error| metadata_io("replace previous properties", path, error.error))?;
        } else {
            fs::remove_file(path)
                .map_err(|error| metadata_io("remove new properties", path, error))?;
        }
    }
    Ok(())
}

pub(crate) fn update_properties(
    repository: &crate::minecraft::metadata::MetadataRepository<ServerConfig>,
    server_id: &str,
    directory: &std::path::Path,
    entries: &[crate::server::files::PropertyEntry],
) -> Result<(), ConfigError> {
    use crate::minecraft::metadata::MetadataError;
    crate::server::files::validate_updates(entries)?;
    let port = entries
        .iter()
        .find(|entry| entry.key == "server-port")
        .map(|entry| {
            entry
                .value
                .trim()
                .parse::<u16>()
                .map_err(|_| ConfigError::InvalidPort)
        })
        .transpose()?;
    repository.update_compensated(
        |servers| {
            let index = servers
                .iter()
                .position(|server| server.id == server_id)
                .ok_or_else(|| MetadataError::Conflict("Server not found".into()))?;
            if let Some(port) = port {
                check_port(servers, server_id, port).map_err(MetadataError::Conflict)?;
            }
            let path = directory.join("server.properties");
            let previous = match fs::read(&path) {
                Ok(bytes) => Some(bytes),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(metadata_io("read previous properties", &path, error)),
            };
            crate::server::files::write_properties(directory, entries).map_err(|error| {
                metadata_io("publish properties", &path, std::io::Error::other(error))
            })?;
            if let Some(port) = port {
                servers[index].port = port;
                servers[index].internal_port = Some(internal_port_for(port));
            }
            Ok(Some((path, previous)))
        },
        restore_properties,
    )?;
    Ok(())
}
