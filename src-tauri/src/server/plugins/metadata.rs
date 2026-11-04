use super::models::{PluginError, RawJarMetadata};
use std::path::Path;

/// Only plain jar names inside the plugins/mods folder
pub fn safe_jar_name(name: &str) -> Result<&str, PluginError> {
    let base = name.strip_suffix(".disabled").unwrap_or(name);
    let plain = Path::new(name).file_name().and_then(|f| f.to_str()) == Some(name);
    if !plain || !base.to_ascii_lowercase().ends_with(".jar") || name.starts_with('.') {
        return Err(PluginError::InvalidJarName(name.to_string()));
    }
    Ok(name)
}

/// name/version/description/authors from plugin.yml, paper-plugin.yml, fabric.mod.json,
/// quilt.mod.json or (neoforge.)mods.toml
pub(crate) fn read_jar_metadata(path: &Path) -> Option<RawJarMetadata> {
    crate::minecraft::instance_content::metadata::read_raw_jar_metadata(path)
}
