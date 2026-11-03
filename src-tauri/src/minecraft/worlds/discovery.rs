use super::dimensions::{has_nether, has_the_end};
use super::error::WorldError;
use super::level_dat::{parse_level_dat, read_external_world_seed};
use super::models::WorldSummary;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use std::fs;
use std::path::Path;

pub fn calculate_dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if let Ok(meta) = fs::symlink_metadata(&p) {
            if meta.is_file() {
                total += meta.len();
            } else if meta.is_dir() {
                total += calculate_dir_size(&p);
            }
        }
    }
    total
}

pub fn get_instance_worlds(
    instance_dir: &Path,
    instance_id: &str,
    instance_name: &str,
) -> Result<Vec<WorldSummary>, WorldError> {
    let saves_dir = instance_dir.join("saves");
    if !saves_dir.is_dir() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&saves_dir)?;
    let mut worlds = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let folder_name = entry.file_name().to_string_lossy().to_string();
        if folder_name == "qp_world"
            || folder_name.starts_with(".qp_")
            || folder_name.starts_with(".staging_")
            || folder_name == ".ingot-trash"
        {
            continue;
        }

        let level_dat = path.join("level.dat");
        if !level_dat.is_file() {
            continue;
        }

        let level_bytes = fs::read(&level_dat).unwrap_or_default();
        let parsed = parse_level_dat(&level_bytes, &folder_name).unwrap_or_else(|_| {
            super::level_dat::ParsedLevelDat {
                display_name: folder_name.clone(),
                game_mode: "Survival".to_string(),
                is_hardcore: false,
                seed: None,
                last_played: None,
                version_name: None,
            }
        });

        let mut icon = None;
        let icon_path = path.join("icon.png");
        if icon_path.is_file() {
            if let Ok(icon_bytes) = fs::read(&icon_path) {
                let b64 = BASE64.encode(&icon_bytes);
                icon = Some(format!("data:image/png;base64,{b64}"));
            }
        }

        let size_bytes = calculate_dir_size(&path);
        let nether_present = has_nether(&path);
        let the_end_present = has_the_end(&path);
        let seed = parsed.seed.or_else(|| read_external_world_seed(&path));

        worlds.push(WorldSummary {
            folder_name,
            display_name: parsed.display_name,
            instance_id: instance_id.to_string(),
            instance_name: instance_name.to_string(),
            game_mode: parsed.game_mode,
            is_hardcore: parsed.is_hardcore,
            seed,
            last_played: parsed.last_played,
            size_bytes,
            version_name: parsed.version_name,
            icon,
            has_nether: nether_present,
            has_the_end: the_end_present,
        });
    }

    worlds.sort_by_key(|w| std::cmp::Reverse(w.last_played));
    Ok(worlds)
}
