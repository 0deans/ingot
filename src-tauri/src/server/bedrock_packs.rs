use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BedrockPackInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub pack_type: String, // "behavior" | "resource"
    pub folder_name: String,
    pub enabled: bool,
    pub system: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum BedrockPackError {
    #[error("I/O error during Bedrock pack operation: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error during Bedrock pack operation: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Failed to persist pack file: {0}")]
    Persist(#[from] tempfile::PersistError),

    #[error("Network request failed: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Cannot delete built-in BDS system packs")]
    SystemPack,

    #[error("Invalid pack folder name or linked destination")]
    InvalidFolderName,

    #[error("Pack folder not found: {0}")]
    NotFound(String),

    #[error("Download failed with status: {0}")]
    HttpStatus(reqwest::StatusCode),

    #[error("Task join error: {0}")]
    Task(#[from] tokio::task::JoinError),

    #[error("Archive error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Archive extraction error: {0}")]
    Archive(#[from] crate::app::ArchiveError),

    #[error("No manifest.json found in pack archive")]
    MissingManifest,

    #[error("Failed to parse pack manifest.json")]
    InvalidManifest,

    #[error("Pack folder '{0}' already exists; remove it explicitly before installing")]
    AlreadyExists(String),

    #[error("{primary}; removing unpublished pack failed: {cleanup}; retained at {}", path.display())]
    CleanupFailed {
        primary: String,
        cleanup: String,
        path: std::path::PathBuf,
    },
}

#[derive(Debug, Deserialize)]
struct PackManifest {
    header: Option<PackHeader>,
    modules: Option<Vec<PackModule>>,
}

#[derive(Debug, Deserialize)]
struct PackHeader {
    name: Option<String>,
    description: Option<String>,
    uuid: Option<String>,
    version: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PackModule {
    #[serde(rename = "type")]
    module_type: Option<String>,
    uuid: Option<String>,
    version: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorldPackRef {
    pack_id: String,
    version: serde_json::Value,
}

fn level_name(server_dir: &Path) -> String {
    let props = server_dir.join("server.properties");
    if let Ok(content) = fs::read_to_string(&props) {
        for line in content.lines() {
            let line = line.trim();
            if let Some(val) = line.strip_prefix("level-name=") {
                let name = val.trim();
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
    }
    "Bedrock level".to_string()
}

fn world_dir(server_dir: &Path) -> PathBuf {
    let level = level_name(server_dir);
    let p = server_dir.join("worlds").join(&level);
    if p.exists() {
        return p;
    }
    let p_plain = server_dir.join(&level);
    if p_plain.exists() {
        return p_plain;
    }
    // Fallback to first folder in worlds/
    if let Ok(entries) = fs::read_dir(server_dir.join("worlds")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                return path;
            }
        }
    }
    server_dir.join("worlds").join(level)
}

fn read_world_pack_refs(
    world: &Path,
    file_name: &str,
) -> Result<Vec<WorldPackRef>, BedrockPackError> {
    let path = world.join(file_name);
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(BedrockPackError::Io(error)),
    };
    Ok(serde_json::from_str(&content)?)
}

fn write_world_pack_refs(
    world: &Path,
    file_name: &str,
    refs: &[WorldPackRef],
) -> Result<(), BedrockPackError> {
    let path = world.join(file_name);
    fs::create_dir_all(world)?;
    let json = serde_json::to_string_pretty(refs)?;
    let mut staged = tempfile::NamedTempFile::new_in(world)?;
    std::io::Write::write_all(&mut staged, json.as_bytes())?;
    staged.as_file().sync_all()?;
    staged.persist(&path)?;
    Ok(())
}

fn format_version(v: Option<&serde_json::Value>) -> String {
    match v {
        Some(serde_json::Value::Array(arr)) => {
            let parts: Vec<String> = arr
                .iter()
                .filter_map(|x| x.as_u64().map(|n| n.to_string()))
                .collect();
            if !parts.is_empty() {
                return parts.join(".");
            }
        }
        Some(serde_json::Value::String(s)) => return s.clone(),
        _ => {}
    }
    "1.0.0".to_string()
}

fn parse_manifest(
    manifest_bytes: &[u8],
) -> Option<(String, String, String, Option<String>, String)> {
    let manifest: PackManifest = serde_json::from_slice(manifest_bytes).ok()?;
    let header = manifest.header?;
    let uuid = header.uuid?;
    let name = header.name.unwrap_or_else(|| "Unnamed Pack".to_string());
    let version = format_version(header.version.as_ref());
    let desc = header.description;

    let mut pack_type = "behavior".to_string();
    if let Some(modules) = manifest.modules {
        for m in modules {
            if let Some(t) = m.module_type {
                if t.eq_ignore_ascii_case("resources") {
                    pack_type = "resource".to_string();
                    break;
                }
            }
        }
    }
    Some((uuid, name, version, desc, pack_type))
}

fn is_system_pack(folder_name: &str, name: &str, description: Option<&str>) -> bool {
    let f = folder_name.to_ascii_lowercase();
    if f.starts_with("vanilla")
        || f.starts_with("chemistry")
        || f.starts_with("editor")
        || f.starts_with("experimental_")
        || f.starts_with("server_")
    {
        return true;
    }
    let n = name.to_ascii_lowercase();
    if n.starts_with("@minecraft")
        || n.starts_with("resourcepack.vanilla")
        || n.starts_with("server editor")
        || n.starts_with("experimental")
        || n.contains("mojang")
    {
        return true;
    }
    if let Some(d) = description {
        let dl = d.to_ascii_lowercase();
        if dl.contains("@minecraft") || dl.starts_with("experimental vanilla") {
            return true;
        }
    }
    false
}

pub fn list_installed_packs(server_dir: &Path) -> Result<Vec<BedrockPackInfo>, BedrockPackError> {
    let world = world_dir(server_dir);
    let active_behavior = read_world_pack_refs(&world, "world_behavior_packs.json")?;
    let active_resource = read_world_pack_refs(&world, "world_resource_packs.json")?;

    let mut results = Vec::new();

    for (sub, ptype, active_list) in [
        ("behavior_packs", "behavior", &active_behavior),
        ("resource_packs", "resource", &active_resource),
    ] {
        let dir = server_dir.join(sub);
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let folder_name = entry.file_name().to_string_lossy().into_owned();

            let manifest_path = path.join("manifest.json");
            let Ok(bytes) = fs::read(&manifest_path) else {
                continue;
            };

            if let Some((uuid, name, version, description, _)) = parse_manifest(&bytes) {
                if is_system_pack(&folder_name, &name, description.as_deref()) {
                    continue;
                }
                let enabled = active_list
                    .iter()
                    .any(|r| r.pack_id.eq_ignore_ascii_case(&uuid));
                results.push(BedrockPackInfo {
                    id: uuid,
                    name,
                    version,
                    description,
                    pack_type: ptype.to_string(),
                    folder_name,
                    enabled,
                    system: false,
                });
            }
        }
    }

    results.sort_by(|a, b| {
        if a.system != b.system {
            a.system.cmp(&b.system)
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    Ok(results)
}

pub fn set_pack_enabled(
    server_dir: &Path,
    pack_id: &str,
    pack_type: &str,
    enabled: bool,
) -> Result<(), BedrockPackError> {
    let world = world_dir(server_dir);
    let file_name = if pack_type == "resource" {
        "world_resource_packs.json"
    } else {
        "world_behavior_packs.json"
    };

    let mut active = read_world_pack_refs(&world, file_name)?;

    if enabled {
        if !active
            .iter()
            .any(|r| r.pack_id.eq_ignore_ascii_case(pack_id))
        {
            // Find the pack's version from manifest
            let sub = if pack_type == "resource" {
                "resource_packs"
            } else {
                "behavior_packs"
            };
            let mut pack_version = serde_json::json!([1, 0, 0]);

            if let Ok(entries) = fs::read_dir(server_dir.join(sub)) {
                for entry in entries.flatten() {
                    let manifest_path = entry.path().join("manifest.json");
                    if let Ok(bytes) = fs::read(&manifest_path) {
                        if let Ok(m) = serde_json::from_slice::<PackManifest>(&bytes) {
                            if let Some(h) = m.header {
                                if h.uuid
                                    .as_deref()
                                    .is_some_and(|u| u.eq_ignore_ascii_case(pack_id))
                                {
                                    if let Some(v) = h.version {
                                        pack_version = v;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            active.push(WorldPackRef {
                pack_id: pack_id.to_string(),
                version: pack_version,
            });
        }
    } else {
        active.retain(|r| !r.pack_id.eq_ignore_ascii_case(pack_id));
    }

    write_world_pack_refs(&world, file_name, &active)
}

pub fn delete_pack(
    server_dir: &Path,
    folder_name: &str,
    pack_type: &str,
) -> Result<(), BedrockPackError> {
    if is_system_pack(folder_name, "", None) || folder_name.contains("..") {
        return Err(BedrockPackError::SystemPack);
    }

    let sub = if pack_type == "resource" {
        "resource_packs"
    } else {
        "behavior_packs"
    };
    let target = crate::app::safe_enclosed_filename(&server_dir.join(sub), folder_name)
        .ok_or(BedrockPackError::InvalidFolderName)?;
    if !target.exists() {
        return Err(BedrockPackError::NotFound(folder_name.to_string()));
    }

    // Read manifest to unregister from world
    if let Ok(bytes) = fs::read(target.join("manifest.json")) {
        if let Some((uuid, _, _, _, _)) = parse_manifest(&bytes) {
            set_pack_enabled(server_dir, &uuid, pack_type, false)?;
        }
    }

    fs::remove_dir_all(&target)?;
    Ok(())
}

mod install;
pub use install::install_bedrock_pack;
