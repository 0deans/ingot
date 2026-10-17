use serde::{Deserialize, Serialize};
use std::{fs, time::UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstanceWorldSummary {
    pub folder_name: String,
    pub display_name: String,
    pub icon: Option<String>,
    pub last_played: Option<u64>,
}

pub fn get_instance_worlds(
    instance_dir: &std::path::Path,
) -> Result<Vec<InstanceWorldSummary>, String> {
    use std::io::Read;
    let saves_dir = instance_dir.join("saves");
    if !saves_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&saves_dir).map_err(|e| format!("Failed to read saves dir: {e}"))?;
    let mut worlds = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let folder_name = entry.file_name().to_string_lossy().to_string();
        if folder_name == "qp_world" || folder_name.starts_with(".qp_") {
            continue;
        }
        let level_dat = path.join("level.dat");
        if !level_dat.is_file() {
            continue;
        }

        let mut display_name = folder_name.clone();
        let mut last_played = None;

        if let Ok(metadata) = level_dat.metadata() {
            if let Ok(modified) = metadata.modified() {
                if let Ok(dur) = modified.duration_since(UNIX_EPOCH) {
                    last_played = Some(dur.as_secs());
                }
            }
        }

        if let Ok(compressed_bytes) = fs::read(&level_dat) {
            let mut gz = flate2::read::GzDecoder::new(&compressed_bytes[..]);
            let mut decompressed = Vec::new();
            if gz.read_to_end(&mut decompressed).is_ok() {
                // Search for LevelName string tag in NBT: Tag ID (0x08), Name Len (0x00, 0x09), "LevelName"
                let needle = b"\x08\x00\x09LevelName";
                if let Some(pos) = decompressed.windows(needle.len()).position(|w| w == needle) {
                    let val_start = pos + needle.len();
                    if val_start + 2 <= decompressed.len() {
                        let str_len = u16::from_be_bytes([
                            decompressed[val_start],
                            decompressed[val_start + 1],
                        ]) as usize;
                        let str_end = val_start + 2 + str_len;
                        if str_end <= decompressed.len() {
                            if let Ok(name) =
                                std::str::from_utf8(&decompressed[val_start + 2..str_end])
                            {
                                if !name.trim().is_empty() {
                                    display_name = name.to_string();
                                }
                            }
                        }
                    }
                }

                // Also check for LastPlayed tag in NBT: Tag ID (0x04), Name Len (0x00, 0x0A), "LastPlayed"
                let lp_needle = b"\x04\x00\x0aLastPlayed";
                if let Some(pos) = decompressed
                    .windows(lp_needle.len())
                    .position(|w| w == lp_needle)
                {
                    let val_start = pos + lp_needle.len();
                    if val_start + 8 <= decompressed.len() {
                        if let Ok(bytes) = decompressed[val_start..val_start + 8].try_into() {
                            let lp_ms = i64::from_be_bytes(bytes);
                            if lp_ms > 0 {
                                last_played = Some((lp_ms / 1000) as u64);
                            }
                        }
                    }
                }
            }
        }

        let mut icon = None;
        let icon_path = path.join("icon.png");
        if icon_path.is_file() {
            if let Ok(icon_bytes) = fs::read(&icon_path) {
                use base64::Engine;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&icon_bytes);
                icon = Some(format!("data:image/png;base64,{b64}"));
            }
        }

        worlds.push(InstanceWorldSummary {
            folder_name,
            display_name,
            icon,
            last_played,
        });
    }

    worlds.sort_by_key(|a| std::cmp::Reverse(a.last_played));
    Ok(worlds)
}
