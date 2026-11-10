use super::error::WorldError;
use flate2::read::GzDecoder;
use std::collections::HashMap;
use std::io::Read;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLevelDat {
    pub display_name: String,
    pub game_mode: String,
    pub is_hardcore: bool,
    pub seed: Option<String>,
    pub last_played: Option<u64>,
    pub version_name: Option<String>,
}

pub fn decompress_if_gzipped(raw_bytes: &[u8]) -> Result<Vec<u8>, WorldError> {
    if raw_bytes.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = GzDecoder::new(raw_bytes);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| WorldError::Gz(e.to_string()))?;
        Ok(decompressed)
    } else {
        Ok(raw_bytes.to_vec())
    }
}

pub fn parse_level_dat(bytes: &[u8], fallback_name: &str) -> Result<ParsedLevelDat, WorldError> {
    let decompressed = decompress_if_gzipped(bytes)?;
    let root: fastnbt::Value = fastnbt::from_bytes(&decompressed)
        .map_err(|e| WorldError::Nbt(format!("Failed to parse level.dat NBT: {e}")))?;

    let root_map = match &root {
        fastnbt::Value::Compound(m) => m,
        _ => {
            return Err(WorldError::Nbt(
                "Root tag is not a compound tag".to_string(),
            ))
        }
    };

    let data_map = match root_map.get("Data") {
        Some(fastnbt::Value::Compound(m)) => m,
        _ => root_map,
    };

    let display_name = get_str(data_map, "LevelName")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| fallback_name.to_string());

    let raw_game_type = get_i32(data_map, "GameType").unwrap_or(0);
    let hardcore_byte = get_i32(data_map, "hardcore").unwrap_or(0);
    let is_hardcore = hardcore_byte != 0;

    let game_mode = if is_hardcore {
        "Hardcore".to_string()
    } else {
        match raw_game_type {
            1 => "Creative".to_string(),
            2 => "Adventure".to_string(),
            3 => "Spectator".to_string(),
            _ => "Survival".to_string(),
        }
    };

    let last_played = get_i64(data_map, "LastPlayed")
        .filter(|&ms| ms > 0)
        .map(|ms| (ms / 1000) as u64);

    let version_name = match data_map.get("Version") {
        Some(fastnbt::Value::Compound(v)) => get_str(v, "Name").map(str::to_string),
        _ => None,
    };

    let seed = find_seed_in_value(&root);

    Ok(ParsedLevelDat {
        display_name,
        game_mode,
        is_hardcore,
        seed,
        last_played,
        version_name,
    })
}

pub fn extract_seed_from_val(val: &fastnbt::Value) -> Option<String> {
    match val {
        fastnbt::Value::Long(v) => Some(v.to_string()),
        fastnbt::Value::Int(v) => Some(v.to_string()),
        fastnbt::Value::Short(v) => Some(v.to_string()),
        fastnbt::Value::Byte(v) => Some(v.to_string()),
        fastnbt::Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
        _ => None,
    }
}

pub fn find_seed_in_value(val: &fastnbt::Value) -> Option<String> {
    match val {
        fastnbt::Value::Compound(map) => {
            // Direct "seed" or "RandomSeed" key
            if let Some(s) = map
                .get("seed")
                .or_else(|| map.get("RandomSeed"))
                .and_then(extract_seed_from_val)
            {
                return Some(s);
            }

            // Prioritized sub-compounds
            for key in ["WorldGenSettings", "data", "Data", "dimensions"] {
                if let Some(sub) = map.get(key) {
                    if let Some(found) = find_seed_in_value(sub) {
                        return Some(found);
                    }
                }
            }

            // Fallback: recursively search remaining child compounds
            for (key, child) in map {
                if key != "WorldGenSettings"
                    && key != "data"
                    && key != "Data"
                    && key != "dimensions"
                {
                    if let Some(found) = find_seed_in_value(child) {
                        return Some(found);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

pub fn read_external_world_seed(world_dir: &std::path::Path) -> Option<String> {
    let candidate_paths = [
        world_dir
            .join("data")
            .join("minecraft")
            .join("world_gen_settings.dat"),
        world_dir.join("data").join("world_gen_settings.dat"),
        world_dir
            .join("data")
            .join("minecraft")
            .join("world_gen_settings.dat_old"),
        world_dir.join("data").join("world_gen_settings.dat_old"),
    ];

    for path in candidate_paths {
        if path.is_file() {
            if let Ok(bytes) = std::fs::read(&path) {
                if let Ok(decompressed) = decompress_if_gzipped(&bytes) {
                    if let Ok(root) = fastnbt::from_bytes::<fastnbt::Value>(&decompressed) {
                        if let Some(seed) = find_seed_in_value(&root) {
                            return Some(seed);
                        }
                    }
                }
            }
        }
    }

    None
}

fn get_str<'a>(map: &'a HashMap<String, fastnbt::Value>, key: &str) -> Option<&'a str> {
    match map.get(key) {
        Some(fastnbt::Value::String(s)) => Some(s.as_str()),
        _ => None,
    }
}

fn get_i32(map: &HashMap<String, fastnbt::Value>, key: &str) -> Option<i32> {
    match map.get(key) {
        Some(fastnbt::Value::Int(v)) => Some(*v),
        Some(fastnbt::Value::Byte(v)) => Some(*v as i32),
        Some(fastnbt::Value::Short(v)) => Some(*v as i32),
        Some(fastnbt::Value::Long(v)) => Some(*v as i32),
        _ => None,
    }
}

fn get_i64(map: &HashMap<String, fastnbt::Value>, key: &str) -> Option<i64> {
    match map.get(key) {
        Some(fastnbt::Value::Long(v)) => Some(*v),
        Some(fastnbt::Value::Int(v)) => Some(*v as i64),
        Some(fastnbt::Value::Short(v)) => Some(*v as i64),
        Some(fastnbt::Value::Byte(v)) => Some(*v as i64),
        _ => None,
    }
}
