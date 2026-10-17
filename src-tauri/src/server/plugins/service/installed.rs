use super::super::{
    metadata::{read_jar_metadata, safe_jar_name},
    models::{platform, InstalledPlugin, PluginError},
    tracking::{read_tracking, write_tracking},
};
use super::publication::publication_error;
use crate::server::config::ServerCoreType;
use std::path::{Path, PathBuf};
pub fn folder(server_dir: &Path, core: &ServerCoreType) -> Result<PathBuf, PluginError> {
    let platform = platform(core).ok_or(PluginError::UnsupportedCore)?;
    Ok(server_dir.join(platform.folder))
}

pub fn list_installed(
    server_dir: &Path,
    core: &ServerCoreType,
) -> Result<Vec<InstalledPlugin>, PluginError> {
    let dir = folder(server_dir, core)?;
    let tracked = read_tracking(server_dir)?;
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(PluginError::Io(error)),
    };
    let mut plugins = Vec::new();
    for entry in entries {
        let entry = entry?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if safe_jar_name(&file_name).is_err() {
            continue;
        }
        let metadata = entry.metadata()?;
        if !metadata.is_file() {
            continue;
        }
        let enabled = !file_name.ends_with(".disabled");
        let jar_name = file_name.trim_end_matches(".disabled");
        let track = tracked.iter().find(|item| item.file_name == jar_name);
        // Third-party JARs need not contain recognised optional display metadata.
        let (name, version, description, authors) = read_jar_metadata(&entry.path())
            .unwrap_or_else(|| {
                (
                    jar_name.trim_end_matches(".jar").to_owned(),
                    None,
                    None,
                    Vec::new(),
                )
            });
        plugins.push(InstalledPlugin {
            file_name,
            enabled,
            size: metadata.len().min(u32::MAX as u64) as u32,
            version: version.or_else(|| track.map(|item| item.version_number.clone())),
            description,
            authors,
            source: track.map(|item| item.source),
            project_id: track.map(|item| item.project_id.clone()),
            icon_url: track.and_then(|item| item.icon_url.clone()),
            system: name == crate::server::companion::PLUGIN_NAME,
            name,
        });
    }
    plugins.sort_by_key(|p| p.name.to_lowercase());
    Ok(plugins)
}

pub fn set_enabled(
    server_dir: &Path,
    core: &ServerCoreType,
    file_name: &str,
    enabled: bool,
) -> Result<(), PluginError> {
    let dir = folder(server_dir, core)?;
    let file_name = safe_jar_name(file_name)?;
    let jar = file_name.trim_end_matches(".disabled");
    let (from, to) = if enabled {
        (format!("{jar}.disabled"), jar.to_string())
    } else {
        (jar.to_string(), format!("{jar}.disabled"))
    };
    if dir.join(&to).exists() {
        return Ok(());
    }
    let from = crate::app::safe_enclosed_filename(&dir, &from)
        .ok_or_else(|| PluginError::InvalidJarName(from))?;
    let to = crate::app::safe_enclosed_filename(&dir, &to)
        .ok_or_else(|| PluginError::InvalidJarName(to))?;
    std::fs::rename(from, to).map_err(PluginError::Io)
}

pub fn remove(
    server_dir: &Path,
    core: &ServerCoreType,
    file_name: &str,
) -> Result<(), PluginError> {
    let dir = folder(server_dir, core)?;
    let file_name = safe_jar_name(file_name)?;
    let mut tracked = read_tracking(server_dir)?;
    let path = crate::app::safe_enclosed_filename(&dir, file_name)
        .ok_or_else(|| PluginError::InvalidJarName(file_name.into()))?;
    let backup = dir.join(format!(".removed-{}", uuid::Uuid::new_v4()));
    std::fs::rename(&path, &backup)?;
    let jar = file_name.trim_end_matches(".disabled");
    tracked.retain(|item| item.file_name != jar);
    if let Err(primary) = write_tracking(server_dir, &tracked) {
        return Err(publication_error(
            PluginError::Tracking(primary),
            &[(path, backup)],
        ));
    }
    std::fs::remove_file(&backup).map_err(|source| PluginError::Cleanup {
        path: backup,
        source,
    })?;
    Ok(())
}
