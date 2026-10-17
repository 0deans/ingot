//! Moving servers around: export to a zip (configs only, or everything including
//! worlds), import such a zip as a new server, duplicate, and change version.

use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::Runtime;

use super::config::{self, ServerConfig, ServerCoreType};
use super::files::PropertyEntry;

const MANIFEST: &str = "ingot-server.json";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ExportMode {
    /// server.properties, plugin/mod configs, whitelist, ops, icon - no worlds or jars
    Configs,
    /// Everything needed to run it elsewhere, worlds and plugins included
    Full,
}

/// An imported server, and plugins/mods that couldn't be downloaded again
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportedServer {
    pub server: ServerConfig,
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum TransferError {
    #[error("I/O error during server transfer: {0}")]
    Io(#[from] std::io::Error),

    #[error("Archive error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Archive extraction error: {0}")]
    Archive(#[from] crate::app::ArchiveError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Server file error: {0}")]
    File(#[from] super::files::ServerFileError),

    #[error("Path prefix error: {0}")]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error("Installer error: {0}")]
    Installer(#[from] super::installer::InstallerError),

    #[error("Invalid export path encoding")]
    InvalidPathEncoding,

    #[error("This is not a valid zip file")]
    InvalidZip,

    #[error("This zip wasn't exported from Ingot (no ingot-server.json)")]
    NotAnIngotExport,

    #[error("Invalid ingot-server.json: {0}")]
    InvalidManifest(String),

    #[error("Archive destination has no parent")]
    MissingParent,

    #[error("Server transfer contains an unsupported link or special entry")]
    UnsupportedEntry,

    #[error("Failed to copy file to {}: {source}", path.display())]
    CopyFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// What an export carries about the server, so an import can recreate it
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    format: u32,
    name: String,
    core: ServerCoreType,
    game_version: String,
    build_number: Option<String>,
    memory_min_mb: u32,
    memory_max_mb: u32,
    mode: ExportMode,
}

/// Regenerated automatically, machine-specific, or huge and re-downloadable
const SKIP_DIRS: [&str; 7] = [
    "cache",
    "libraries",
    "versions",
    "logs",
    "crash-reports",
    "debug",
    ".ingot",
];
const SKIP_FILES: [&str; 5] = [
    "server.jar",
    "bedrock_server.exe",
    "bedrock_server",
    "session.lock",
    "usercache.json",
];
/// Written by the Forge, NeoForge and Quilt installers, which run again after an import
const INSTALLER_FILES: [&str; 5] = [
    "quilt-server-launch.jar",
    "run.bat",
    "run.sh",
    "user_jvm_args.txt",
    "README.txt",
];
/// The one file kept from .ingot: which plugins/mods came from Modrinth or Hangar (for
/// updates, and to download them again when an export has no jars)
const TRACKING: &str = ".ingot/plugins.json";

/// Top-level files that are part of the loader install (see INSTALLER_FILES)
fn is_installer_file(name: &str) -> bool {
    INSTALLER_FILES.contains(&name) || (name.starts_with("forge-") && name.ends_with("-shim.jar"))
}

fn is_world_dir(dir: &Path) -> bool {
    dir.join("level.dat").exists()
        || dir.join("region").is_dir()
        || dir.join("levelname.txt").exists()
        || dir
            .file_name()
            .is_some_and(|n| n.eq_ignore_ascii_case("worlds"))
}

fn is_config_file(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    // Scripts, quests and old-style configs are how mods are usually set up
    [
        ".yml",
        ".yaml",
        ".json",
        ".json5",
        ".toml",
        ".properties",
        ".conf",
        ".txt",
        ".cfg",
        ".ini",
        ".snbt",
        ".js",
        ".zs",
    ]
    .iter()
    .any(|ext| lower.ends_with(ext))
        || lower == "server-icon.png"
}

/// Relative paths (forward slashes) of everything to put in the archive
fn collect(
    root: &Path,
    dir: &Path,
    mode: ExportMode,
    out: &mut Vec<PathBuf>,
) -> Result<(), TransferError> {
    let entries = fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let kind = entry.file_type()?;
        let relative = path.strip_prefix(root)?;
        let relative = relative
            .to_str()
            .ok_or(TransferError::InvalidPathEncoding)?;
        if crate::app::safe_enclosed_path(root, relative).is_none()
            || (!kind.is_dir() && !kind.is_file())
        {
            return Err(TransferError::UnsupportedEntry);
        }
        if kind.is_dir() {
            let top_level = dir == root;
            if top_level && name == ".ingot" && root.join(TRACKING).is_file() {
                out.push(PathBuf::from(TRACKING));
            }
            if top_level && SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            if mode == ExportMode::Configs && is_world_dir(&path) {
                continue;
            }
            collect(root, &path, mode, out)?;
        } else if !SKIP_FILES.contains(&name.as_str())
            && !name.ends_with(".part")
            && !(dir == root && is_installer_file(&name))
        {
            let keep = match mode {
                ExportMode::Full => true,
                ExportMode::Configs => is_config_file(&name),
            };
            if keep {
                if let Ok(rel) = path.strip_prefix(root) {
                    out.push(rel.to_path_buf());
                }
            }
        }
    }
    Ok(())
}

/// Writes the archive and returns its path
pub fn export(
    server_dir: &Path,
    server: &ServerConfig,
    mode: ExportMode,
    dest: &Path,
) -> Result<PathBuf, TransferError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut files = Vec::new();
    collect(server_dir, server_dir, mode, &mut files)?;

    let parent = dest.parent().ok_or(TransferError::MissingParent)?;
    let tmp = tempfile::NamedTempFile::new_in(parent)?;
    let result = (|| -> Result<(), TransferError> {
        let mut zip = zip::ZipWriter::new(tmp.reopen()?);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .large_file(true);

        let manifest = Manifest {
            format: 1,
            name: server.name.clone(),
            core: server.core.clone(),
            game_version: server.game_version.clone(),
            build_number: server.build_number.clone(),
            memory_min_mb: server.memory_min_mb,
            memory_max_mb: server.memory_max_mb,
            mode,
        };
        zip.start_file(MANIFEST, options)?;
        zip.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;

        for rel in files {
            let name = rel.to_string_lossy().replace('\\', "/");
            let mut source = File::open(server_dir.join(&rel))?;
            zip.start_file(name, options)?;
            std::io::copy(&mut source, &mut zip)?;
        }
        zip.finish()?.sync_all()?;
        Ok(())
    })();
    result?;
    tmp.persist(dest).map_err(|e| TransferError::Io(e.error))?;
    Ok(dest.to_path_buf())
}

/// A file name for the archive, e.g. "My_Server-26.2-full.zip"
pub fn export_file_name(server: &ServerConfig, mode: ExportMode) -> String {
    let safe: String = server
        .name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let kind = if mode == ExportMode::Full {
        "full"
    } else {
        "configs"
    };
    format!(
        "{}-{}-{kind}.zip",
        safe.trim_matches('_'),
        server.game_version
    )
}

/// Sets server-port in server.properties (copies/imports get their own port)
fn set_port(server_dir: &Path, port: u16) -> Result<(), TransferError> {
    super::files::write_properties(
        server_dir,
        &[
            PropertyEntry {
                key: "server-port".into(),
                value: port.to_string(),
            },
            PropertyEntry {
                key: "query.port".into(),
                value: port.to_string(),
            },
        ],
    )?;
    Ok(())
}

/// Creates a new server from an exported archive
pub fn import<R: Runtime>(
    app: &tauri::AppHandle<R>,
    archive: &Path,
) -> Result<ServerConfig, TransferError> {
    let file = File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|_| TransferError::InvalidZip)?;
    let manifest: Manifest = {
        let mut entry = zip
            .by_name(MANIFEST)
            .map_err(|_| TransferError::NotAnIngotExport)?;
        let mut raw = String::new();
        entry.read_to_string(&mut raw)?;
        serde_json::from_str(&raw).map_err(|e| TransferError::InvalidManifest(e.to_string()))?
    };

    // Keep names distinguishable when importing a server that already exists here
    let taken = config::load_servers(app)?
        .iter()
        .any(|s| s.name == manifest.name);
    let name = if taken {
        format!("{} (imported)", manifest.name)
    } else {
        manifest.name.clone()
    };
    let created = config::build_unpublished_server(
        app,
        name,
        manifest.core.clone(),
        manifest.game_version.clone(),
        manifest.build_number.clone(),
        Some(manifest.memory_min_mb),
        Some(manifest.memory_max_mb),
    )?;
    let servers_dir = config::get_servers_dir(app)?;
    let staging = tempfile::tempdir_in(&servers_dir)?;
    crate::app::extract_zip_entries(&mut zip, staging.path(), "", Some(MANIFEST))?;
    set_port(staging.path(), created.port)?;
    Ok(config::publish_staged_server(app, created, staging.path())?)
}

fn copy_dir(from: &Path, to: &Path, skip_jar: bool) -> Result<(), TransferError> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let kind = entry.file_type()?;
        if crate::app::safe_enclosed_filename(from, &name_str).is_none()
            || (!kind.is_dir() && !kind.is_file())
        {
            return Err(TransferError::UnsupportedEntry);
        }
        if name_str == "session.lock" || (skip_jar && name_str == "server.jar") {
            continue;
        }
        let dest = to.join(&name);
        if kind.is_dir() {
            // Only skip regenerated folders at the top level of the server
            if from.join("server.properties").exists()
                && (name_str == "cache" || name_str == "logs" || name_str == ".ingot")
            {
                if name_str == ".ingot" && from.join(TRACKING).is_file() {
                    fs::create_dir_all(&dest)?;
                    fs::copy(from.join(TRACKING), to.join(TRACKING))?;
                }
                continue;
            }
            copy_dir(&entry.path(), &dest, false)?;
        } else {
            fs::copy(entry.path(), &dest).map_err(|e| TransferError::CopyFailed {
                path: dest,
                source: e,
            })?;
        }
    }
    Ok(())
}

/// Copies a server (worlds, plugins, configs) into a new one, optionally on another
/// Minecraft version so the original stays untouched as a fallback
pub fn duplicate<R: Runtime>(
    app: &tauri::AppHandle<R>,
    source: &ServerConfig,
    name: &str,
    game_version: Option<&str>,
    build_number: Option<String>,
) -> Result<ServerConfig, TransferError> {
    let version_changes = game_version.is_some_and(|v| v != source.game_version);
    let mut created = config::build_unpublished_server(
        app,
        name.to_string(),
        source.core.clone(),
        game_version.unwrap_or(&source.game_version).to_string(),
        if version_changes {
            build_number
        } else {
            source.build_number.clone()
        },
        Some(source.memory_min_mb),
        Some(source.memory_max_mb),
    )?;
    let from = config::get_server_dir(app, &source.id)?;
    let parent = config::get_servers_dir(app)?;
    let staging = tempfile::tempdir_in(parent)?;
    let to = staging.path();
    copy_dir(&from, to, version_changes)?;
    if version_changes {
        // Forge, NeoForge and Quilt install themselves again for the new version
        super::installer::uninstall(to, &created.core)?;
    }
    set_port(to, created.port)?;

    created.jvm_args = source.jvm_args.clone();
    created.java_path = source.java_path.clone();
    created.sleep_enabled = source.sleep_enabled;
    created.idle_timeout_seconds = source.idle_timeout_seconds;
    Ok(config::publish_staged_server(app, created, staging.path())?)
}

#[cfg(test)]
mod tests;
