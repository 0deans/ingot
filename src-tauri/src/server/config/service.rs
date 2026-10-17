pub(crate) use super::error::ConfigError;
use super::{
    models::ServerConfig,
    ports::{check_port, internal_port_for},
};
use std::fs;

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

pub(crate) fn update_server_record(
    repository: &crate::minecraft::metadata::MetadataRepository<ServerConfig>,
    mut config: ServerConfig,
    directory: &std::path::Path,
) -> Result<(), ConfigError> {
    super::creation::validate_server_settings(&config)?;
    use crate::minecraft::metadata::MetadataError;
    repository.update_compensated(
        |records| {
            let index = records
                .iter()
                .position(|record| record.id == config.id)
                .ok_or_else(|| MetadataError::Conflict("Server not found".into()))?;
            let previous = if records[index].port != config.port {
                check_port(records, &config.id, config.port)
                    .map_err(|e| MetadataError::Conflict(e.to_string()))?;
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
                check_port(servers, server_id, port)
                    .map_err(|e| MetadataError::Conflict(e.to_string()))?;
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
