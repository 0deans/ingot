//! Pure Binary VDF parser, serializer, and non-Steam shortcut upsert rules for `shortcuts.vdf`.

use std::path::PathBuf;

const TYPE_MAP: u8 = 0x00;
const TYPE_STRING: u8 = 0x01;
const TYPE_INT: u8 = 0x02;
const TYPE_END: u8 = 0x08;
const MAX_VDF_DEPTH: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VdfValue {
    Map(Vec<VdfEntry>),
    Str(String),
    Int(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VdfEntry {
    pub key: String,
    pub value: VdfValue,
}

#[derive(Debug, Clone)]
pub struct SteamShortcutSpec {
    pub instance_id: String,
    pub app_name: String,
    pub target_exe: PathBuf,
    pub start_dir: PathBuf,
    pub icon_path: Option<PathBuf>,
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
pub fn quote_steam_path(path: &std::path::Path) -> String {
    let raw = path.to_string_lossy();
    let trimmed = raw.trim_matches('"');
    format!("\"{trimmed}\"")
}

/// Parses a binary `shortcuts.vdf` payload into a lossless entry tree.
pub fn parse_shortcuts_vdf(bytes: &[u8]) -> Result<Vec<VdfEntry>, String> {
    if bytes.is_empty() {
        return Ok(vec![VdfEntry {
            key: "shortcuts".to_string(),
            value: VdfValue::Map(Vec::new()),
        }]);
    }

    let mut cursor = 0usize;
    let entries = parse_map_entries(bytes, &mut cursor, 0)?;

    let has_shortcuts_map = entries.iter().any(|entry| {
        entry.key.eq_ignore_ascii_case("shortcuts") && matches!(entry.value, VdfValue::Map(_))
    });
    if !has_shortcuts_map {
        return Err("Missing root 'shortcuts' map in shortcuts.vdf".to_string());
    }

    Ok(entries)
}

fn parse_map_entries(
    bytes: &[u8],
    cursor: &mut usize,
    depth: usize,
) -> Result<Vec<VdfEntry>, String> {
    if depth > MAX_VDF_DEPTH {
        return Err("Binary VDF nesting depth exceeded limit".to_string());
    }

    let mut entries = Vec::new();
    while let Some(&tag) = bytes.get(*cursor) {
        *cursor += 1;

        if tag == TYPE_END {
            return Ok(entries);
        }

        let key = read_cstring(bytes, cursor)?;
        let value = match tag {
            TYPE_MAP => {
                let children = parse_map_entries(bytes, cursor, depth + 1)?;
                VdfValue::Map(children)
            }
            TYPE_STRING => {
                let text = read_cstring(bytes, cursor)?;
                VdfValue::Str(text)
            }
            TYPE_INT => {
                let number = read_u32_le(bytes, cursor)?;
                VdfValue::Int(number)
            }
            other => {
                return Err(format!(
                    "Unexpected binary VDF type tag 0x{other:02x} at byte offset {}",
                    cursor.saturating_sub(1)
                ));
            }
        };

        entries.push(VdfEntry { key, value });
    }

    if depth > 0 {
        return Err("Unexpected end of file inside nested binary VDF map".to_string());
    }

    Ok(entries)
}

fn read_cstring(bytes: &[u8], cursor: &mut usize) -> Result<String, String> {
    let start = *cursor;
    while let Some(&byte) = bytes.get(*cursor) {
        if byte == 0 {
            let slice = bytes
                .get(start..*cursor)
                .ok_or_else(|| "Invalid string slice bounds in binary VDF".to_string())?;
            *cursor += 1;
            return Ok(String::from_utf8_lossy(slice).into_owned());
        }
        *cursor += 1;
    }
    Err("Unterminated null-delimited string in binary VDF".to_string())
}

fn read_u32_le(bytes: &[u8], cursor: &mut usize) -> Result<u32, String> {
    let end = cursor
        .checked_add(4)
        .ok_or_else(|| "Integer overflow while reading binary VDF".to_string())?;
    let slice = bytes.get(*cursor..end).ok_or_else(|| {
        "Unexpected end of file while reading 32-bit integer in binary VDF".to_string()
    })?;
    let raw: [u8; 4] = slice
        .try_into()
        .map_err(|_| "Failed to read 4-byte integer in binary VDF".to_string())?;
    *cursor = end;
    Ok(u32::from_le_bytes(raw))
}

/// Serializes the root VDF entries back into binary `shortcuts.vdf` format.
pub fn serialize_shortcuts_vdf(entries: &[VdfEntry]) -> Vec<u8> {
    let mut out = Vec::new();
    write_map_entries(&mut out, entries);
    out.push(TYPE_END);
    out
}

fn write_map_entries(out: &mut Vec<u8>, entries: &[VdfEntry]) {
    for entry in entries {
        match &entry.value {
            VdfValue::Map(children) => {
                out.push(TYPE_MAP);
                write_cstring(out, &entry.key);
                write_map_entries(out, children);
                out.push(TYPE_END);
            }
            VdfValue::Str(text) => {
                out.push(TYPE_STRING);
                write_cstring(out, &entry.key);
                write_cstring(out, text);
            }
            VdfValue::Int(number) => {
                out.push(TYPE_INT);
                write_cstring(out, &entry.key);
                out.extend_from_slice(&number.to_le_bytes());
            }
        }
    }
}

fn write_cstring(out: &mut Vec<u8>, text: &str) {
    for byte in text.as_bytes() {
        if *byte != 0 {
            out.push(*byte);
        }
    }
    out.push(0);
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

    if let Some(fields) = existing_entry {
        set_field(fields, "appid", VdfValue::Int(app_id));
        set_field(fields, "AppName", VdfValue::Str(spec.app_name.clone()));
        set_field(fields, "Exe", VdfValue::Str(quoted_exe));
        set_field(fields, "StartDir", VdfValue::Str(quoted_start_dir));
        set_field(fields, "icon", VdfValue::Str(icon_str));
        set_field(fields, "LaunchOptions", VdfValue::Str(launch_options));
        ensure_tags(fields);
    } else {
        let next_index = next_shortcut_key(shortcuts_map);
        let fields = build_new_shortcut_fields(
            app_id,
            &spec.app_name,
            &quoted_exe,
            &quoted_start_dir,
            &icon_str,
            &launch_options,
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

fn ensure_tags(fields: &mut Vec<VdfEntry>) {
    let required_tags = ["Minecraft", "Ingot"];
    if let Some(tags_entry) = fields
        .iter_mut()
        .find(|entry| entry.key.eq_ignore_ascii_case("tags"))
    {
        if let VdfValue::Map(tag_list) = &mut tags_entry.value {
            for &required in &required_tags {
                let already_present = tag_list.iter().any(|item| {
                    matches!(&item.value, VdfValue::Str(val) if val.eq_ignore_ascii_case(required))
                });
                if !already_present {
                    let key = next_shortcut_key(tag_list);
                    tag_list.push(VdfEntry {
                        key,
                        value: VdfValue::Str(required.to_string()),
                    });
                }
            }
            return;
        }
    }

    fields.push(VdfEntry {
        key: "tags".to_string(),
        value: VdfValue::Map(vec![
            VdfEntry {
                key: "0".to_string(),
                value: VdfValue::Str("Minecraft".to_string()),
            },
            VdfEntry {
                key: "1".to_string(),
                value: VdfValue::Str("Ingot".to_string()),
            },
        ]),
    });
}

fn build_new_shortcut_fields(
    app_id: u32,
    app_name: &str,
    quoted_exe: &str,
    quoted_start_dir: &str,
    icon_str: &str,
    launch_options: &str,
) -> Vec<VdfEntry> {
    vec![
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
            value: VdfValue::Int(0),
        },
        VdfEntry {
            key: "FlatpakAppID".to_string(),
            value: VdfValue::Str(String::new()),
        },
        VdfEntry {
            key: "sortas".to_string(),
            value: VdfValue::Str(String::new()),
        },
        VdfEntry {
            key: "tags".to_string(),
            value: VdfValue::Map(vec![
                VdfEntry {
                    key: "0".to_string(),
                    value: VdfValue::Str("Minecraft".to_string()),
                },
                VdfEntry {
                    key: "1".to_string(),
                    value: VdfValue::Str("Ingot".to_string()),
                },
            ]),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn computes_standard_ieee_crc32_and_steam_appid() {
        assert_eq!(crc32_ieee(b"123456789"), 0xCBF4_3926);
        let app_id = compute_steam_app_id("\"/usr/bin/ingot\"", "Ingot - Survival");
        assert_ne!(app_id & 0x8000_0000, 0);
    }

    #[test]
    fn round_trips_binary_vdf_and_preserves_unknown_fields() {
        let mut root = parse_shortcuts_vdf(&[]).unwrap();
        let spec = SteamShortcutSpec {
            instance_id: "survival-1".to_string(),
            app_name: "Ingot - Survival".to_string(),
            target_exe: PathBuf::from("/usr/bin/ingot"),
            start_dir: PathBuf::from("/usr/bin"),
            icon_path: Some(PathBuf::from("/tmp/icon.png")),
        };

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
}
