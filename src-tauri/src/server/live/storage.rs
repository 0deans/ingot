use super::models::{bedrock_player_details, KnownPlayer, PlayerDetails};
use super::nbt::Nbt;
use super::players::player_from_nbt;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Folders holding `<uuid>.dat` player files (26.x layout first)
pub fn player_data_dirs(world_dir: &Path) -> [PathBuf; 2] {
    [
        world_dir.join("players").join("data"),
        world_dir.join("playerdata"),
    ]
}

pub fn read_player_file(path: &Path) -> Option<Nbt> {
    let bytes = std::fs::read(path).ok()?;
    let mut decoder = flate2::read::GzDecoder::new(bytes.as_slice());
    let mut raw = Vec::new();
    std::io::Read::read_to_end(&mut decoder, &mut raw).ok()?;
    fastnbt::from_bytes::<fastnbt::Value>(&raw)
        .ok()
        .map(Nbt::from)
}

/// name/uuid pairs the server has seen, from usercache.json
pub fn usercache(server_dir: &Path) -> HashMap<String, String> {
    #[derive(Deserialize)]
    struct Entry {
        name: String,
        uuid: String,
    }
    std::fs::read_to_string(server_dir.join("usercache.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<Vec<Entry>>(&raw).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|e| (e.uuid.to_lowercase(), e.name))
        .collect()
}

/// Every player with a save file, newest first
pub fn known_players(server_dir: &Path, world_dir: &Path) -> Vec<KnownPlayer> {
    let names = usercache(server_dir);
    let mut players: Vec<KnownPlayer> = Vec::new();
    for dir in player_data_dirs(world_dir) {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("dat") {
                continue;
            }
            let Some(uuid) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_lowercase)
            else {
                continue;
            };
            if players.iter().any(|p| p.uuid == uuid) {
                continue;
            }
            let data = read_player_file(&path);
            let name = names.get(&uuid).cloned().or_else(|| {
                data.as_ref()
                    .and_then(|d| d.get("bukkit"))
                    .and_then(|b| b.get("lastKnownName"))
                    .and_then(Nbt::as_str)
                    .map(str::to_string)
            });
            let Some(name) = name else {
                continue;
            };
            let last_seen = data
                .as_ref()
                .and_then(|d| player_from_nbt(&name, d, false).last_seen)
                .or_else(|| {
                    entry
                        .metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_millis() as f64)
                });
            players.push(KnownPlayer {
                name,
                uuid,
                online: false,
                last_seen,
            });
        }
    }
    players.sort_by(|a, b| {
        b.last_seen
            .unwrap_or(0.0)
            .total_cmp(&a.last_seen.unwrap_or(0.0))
    });
    players
}

/// Reads an offline player's last saved state
pub fn offline_player(
    server_dir: &Path,
    world_dir: &Path,
    name: &str,
) -> Result<PlayerDetails, String> {
    let known = known_players(server_dir, world_dir);
    let player = known
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| format!("No saved data for {name}"))?;
    for dir in player_data_dirs(world_dir) {
        if let Some(data) = read_player_file(&dir.join(format!("{}.dat", player.uuid))) {
            let mut details = player_from_nbt(&player.name, &data, false);
            details.uuid = Some(player.uuid.clone());
            return Ok(details);
        }
    }
    Err(format!("No saved data for {name}"))
}

/// Reads known Bedrock players from allowlist.json / whitelist.json
pub fn known_bedrock_players(server_dir: &Path) -> Vec<KnownPlayer> {
    let mut players: Vec<KnownPlayer> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for filename in ["allowlist.json", "whitelist.json"] {
        let path = server_dir.join(filename);
        if let Ok(raw) = std::fs::read_to_string(&path) {
            if let Ok(entries) = serde_json::from_str::<Vec<serde_json::Value>>(&raw) {
                for entry in entries {
                    if let Some(name) = entry.get("name").and_then(|n| n.as_str()) {
                        let lower = name.to_ascii_lowercase();
                        if !seen.contains(&lower) {
                            seen.insert(lower);
                            let uuid = entry
                                .get("xuid")
                                .or_else(|| entry.get("uuid"))
                                .and_then(|u| u.as_str())
                                .unwrap_or_default()
                                .to_string();
                            players.push(KnownPlayer {
                                name: name.to_string(),
                                uuid,
                                online: false,
                                last_seen: None,
                            });
                        }
                    }
                }
            }
        }
    }
    players
}

pub fn offline_bedrock_player(server_dir: &Path, name: &str) -> PlayerDetails {
    let known = known_bedrock_players(server_dir);
    let xuid = known
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
        .map(|p| p.uuid.clone());
    bedrock_player_details(name, false, xuid)
}
