use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

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

fn read_world_pack_refs(world: &Path, file_name: &str) -> Vec<WorldPackRef> {
    let path = world.join(file_name);
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(refs) = serde_json::from_str::<Vec<WorldPackRef>>(&content) {
            return refs;
        }
    }
    Vec::new()
}

fn write_world_pack_refs(world: &Path, file_name: &str, refs: &[WorldPackRef]) -> Result<(), String> {
    let path = world.join(file_name);
    let _ = fs::create_dir_all(world);
    let json = serde_json::to_string_pretty(refs).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| format!("Failed to write {}: {e}", path.display()))
}

fn format_version(v: Option<&serde_json::Value>) -> String {
    match v {
        Some(serde_json::Value::Array(arr)) => {
            let parts: Vec<String> = arr.iter().filter_map(|x| x.as_u64().map(|n| n.to_string())).collect();
            if !parts.is_empty() {
                return parts.join(".");
            }
        }
        Some(serde_json::Value::String(s)) => return s.clone(),
        _ => {}
    }
    "1.0.0".to_string()
}

fn parse_manifest(manifest_bytes: &[u8]) -> Option<(String, String, String, Option<String>, String)> {
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

pub fn list_installed_packs(server_dir: &Path) -> Result<Vec<BedrockPackInfo>, String> {
    let world = world_dir(server_dir);
    let active_behavior = read_world_pack_refs(&world, "world_behavior_packs.json");
    let active_resource = read_world_pack_refs(&world, "world_resource_packs.json");

    let mut results = Vec::new();

    for (sub, ptype, active_list) in [
        ("behavior_packs", "behavior", &active_behavior),
        ("resource_packs", "resource", &active_resource),
    ] {
        let dir = server_dir.join(sub);
        let Ok(entries) = fs::read_dir(&dir) else { continue };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let folder_name = entry.file_name().to_string_lossy().into_owned();

            let manifest_path = path.join("manifest.json");
            let Ok(bytes) = fs::read(&manifest_path) else { continue };

            if let Some((uuid, name, version, description, _)) = parse_manifest(&bytes) {
                if is_system_pack(&folder_name, &name, description.as_deref()) {
                    continue;
                }
                let enabled = active_list.iter().any(|r| r.pack_id.eq_ignore_ascii_case(&uuid));
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
) -> Result<(), String> {
    let world = world_dir(server_dir);
    let file_name = if pack_type == "resource" {
        "world_resource_packs.json"
    } else {
        "world_behavior_packs.json"
    };

    let mut active = read_world_pack_refs(&world, file_name);

    if enabled {
        if !active.iter().any(|r| r.pack_id.eq_ignore_ascii_case(pack_id)) {
            // Find the pack's version from manifest
            let sub = if pack_type == "resource" { "resource_packs" } else { "behavior_packs" };
            let mut pack_version = serde_json::json!([1, 0, 0]);

            if let Ok(entries) = fs::read_dir(server_dir.join(sub)) {
                for entry in entries.flatten() {
                    let manifest_path = entry.path().join("manifest.json");
                    if let Ok(bytes) = fs::read(&manifest_path) {
                        if let Ok(m) = serde_json::from_slice::<PackManifest>(&bytes) {
                            if let Some(h) = m.header {
                                if h.uuid.as_deref().is_some_and(|u| u.eq_ignore_ascii_case(pack_id)) {
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

pub fn delete_pack(server_dir: &Path, folder_name: &str, pack_type: &str) -> Result<(), String> {
    if is_system_pack(folder_name, "", None) || folder_name.contains("..") {
        return Err("Cannot delete built-in BDS system packs".to_string());
    }

    let sub = if pack_type == "resource" { "resource_packs" } else { "behavior_packs" };
    let target = server_dir.join(sub).join(folder_name);
    if !target.exists() {
        return Err(format!("Pack folder not found: {folder_name}"));
    }

    // Read manifest to unregister from world
    if let Ok(bytes) = fs::read(target.join("manifest.json")) {
        if let Some((uuid, _, _, _, _)) = parse_manifest(&bytes) {
            let _ = set_pack_enabled(server_dir, &uuid, pack_type, false);
        }
    }

    fs::remove_dir_all(&target).map_err(|e| format!("Failed to delete pack: {e}"))
}

pub async fn install_bedrock_pack(
    server_dir: &Path,
    download_url: &str,
    filename: &str,
) -> Result<String, String> {
    let clean_stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("addon")
        .replace([' ', '(', ')', '[', ']', '+'], "_");

    let client = reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()
        .map_err(|e| e.to_string())?;

    let mut req = client.get(download_url);
    if download_url.contains("curseforge.com") {
        req = req.header("x-api-key", crate::minecraft::content::CURSEFORGE_API_KEY);
    }

    let resp = req.send().await.map_err(|e| format!("Download failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Download failed with status: {}", resp.status()));
    }

    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;

    // Try extracting as zip archive (mcpack, mcaddon, or zip)
    let cursor = Cursor::new(&bytes);
    let mut zip = ZipArchive::new(cursor).map_err(|e| format!("Invalid archive file: {e}"))?;

    // Check if archive contains nested .mcpack files (.mcaddon bundle)
    let mut nested_packs: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..zip.len() {
        if let Ok(mut file) = zip.by_index(i) {
            let name = file.name().to_string();
            if name.ends_with(".mcpack") || name.ends_with(".zip") {
                let mut buf = Vec::new();
                if file.read_to_end(&mut buf).is_ok() {
                    nested_packs.push((name, buf));
                }
            }
        }
    }

    let mut installed_names = Vec::new();

    if !nested_packs.is_empty() {
        for (nested_name, nested_bytes) in nested_packs {
            let sub_stem = Path::new(&nested_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&clean_stem)
                .replace([' ', '(', ')', '[', ']', '+'], "_");

            if let Ok(mut sub_zip) = ZipArchive::new(Cursor::new(&nested_bytes)) {
                let extracted = extract_single_pack(server_dir, &mut sub_zip, &sub_stem)?;
                installed_names.push(extracted);
            }
        }
    } else {
        let extracted = extract_single_pack(server_dir, &mut zip, &clean_stem)?;
        installed_names.push(extracted);
    }

    Ok(format!("Installed {}", installed_names.join(", ")))
}

fn extract_single_pack<R: Read + std::io::Seek>(
    server_dir: &Path,
    zip: &mut ZipArchive<R>,
    pack_name: &str,
) -> Result<String, String> {
    // Find manifest to determine if behavior or resource pack
    let mut manifest_bytes = None;
    for i in 0..zip.len() {
        if let Ok(mut f) = zip.by_index(i) {
            let name = f.name().to_lowercase().replace('\\', "/");
            if name == "manifest.json" || name.ends_with("/manifest.json") {
                let mut buf = Vec::new();
                if f.read_to_end(&mut buf).is_ok() {
                    manifest_bytes = Some(buf);
                    break;
                }
            }
        }
    }

    let Some(m_bytes) = manifest_bytes else {
        return Err("No manifest.json found in pack archive".to_string());
    };

    let (uuid, name, _, _, pack_type) = parse_manifest(&m_bytes)
        .ok_or_else(|| "Failed to parse pack manifest.json".to_string())?;

    let sub = if pack_type == "resource" { "resource_packs" } else { "behavior_packs" };
    let out_dir = server_dir.join(sub).join(pack_name);
    fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

    // Determine strip prefix if files are wrapped in a top folder
    let mut prefix = String::new();
    for i in 0..zip.len() {
        if let Ok(f) = zip.by_index(i) {
            let name = f.name().replace('\\', "/");
            if name.ends_with("/manifest.json") {
                if let Some(p) = name.strip_suffix("manifest.json") {
                    prefix = p.to_string();
                    break;
                }
            }
        }
    }

    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(|e| e.to_string())?;
        let full_name = file.name().replace('\\', "/");
        let rel_name = if !prefix.is_empty() && full_name.starts_with(&prefix) {
            &full_name[prefix.len()..]
        } else {
            &full_name
        };

        if rel_name.is_empty() {
            continue;
        }

        let dest = out_dir.join(rel_name);
        if file.is_dir() {
            let _ = fs::create_dir_all(&dest);
        } else {
            if let Some(p) = dest.parent() {
                let _ = fs::create_dir_all(p);
            }
            let mut out = fs::File::create(&dest).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut out).map_err(|e| e.to_string())?;
        }
    }

    // Automatically enable for the world
    let _ = set_pack_enabled(server_dir, &uuid, &pack_type, true);

    Ok(format!("{name} ({pack_type})"))
}
