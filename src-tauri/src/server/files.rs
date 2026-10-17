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
    #[error("Invalid property key: {0}")]
    InvalidProperty(String),
    #[error("Not an editable config path: {0}")]
    InvalidPath(String),
    #[error("Config file does not exist: {0}")]
    MissingConfig(String),
    #[error("Invalid access-list entry: {0}")]
    InvalidAccessEntry(&'static str),
}

impl From<ServerFileError> for String {
    fn from(error: ServerFileError) -> Self {
        error.to_string()
    }
}

pub(super) fn publish_file(path: &Path, bytes: &[u8]) -> Result<(), ServerFileError> {
    use std::io::Write;
    let parent = path
        .parent()
        .ok_or_else(|| ServerFileError::InvalidPath(path.display().to_string()))?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    staged.write_all(bytes)?;
    staged.as_file().sync_all()?;
    staged
        .persist(path)
        .map_err(|error| ServerFileError::Io(error.error))?;
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
