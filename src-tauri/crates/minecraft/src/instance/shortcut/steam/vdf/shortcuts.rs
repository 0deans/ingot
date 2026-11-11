//! Non-Steam shortcut identification, `shortcuts.vdf` entry merging, and atomic persistence.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::instance::shortcut::ShortcutError;

use super::codec::{parse_vdf, serialize_vdf, VdfEntry, VdfValue};

#[derive(Debug, Clone)]
pub struct SteamShortcutSpec {
    pub instance_id: String,
    pub app_name: String,
    pub target_exe: PathBuf,
    pub start_dir: PathBuf,
    pub icon_path: Option<PathBuf>,
    pub group: Option<String>,
    pub loader_name: String,
    pub last_played: Option<u64>,
}

/// Computes standard IEEE 802.3 CRC32 over a byte slice.
pub fn crc32_ieee(bytes: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            if (crc & 1) != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Computes the 32-bit unsigned Steam non-Steam shortcut AppID (`crc32(exe + app_name) | 0x80000000`).
pub fn compute_steam_app_id(quoted_exe: &str, app_name: &str) -> u32 {
    let input = format!("{quoted_exe}{app_name}");
    crc32_ieee(input.as_bytes()) | 0x8000_0000
}

/// Formats an executable or directory path wrapped in double quotes as expected by Steam.
pub fn quote_steam_path(path: &Path) -> String {
    let raw = path.to_string_lossy();
    let trimmed = raw.trim_matches('"');
    format!("\"{trimmed}\"")
}

/// Parses a binary `shortcuts.vdf` payload into a lossless entry tree, validating the root `shortcuts` map.
pub fn parse_shortcuts_vdf(bytes: &[u8]) -> Result<Vec<VdfEntry>, String> {
    if bytes.is_empty() {
        return Ok(vec![VdfEntry {
            key: "shortcuts".to_string(),
            value: VdfValue::Map(Vec::new()),
        }]);
    }

    let entries = parse_vdf(bytes)?;

    let has_shortcuts_map = entries.iter().any(|entry| {
        entry.key.eq_ignore_ascii_case("shortcuts") && matches!(entry.value, VdfValue::Map(_))
    });
    if !has_shortcuts_map {
        return Err("Missing root 'shortcuts' map in shortcuts.vdf".to_string());
    }

    Ok(entries)
}

/// Serializes the root `shortcuts.vdf` entries back into binary VDF format.
pub fn serialize_shortcuts_vdf(entries: &[VdfEntry]) -> Vec<u8> {
    serialize_vdf(entries)
}

/// Reads `<config_dir>/shortcuts.vdf` (if present), merges the shortcut spec, and atomically persists it.
pub(in crate::instance::shortcut::steam) fn upsert_shortcuts_file(
    config_dir: &Path,
    spec: &SteamShortcutSpec,
) -> Result<PathBuf, ShortcutError> {
    let vdf_path = config_dir.join("shortcuts.vdf");
    let existing_bytes = match std::fs::read(&vdf_path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(err) => return Err(ShortcutError::Io(err)),
    };

    let mut root_entries =
        parse_shortcuts_vdf(&existing_bytes).map_err(ShortcutError::SteamCorruptVdf)?;
    upsert_steam_shortcut(&mut root_entries, spec).map_err(ShortcutError::SteamCorruptVdf)?;
    let serialized = serialize_shortcuts_vdf(&root_entries);

    let mut staged = tempfile::NamedTempFile::new_in(config_dir)?;
    staged.write_all(&serialized)?;
    staged.flush()?;
    staged.persist(&vdf_path).map_err(|e| e.error)?;

    Ok(vdf_path)
}

/// Inserts or updates an Ingot instance shortcut in the parsed `shortcuts.vdf` AST and returns its `appid`.
pub fn upsert_steam_shortcut(
    root_entries: &mut Vec<VdfEntry>,
    spec: &SteamShortcutSpec,
) -> Result<u32, String> {
    let quoted_exe = quote_steam_path(&spec.target_exe);
    let quoted_start_dir = quote_steam_path(&spec.start_dir);
    let icon_str = spec
        .icon_path
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let launch_options = format!("--launch-instance \"{}\"", spec.instance_id);
    let app_id = compute_steam_app_id(&quoted_exe, &spec.app_name);

    if !root_entries
        .iter()
        .any(|entry| entry.key.eq_ignore_ascii_case("shortcuts"))
    {
        root_entries.push(VdfEntry {
            key: "shortcuts".to_string(),
            value: VdfValue::Map(Vec::new()),
        });
    }

    let shortcuts_entry = root_entries
        .iter_mut()
        .find(|entry| entry.key.eq_ignore_ascii_case("shortcuts"))
        .ok_or_else(|| "Missing root 'shortcuts' entry".to_string())?;

    let VdfValue::Map(shortcuts_map) = &mut shortcuts_entry.value else {
        return Err("Root 'shortcuts' entry in shortcuts.vdf is not a map".to_string());
    };

    let existing_entry = shortcuts_map
        .iter_mut()
        .find_map(|entry| match &mut entry.value {
            VdfValue::Map(fields)
                if matches_instance_shortcut(fields, &spec.instance_id, app_id) =>
            {
                Some(fields)
            }
            _ => None,
        });

    let last_play_time = spec.last_played.unwrap_or(0).min(u64::from(u32::MAX)) as u32;
    let extra_tags = build_extra_tags(spec);

    if let Some(fields) = existing_entry {
        set_field(fields, "appid", VdfValue::Int(app_id));
        set_field(fields, "AppName", VdfValue::Str(spec.app_name.clone()));
        set_field(fields, "Exe", VdfValue::Str(quoted_exe));
        set_field(fields, "StartDir", VdfValue::Str(quoted_start_dir));
        set_field(fields, "icon", VdfValue::Str(icon_str));
        set_field(fields, "LaunchOptions", VdfValue::Str(launch_options));
        update_last_play_time(fields, last_play_time);
        ensure_tags(fields, &extra_tags);
    } else {
        let next_index = next_shortcut_key(shortcuts_map);
        let fields = build_new_shortcut_fields(
            app_id,
            &spec.app_name,
            &quoted_exe,
            &quoted_start_dir,
            &icon_str,
            &launch_options,
            last_play_time,
            &extra_tags,
        );
        shortcuts_map.push(VdfEntry {
            key: next_index,
            value: VdfValue::Map(fields),
        });
    }

    Ok(app_id)
}

fn matches_instance_shortcut(fields: &[VdfEntry], instance_id: &str, app_id: u32) -> bool {
    let quoted_target = format!("--launch-instance \"{instance_id}\"");
    let unquoted_target = format!("--launch-instance {instance_id}");

    for field in fields {
        if field.key.eq_ignore_ascii_case("LaunchOptions") {
            if let VdfValue::Str(options) = &field.value {
                if options.contains(&quoted_target) || options.trim() == unquoted_target {
                    return true;
                }
            }
        }
        if field.key.eq_ignore_ascii_case("appid") {
            if let VdfValue::Int(existing_id) = field.value {
                if existing_id == app_id {
                    return true;
                }
            }
        }
    }
    false
}

fn next_shortcut_key(shortcuts_map: &[VdfEntry]) -> String {
    let max_idx = shortcuts_map
        .iter()
        .filter_map(|entry| entry.key.parse::<usize>().ok())
        .max();
    match max_idx {
        Some(current_max) => (current_max + 1).to_string(),
        None => shortcuts_map.len().to_string(),
    }
}

fn set_field(fields: &mut Vec<VdfEntry>, name: &str, value: VdfValue) {
    if let Some(existing) = fields
        .iter_mut()
        .find(|entry| entry.key.eq_ignore_ascii_case(name))
    {
        existing.value = value;
    } else {
        fields.push(VdfEntry {
            key: name.to_string(),
            value,
        });
    }
}

fn build_extra_tags(spec: &SteamShortcutSpec) -> Vec<String> {
    let mut tags = Vec::new();
    let loader = spec.loader_name.trim();
    if !loader.is_empty() {
        tags.push(loader.to_string());
    }
    if let Some(group) = &spec.group {
        let trimmed = group.trim();
        if !trimmed.is_empty() {
            tags.push(trimmed.to_string());
        }
    }
    tags
}

fn update_last_play_time(fields: &mut Vec<VdfEntry>, candidate: u32) {
    if candidate == 0 {
        return;
    }
    if let Some(existing) = fields
        .iter_mut()
        .find(|entry| entry.key.eq_ignore_ascii_case("LastPlayTime"))
    {
        if let VdfValue::Int(current) = existing.value {
            if candidate > current {
                existing.value = VdfValue::Int(candidate);
            }
            return;
        }
        existing.value = VdfValue::Int(candidate);
    } else {
        fields.push(VdfEntry {
            key: "LastPlayTime".to_string(),
            value: VdfValue::Int(candidate),
        });
    }
}

fn ensure_tags(fields: &mut Vec<VdfEntry>, extra_tags: &[String]) {
    let mut required_tags = vec!["Minecraft".to_string(), "Ingot".to_string()];
    for extra in extra_tags {
        if !required_tags
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(extra))
        {
            required_tags.push(extra.clone());
        }
    }

    if let Some(tags_entry) = fields
        .iter_mut()
        .find(|entry| entry.key.eq_ignore_ascii_case("tags"))
    {
        if let VdfValue::Map(tag_list) = &mut tags_entry.value {
            for required in &required_tags {
                let already_present = tag_list.iter().any(|item| {
                    matches!(&item.value, VdfValue::Str(val) if val.eq_ignore_ascii_case(required))
                });
                if !already_present {
                    let key = next_shortcut_key(tag_list);
                    tag_list.push(VdfEntry {
                        key,
                        value: VdfValue::Str(required.clone()),
                    });
                }
            }
            return;
        }
    }

    let tag_entries = required_tags
        .into_iter()
        .enumerate()
        .map(|(idx, tag)| VdfEntry {
            key: idx.to_string(),
            value: VdfValue::Str(tag),
        })
        .collect();

    fields.push(VdfEntry {
        key: "tags".to_string(),
        value: VdfValue::Map(tag_entries),
    });
}

#[allow(clippy::too_many_arguments)]
fn build_new_shortcut_fields(
    app_id: u32,
    app_name: &str,
    quoted_exe: &str,
    quoted_start_dir: &str,
    icon_str: &str,
    launch_options: &str,
    last_play_time: u32,
    extra_tags: &[String],
) -> Vec<VdfEntry> {
    let mut fields = vec![
        VdfEntry {
            key: "appid".to_string(),
            value: VdfValue::Int(app_id),
        },
        VdfEntry {
            key: "AppName".to_string(),
            value: VdfValue::Str(app_name.to_string()),
        },
        VdfEntry {
            key: "Exe".to_string(),
            value: VdfValue::Str(quoted_exe.to_string()),
        },
        VdfEntry {
            key: "StartDir".to_string(),
            value: VdfValue::Str(quoted_start_dir.to_string()),
        },
        VdfEntry {
            key: "icon".to_string(),
            value: VdfValue::Str(icon_str.to_string()),
        },
        VdfEntry {
            key: "ShortcutPath".to_string(),
            value: VdfValue::Str(String::new()),
        },
        VdfEntry {
            key: "LaunchOptions".to_string(),
            value: VdfValue::Str(launch_options.to_string()),
        },
        VdfEntry {
            key: "IsHidden".to_string(),
            value: VdfValue::Int(0),
        },
        VdfEntry {
            key: "AllowDesktopConfig".to_string(),
            value: VdfValue::Int(1),
        },
        VdfEntry {
            key: "AllowOverlay".to_string(),
            value: VdfValue::Int(1),
        },
        VdfEntry {
            key: "OpenVR".to_string(),
            value: VdfValue::Int(0),
        },
        VdfEntry {
            key: "Devkit".to_string(),
            value: VdfValue::Int(0),
        },
        VdfEntry {
            key: "DevkitGameID".to_string(),
            value: VdfValue::Str(String::new()),
        },
        VdfEntry {
            key: "DevkitOverrideAppID".to_string(),
            value: VdfValue::Int(0),
        },
        VdfEntry {
            key: "LastPlayTime".to_string(),
            value: VdfValue::Int(last_play_time),
        },
        VdfEntry {
            key: "FlatpakAppID".to_string(),
            value: VdfValue::Str(String::new()),
        },
        VdfEntry {
            key: "sortas".to_string(),
            value: VdfValue::Str(String::new()),
        },
    ];
    ensure_tags(&mut fields, extra_tags);
    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_spec(
        instance_id: &str,
        app_name: &str,
        last_played: Option<u64>,
    ) -> SteamShortcutSpec {
        SteamShortcutSpec {
            instance_id: instance_id.to_string(),
            app_name: app_name.to_string(),
            target_exe: PathBuf::from("/usr/bin/ingot"),
            start_dir: PathBuf::from("/usr/bin"),
            icon_path: Some(PathBuf::from("/tmp/icon.png")),
            group: Some("1.21 SMP".to_string()),
            loader_name: "Fabric".to_string(),
            last_played,
        }
    }

    #[test]
    fn computes_standard_ieee_crc32_and_steam_appid() {
        assert_eq!(crc32_ieee(b"123456789"), 0xCBF4_3926);
        let app_id = compute_steam_app_id("\"/usr/bin/ingot\"", "Ingot - Survival");
        assert_ne!(app_id & 0x8000_0000, 0);
    }

    #[test]
    fn round_trips_binary_vdf_and_preserves_unknown_fields() {
        let mut root = parse_shortcuts_vdf(&[]).unwrap();
        let spec = sample_spec("survival-1", "Ingot - Survival", Some(1_700_000_000));

        let first_app_id = upsert_steam_shortcut(&mut root, &spec).unwrap();
        if let VdfValue::Map(shortcuts) = &mut root[0].value {
            if let VdfValue::Map(fields) = &mut shortcuts[0].value {
                fields.push(VdfEntry {
                    key: "CustomSteamField".to_string(),
                    value: VdfValue::Str("preserved-value".to_string()),
                });
            }
        }

        let bytes = serialize_shortcuts_vdf(&root);
        let mut reparsed = parse_shortcuts_vdf(&bytes).unwrap();
        assert_eq!(reparsed, root);

        // Updating the same instance is idempotent and preserves CustomSteamField
        let updated_spec = SteamShortcutSpec {
            app_name: "Ingot - Survival Renamed".to_string(),
            ..spec
        };
        let second_app_id = upsert_steam_shortcut(&mut reparsed, &updated_spec).unwrap();
        assert_ne!(first_app_id, second_app_id);

        let VdfValue::Map(shortcuts) = &reparsed[0].value else {
            panic!("expected shortcuts map");
        };
        assert_eq!(shortcuts.len(), 1);
        let VdfValue::Map(fields) = &shortcuts[0].value else {
            panic!("expected shortcut entry map");
        };
        assert!(fields.iter().any(|f| {
            f.key == "CustomSteamField" && f.value == VdfValue::Str("preserved-value".to_string())
        }));
        assert!(fields.iter().any(|f| {
            f.key == "AppName" && f.value == VdfValue::Str("Ingot - Survival Renamed".to_string())
        }));
    }

    #[test]
    fn rejects_corrupt_binary_vdf_payloads() {
        let invalid_tag = b"\x00shortcuts\x00\x7fbad\x00\x08\x08";
        assert!(parse_shortcuts_vdf(invalid_tag).is_err());

        let truncated_int = b"\x00shortcuts\x00\x02appid\x00\x01\x02";
        assert!(parse_shortcuts_vdf(truncated_int).is_err());

        let missing_shortcuts_root = b"\x00other\x00\x08\x08";
        assert!(parse_shortcuts_vdf(missing_shortcuts_root).is_err());

        let quoted = quote_steam_path(Path::new("C:\\Games\\Ingot\\ingot.exe"));
        assert_eq!(quoted, "\"C:\\Games\\Ingot\\ingot.exe\"");
    }

    #[test]
    fn updates_unquoted_launch_match_and_only_advances_last_play_time() {
        let mut root = vec![VdfEntry {
            key: "shortcuts".to_string(),
            value: VdfValue::Map(vec![VdfEntry {
                key: "0".to_string(),
                value: VdfValue::Map(vec![
                    VdfEntry {
                        key: "appid".to_string(),
                        value: VdfValue::Int(42),
                    },
                    VdfEntry {
                        key: "LaunchOptions".to_string(),
                        value: VdfValue::Str("  --launch-instance skyblock-1  ".to_string()),
                    },
                    VdfEntry {
                        key: "LastPlayTime".to_string(),
                        value: VdfValue::Int(200),
                    },
                ]),
            }]),
        }];

        let older_spec = sample_spec("skyblock-1", "Ingot - Skyblock", Some(150));
        upsert_steam_shortcut(&mut root, &older_spec).unwrap();
        let VdfValue::Map(shortcuts) = &root[0].value else {
            panic!("expected shortcuts map");
        };
        let VdfValue::Map(fields) = &shortcuts[0].value else {
            panic!("expected shortcut fields");
        };
        let last_play = fields
            .iter()
            .find(|f| f.key == "LastPlayTime")
            .map(|f| &f.value);
        assert_eq!(last_play, Some(&VdfValue::Int(200)));

        let newer_spec = sample_spec("skyblock-1", "Ingot - Skyblock", Some(350));
        upsert_steam_shortcut(&mut root, &newer_spec).unwrap();
        let VdfValue::Map(shortcuts) = &root[0].value else {
            panic!("expected shortcuts map");
        };
        let VdfValue::Map(fields) = &shortcuts[0].value else {
            panic!("expected shortcut fields");
        };
        let last_play = fields
            .iter()
            .find(|f| f.key == "LastPlayTime")
            .map(|f| &f.value);
        assert_eq!(last_play, Some(&VdfValue::Int(350)));
    }

    #[test]
    fn upsert_shortcuts_file_creates_updates_and_rejects_corrupt_files_without_overwriting() {
        let temp_dir = tempfile::tempdir().unwrap();
        let first = sample_spec("inst-a", "Ingot - First", Some(100));
        let path = upsert_shortcuts_file(temp_dir.path(), &first).unwrap();
        assert_eq!(path, temp_dir.path().join("shortcuts.vdf"));

        let second = sample_spec("inst-b", "Ingot - Second", Some(200));
        upsert_shortcuts_file(temp_dir.path(), &second).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        let parsed = parse_shortcuts_vdf(&bytes).unwrap();
        let VdfValue::Map(shortcuts) = &parsed[0].value else {
            panic!("expected shortcuts map");
        };
        assert_eq!(shortcuts.len(), 2);

        let corrupt_payload = b"not-valid-vdf";
        std::fs::write(&path, corrupt_payload).unwrap();
        let err = upsert_shortcuts_file(temp_dir.path(), &first).unwrap_err();
        assert!(matches!(err, ShortcutError::SteamCorruptVdf(_)));
        assert_eq!(std::fs::read(&path).unwrap(), corrupt_payload);
    }
}
