use super::{publish_file, ServerFileError};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

// ─── server.properties ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PropertyEntry {
    pub key: String,
    pub value: String,
}

/// Decodes Java .properties escapes (`\uXXXX`, `\:`, `\=`, `\\`, `\n`...)
pub fn unescape_property(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                match u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    Some(decoded) => out.push(decoded),
                    None => out.push_str(&hex),
                }
            }
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

/// Encodes a value the way Java writes .properties: non-ASCII as `\uXXXX`, so it
/// reads back correctly whether the server loads the file as Latin-1 or UTF-8
pub fn escape_property(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_ascii() && !c.is_ascii_control() => out.push(c),
            c => {
                let mut buf = [0u16; 2];
                for unit in c.encode_utf16(&mut buf) {
                    out.push_str(&format!("\\u{unit:04X}"));
                }
            }
        }
    }
    out
}

fn parse_property_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
        return None;
    }
    let (key, value) = trimmed
        .split_once('=')
        .or_else(|| trimmed.split_once(':'))?;
    Some((
        unescape_property(key.trim()),
        unescape_property(value.trim_start()),
    ))
}

/// All properties in file order
pub fn read_properties(server_dir: &Path) -> Result<Vec<PropertyEntry>, ServerFileError> {
    let path = server_dir.join("server.properties");
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    Ok(raw
        .lines()
        .filter_map(parse_property_line)
        .map(|(key, value)| PropertyEntry { key, value })
        .collect())
}

/// Updates the given keys in place (keeping comments and order); new keys are appended
pub fn write_properties(
    server_dir: &Path,
    updates: &[PropertyEntry],
) -> Result<(), ServerFileError> {
    validate_updates(updates)?;
    let path = server_dir.join("server.properties");
    let mut lines: Vec<String> = match fs::read_to_string(&path) {
        Ok(raw) => raw.lines().map(str::to_owned).collect(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            vec!["# Minecraft server properties".into()]
        }
        Err(error) => return Err(ServerFileError::Io(error)),
    };

    for update in updates {
        let new_line = format!("{}={}", update.key, escape_property(&update.value));
        let mut found = false;
        for line in &mut lines {
            if parse_property_line(line).is_some_and(|(key, _)| key == update.key) {
                *line = new_line.clone();
                found = true;
            }
        }
        if !found {
            lines.push(new_line);
        }
    }
    publish_file(&path, (lines.join("\n") + "\n").as_bytes())
}

pub(crate) fn validate_updates(updates: &[PropertyEntry]) -> Result<(), ServerFileError> {
    let mut keys = std::collections::BTreeSet::new();
    for update in updates {
        let invalid = update.key.is_empty()
            || update.key.contains(['=', ':', '\n', '\r', '\\'])
            || update.key.trim() != update.key
            || update.key.starts_with(['#', '!']);
        if invalid || !keys.insert(&update.key) {
            return Err(ServerFileError::InvalidProperty(update.key.clone()));
        }
    }
    Ok(())
}
