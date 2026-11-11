//! Server configuration files: every server.properties key, other config files
//! (bukkit/spigot/paper YAML, plugin configs...) and the access lists
//! (whitelist, operators, bans).

use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum ServerFileError {
    #[error("Server file I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid server JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid base64 icon data: {0}")]
    InvalidIconEncoding(#[source] base64::DecodeError),
    #[error("Invalid property key: {0}")]
    InvalidProperty(String),
    #[error("Not an editable config path: {0}")]
    InvalidPath(String),
    #[error("Config file does not exist: {0}")]
    MissingConfig(String),
    #[error("Invalid access-list entry: {0}")]
    InvalidAccessEntry(&'static str),
}

pub(super) fn publish_file(path: &Path, bytes: &[u8]) -> Result<(), ServerFileError> {
    if path.parent().is_none() {
        return Err(ServerFileError::InvalidPath(path.display().to_string()));
    }
    ingot_core::write_file_atomic(path, bytes)?;
    Ok(())
}

mod access;
mod config;
mod properties;
pub(crate) use properties::validate_updates;

pub use access::{
    add_access_entry, read_access_list, remove_access_entry, AccessEntry, AccessListKind,
};
pub use config::{list_config_files, read_config_file, write_config_file, ConfigFile};
pub use properties::{
    escape_property, read_properties, unescape_property, write_properties, PropertyEntry,
};

#[cfg(test)]
mod tests;
