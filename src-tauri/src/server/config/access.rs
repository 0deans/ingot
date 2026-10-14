use super::models::WhitelistEntry;
use std::fs;
use std::path::Path;
use uuid::Uuid;

pub fn read_server_whitelist(server_dir: &Path) -> Result<Vec<WhitelistEntry>, String> {
    let path = server_dir.join("whitelist.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read whitelist.json: {e}"))?;
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let entries: Vec<serde_json::Value> =
        serde_json::from_str(&raw).map_err(|e| format!("Failed to parse whitelist.json: {e}"))?;
    Ok(entries
        .into_iter()
        .filter_map(|v| {
            let name = v.get("name")?.as_str()?.to_string();
            let uuid = v
                .get("uuid")
                .and_then(|u| u.as_str())
                .unwrap_or("")
                .to_string();
            Some(WhitelistEntry { uuid, name })
        })
        .collect())
}

fn write_server_whitelist(server_dir: &Path, entries: &[WhitelistEntry]) -> Result<(), String> {
    let path = server_dir.join("whitelist.json");
    let json_entries: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "uuid": e.uuid,
                "name": e.name
            })
        })
        .collect();
    let raw = serde_json::to_string_pretty(&json_entries)
        .map_err(|e| format!("Failed to serialize whitelist: {e}"))?;
    fs::write(path, raw).map_err(|e| format!("Failed to write whitelist.json: {e}"))
}

/// Compute the offline-mode UUID that Minecraft uses for a given username.
/// This matches Java's `UUID.nameUUIDFromBytes(("OfflinePlayer:" + name).getBytes("UTF-8"))`.
pub fn offline_uuid(name: &str) -> String {
    let key = format!("OfflinePlayer:{name}");
    let bytes = key.as_bytes();
    let hash = md5_bytes(bytes);
    let mut b = hash;
    b[6] = (b[6] & 0x0f) | 0x30; // version 3
    b[8] = (b[8] & 0x3f) | 0x80; // variant RFC 4122
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        u32::from_be_bytes([b[0], b[1], b[2], b[3]]),
        u16::from_be_bytes([b[4], b[5]]),
        u16::from_be_bytes([b[6], b[7]]),
        u16::from_be_bytes([b[8], b[9]]),
        {
            let hi = u32::from_be_bytes([b[10], b[11], b[12], b[13]]) as u64;
            let lo = u16::from_be_bytes([b[14], b[15]]) as u64;
            (hi << 16) | lo
        }
    )
}

/// Earlier Ingot versions wrote whitelist UUIDs as a v3 UUID in the nil namespace,
/// which doesn't match Minecraft's offline UUID, so whitelisted players were still
/// rejected. Rewrites such entries in whitelist.json / ops.json.
pub fn repair_offline_uuids(server_dir: &Path) {
    for file in ["whitelist.json", "ops.json"] {
        let path = server_dir.join(file);
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(mut entries) = serde_json::from_str::<Vec<serde_json::Value>>(&raw) else {
            continue;
        };
        let mut changed = false;
        for entry in &mut entries {
            let Some(name) = entry
                .get("name")
                .and_then(|n| n.as_str())
                .map(str::to_string)
            else {
                continue;
            };
            let legacy =
                Uuid::new_v3(&Uuid::nil(), format!("OfflinePlayer:{name}").as_bytes()).to_string();
            if entry.get("uuid").and_then(|u| u.as_str()) == Some(legacy.as_str()) {
                entry["uuid"] = serde_json::Value::String(offline_uuid(&name));
                changed = true;
            }
        }
        if changed {
            if let Ok(fixed) = serde_json::to_string_pretty(&entries) {
                let _ = fs::write(&path, fixed);
            }
        }
    }
}

pub fn offline_uuid_for(name: &str) -> String {
    offline_uuid(name)
}

fn md5_bytes(data: &[u8]) -> [u8; 16] {
    use md5::Digest;
    md5::Md5::digest(data).into()
}

pub fn add_to_server_whitelist(server_dir: &Path, username: &str) -> Result<(), String> {
    let mut entries = read_server_whitelist(server_dir)?;
    if entries
        .iter()
        .any(|e| e.name.eq_ignore_ascii_case(username))
    {
        return Ok(());
    }
    let uuid = offline_uuid(username);
    entries.push(WhitelistEntry {
        uuid,
        name: username.to_string(),
    });
    write_server_whitelist(server_dir, &entries)
}

pub fn remove_from_server_whitelist(server_dir: &Path, username: &str) -> Result<(), String> {
    let mut entries = read_server_whitelist(server_dir)?;
    entries.retain(|e| !e.name.eq_ignore_ascii_case(username));
    write_server_whitelist(server_dir, &entries)
}
