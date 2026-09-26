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
const SKIP_DIRS: [&str; 7] = ["cache", "libraries", "versions", "logs", "crash-reports", "debug", ".ingot"];
const SKIP_FILES: [&str; 3] = ["server.jar", "session.lock", "usercache.json"];

fn is_world_dir(dir: &Path) -> bool {
    dir.join("level.dat").exists() || dir.join("region").is_dir()
}

fn is_config_file(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [".yml", ".yaml", ".json", ".json5", ".toml", ".properties", ".conf", ".txt"]
        .iter()
        .any(|ext| lower.ends_with(ext))
        || lower == "server-icon.png"
}

/// Relative paths (forward slashes) of everything to put in the archive
fn collect(root: &Path, dir: &Path, mode: ExportMode, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(kind) = entry.file_type() else { continue };
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            let top_level = dir == root;
            if top_level && SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            if mode == ExportMode::Configs && is_world_dir(&path) {
                continue;
            }
            collect(root, &path, mode, out);
        } else if !SKIP_FILES.contains(&name.as_str()) && !name.ends_with(".part") {
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
}

/// Writes the archive and returns its path
pub fn export(server_dir: &Path, server: &ServerConfig, mode: ExportMode, dest: &Path) -> Result<PathBuf, String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create {}: {e}", parent.display()))?;
    }
    let mut files = Vec::new();
    collect(server_dir, server_dir, mode, &mut files);

    let tmp = dest.with_extension("zip.part");
    let result = (|| -> Result<(), String> {
        let mut zip = zip::ZipWriter::new(File::create(&tmp).map_err(|e| e.to_string())?);
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
        zip.start_file(MANIFEST, options).map_err(|e| e.to_string())?;
        zip.write_all(serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?.as_bytes())
            .map_err(|e| e.to_string())?;

        for rel in files {
            let name = rel.to_string_lossy().replace('\\', "/");
            // Files can vanish or be locked while the server runs; skip those
            let Ok(mut source) = File::open(server_dir.join(&rel)) else { continue };
            zip.start_file(name, options).map_err(|e| e.to_string())?;
            std::io::copy(&mut source, &mut zip).map_err(|e| e.to_string())?;
        }
        zip.finish().map_err(|e| e.to_string())?;
        Ok(())
    })();
    if let Err(e) = result {
        let _ = fs::remove_file(&tmp);
        return Err(format!("Export failed: {e}"));
    }
    fs::rename(&tmp, dest).map_err(|e| format!("Export failed: {e}"))?;
    Ok(dest.to_path_buf())
}

/// A file name for the archive, e.g. "My_Server-26.2-full.zip"
pub fn export_file_name(server: &ServerConfig, mode: ExportMode) -> String {
    let safe: String = server
        .name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
        .collect();
    let kind = if mode == ExportMode::Full { "full" } else { "configs" };
    format!("{}-{}-{kind}.zip", safe.trim_matches('_'), server.game_version)
}

/// Sets server-port in server.properties (copies/imports get their own port)
fn set_port(server_dir: &Path, port: u16) -> Result<(), String> {
    super::files::write_properties(
        server_dir,
        &[
            PropertyEntry { key: "server-port".into(), value: port.to_string() },
            PropertyEntry { key: "query.port".into(), value: port.to_string() },
        ],
    )
}

/// Creates a new server from an exported archive
pub fn import<R: Runtime>(app: &tauri::AppHandle<R>, archive: &Path) -> Result<ServerConfig, String> {
    let file = File::open(archive).map_err(|e| format!("Failed to open archive: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|_| "This is not a valid zip file".to_string())?;
    let manifest: Manifest = {
        let mut entry = zip
            .by_name(MANIFEST)
            .map_err(|_| "This zip wasn't exported from Ingot (no ingot-server.json)".to_string())?;
        let mut raw = String::new();
        entry.read_to_string(&mut raw).map_err(|e| e.to_string())?;
        serde_json::from_str(&raw).map_err(|e| format!("Invalid ingot-server.json: {e}"))?
    };

    // Keep names distinguishable when importing a server that already exists here
    let taken = config::load_servers(app)?.iter().any(|s| s.name == manifest.name);
    let name = if taken { format!("{} (imported)", manifest.name) } else { manifest.name.clone() };
    let created = config::create_server(
        app,
        name,
        manifest.core.clone(),
        manifest.game_version.clone(),
        manifest.build_number.clone(),
        None,
        Some(manifest.memory_min_mb),
        Some(manifest.memory_max_mb),
    )?;
    let dir = config::get_server_dir(app, &created.id)?;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        // enclosed_name rejects absolute paths and "..", so entries stay inside the folder
        let Some(rel) = entry.enclosed_name() else { continue };
        if rel == Path::new(MANIFEST) {
            continue;
        }
        let out = dir.join(rel);
        if entry.is_dir() {
            let _ = fs::create_dir_all(&out);
            continue;
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut target = File::create(&out).map_err(|e| format!("Failed to write {}: {e}", out.display()))?;
        std::io::copy(&mut entry, &mut target).map_err(|e| e.to_string())?;
    }
    set_port(&dir, created.port)?;
    Ok(created)
}

fn copy_dir(from: &Path, to: &Path, skip_jar: bool) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(from).map_err(|e| e.to_string())?.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let Ok(kind) = entry.file_type() else { continue };
        if kind.is_symlink() || name_str == "session.lock" || (skip_jar && name_str == "server.jar") {
            continue;
        }
        let dest = to.join(&name);
        if kind.is_dir() {
            // Only skip regenerated folders at the top level of the server
            if from.join("server.properties").exists() && (name_str == "cache" || name_str == "logs" || name_str == ".ingot") {
                continue;
            }
            copy_dir(&entry.path(), &dest, false)?;
        } else {
            fs::copy(entry.path(), &dest).map_err(|e| format!("Failed to copy {name_str}: {e}"))?;
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
) -> Result<ServerConfig, String> {
    let version_changes = game_version.is_some_and(|v| v != source.game_version);
    let mut created = config::create_server(
        app,
        name.to_string(),
        source.core.clone(),
        game_version.unwrap_or(&source.game_version).to_string(),
        if version_changes { build_number } else { source.build_number.clone() },
        None,
        Some(source.memory_min_mb),
        Some(source.memory_max_mb),
    )?;
    let from = config::get_server_dir(app, &source.id)?;
    let to = config::get_server_dir(app, &created.id)?;
    copy_dir(&from, &to, version_changes)?;
    set_port(&to, created.port)?;

    created.jvm_args = source.jvm_args.clone();
    created.java_path = source.java_path.clone();
    created.sleep_enabled = source.sleep_enabled;
    created.idle_timeout_seconds = source.idle_timeout_seconds;
    config::update_server(app, created.clone())?;
    Ok(created)
}

/// Switches a server to another version in place: the next start downloads the new
/// server jar and Minecraft upgrades the world (which can't be undone)
pub fn change_version<R: Runtime>(
    app: &tauri::AppHandle<R>,
    server: &ServerConfig,
    game_version: &str,
    build_number: Option<String>,
) -> Result<ServerConfig, String> {
    let dir = config::get_server_dir(app, &server.id)?;
    let jar = dir.join("server.jar");
    if jar.exists() {
        fs::remove_file(&jar).map_err(|e| format!("Failed to remove old server.jar: {e}"))?;
    }
    let mut updated = server.clone();
    updated.game_version = game_version.to_string();
    updated.build_number = build_number;
    config::update_server(app, updated.clone())?;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configs_export_skips_worlds_and_jars() {
        let root = std::env::temp_dir().join(format!("ingot-export-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for dir in ["world/region", "plugins/LuckPerms", "logs", "config"] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        for file in [
            "server.properties",
            "server.jar",
            "whitelist.json",
            "world/level.dat",
            "world/region/r.0.0.mca",
            "plugins/LuckPerms.jar",
            "plugins/LuckPerms/config.yml",
            "logs/latest.log",
            "config/paper-global.yml",
        ] {
            fs::write(root.join(file), "x").unwrap();
        }
        let list = |mode| {
            let mut out = Vec::new();
            collect(&root, &root, mode, &mut out);
            let mut names: Vec<String> = out.iter().map(|p| p.to_string_lossy().replace('\\', "/")).collect();
            names.sort();
            names
        };
        assert_eq!(
            list(ExportMode::Configs),
            ["config/paper-global.yml", "plugins/LuckPerms/config.yml", "server.properties", "whitelist.json"]
        );
        let full = list(ExportMode::Full);
        assert!(full.contains(&"world/region/r.0.0.mca".to_string()));
        assert!(full.contains(&"plugins/LuckPerms.jar".to_string()));
        assert!(!full.iter().any(|f| f == "server.jar" || f.starts_with("logs/")));
        let _ = fs::remove_dir_all(&root);
    }
}
