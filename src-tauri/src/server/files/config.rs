use super::{publish_file, ServerFileError};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

// ─── Other config files ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFile {
    /// Relative to the server folder, with forward slashes
    pub path: String,
    #[specta(type = i32)]
    pub size: f64,
}

const CONFIG_EXTENSIONS: [&str; 7] = ["yml", "yaml", "json", "json5", "toml", "properties", "conf"];

/// Managed through dedicated UI or runtime data - not shown as editable configs
const HIDDEN_FILES: [&str; 10] = [
    "whitelist.json",
    "allowlist.json",
    "ops.json",
    "permissions.json",
    "banned-players.json",
    "banned-ips.json",
    "usercache.json",
    "version_history.json",
    "server.properties",
    "commands.yml",
];

fn is_config_file(path: &Path) -> bool {
    let ext_ok = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| CONFIG_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()));
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    ext_ok && !HIDDEN_FILES.contains(&name)
}

fn collect_configs(
    root: &Path,
    dir: &Path,
    depth: usize,
    out: &mut Vec<ConfigFile>,
) -> Result<(), ServerFileError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(ServerFileError::Io(error)),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        let relative = path
            .strip_prefix(root)
            .map_err(|_| ServerFileError::InvalidPath(path.display().to_string()))?;
        let relative = relative.to_string_lossy().replace('\\', "/");
        if metadata.file_type().is_symlink()
            || crate::app::safe_enclosed_path(root, &relative).is_none()
        {
            return Err(ServerFileError::InvalidPath(relative));
        }
        if metadata.is_dir() {
            if depth > 0 {
                collect_configs(root, &path, depth - 1, out)?;
            }
        } else if is_config_file(&path) && metadata.len() <= 512 * 1024 {
            out.push(ConfigFile {
                path: relative,
                size: metadata.len() as f64,
            });
        }
    }
    Ok(())
}

pub fn list_config_files(server_dir: &Path) -> Result<Vec<ConfigFile>, ServerFileError> {
    let mut files = Vec::new();
    collect_configs(server_dir, server_dir, 0, &mut files)?;
    collect_configs(server_dir, &server_dir.join("config"), 3, &mut files)?;
    collect_configs(server_dir, &server_dir.join("plugins"), 2, &mut files)?;
    files.sort_by(|a, b| {
        a.path
            .matches('/')
            .count()
            .cmp(&b.path.matches('/').count())
            .then_with(|| a.path.cmp(&b.path))
    });
    Ok(files)
}

/// Resolves a relative config path, refusing anything outside the server folder
pub(super) fn config_path(server_dir: &Path, rel: &str) -> Result<PathBuf, ServerFileError> {
    let path = crate::app::safe_enclosed_path(server_dir, rel)
        .ok_or_else(|| ServerFileError::InvalidPath(rel.into()))?;
    if !is_config_file(&path) {
        return Err(ServerFileError::InvalidPath(rel.into()));
    }
    Ok(path)
}

pub fn read_config_file(server_dir: &Path, rel: &str) -> Result<String, ServerFileError> {
    fs::read_to_string(config_path(server_dir, rel)?).map_err(ServerFileError::Io)
}

pub fn write_config_file(
    server_dir: &Path,
    rel: &str,
    content: &str,
) -> Result<(), ServerFileError> {
    let path = config_path(server_dir, rel)?;
    if !path.exists() {
        return Err(ServerFileError::MissingConfig(rel.into()));
    }
    publish_file(&path, content.as_bytes())
}
