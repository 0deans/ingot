//! Server configuration files: every server.properties key, other config files
//! (bukkit/spigot/paper YAML, plugin configs...) and the access lists
//! (whitelist, operators, bans).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};

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
    let (key, value) = trimmed.split_once('=').or_else(|| trimmed.split_once(':'))?;
    Some((unescape_property(key.trim()), unescape_property(value.trim_start())))
}

/// All properties in file order
pub fn read_properties(server_dir: &Path) -> Result<Vec<PropertyEntry>, String> {
    let path = server_dir.join("server.properties");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("Failed to read server.properties: {e}"))?;
    Ok(raw
        .lines()
        .filter_map(parse_property_line)
        .map(|(key, value)| PropertyEntry { key, value })
        .collect())
}

/// Updates the given keys in place (keeping comments and order); new keys are appended
pub fn write_properties(server_dir: &Path, updates: &[PropertyEntry]) -> Result<(), String> {
    let path = server_dir.join("server.properties");
    let mut lines: Vec<String> = fs::read_to_string(&path)
        .map(|raw| raw.lines().map(str::to_string).collect())
        .unwrap_or_else(|_| vec!["# Minecraft server properties".to_string()]);

    for update in updates {
        if update.key.is_empty() || update.key.contains(['=', ':', '\n']) {
            return Err(format!("Invalid property key: {:?}", update.key));
        }
        let new_line = format!("{}={}", update.key, escape_property(&update.value));
        match lines
            .iter_mut()
            .find(|l| parse_property_line(l).is_some_and(|(k, _)| k == update.key))
        {
            Some(line) => *line = new_line,
            None => lines.push(new_line),
        }
    }
    fs::write(&path, lines.join("\n") + "\n").map_err(|e| format!("Failed to write server.properties: {e}"))
}

// ─── Other config files ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFile {
    /// Relative to the server folder, with forward slashes
    pub path: String,
    #[specta(type = i32)]
    pub size: f64,
}

const CONFIG_EXTENSIONS: [&str; 7] = ["yml", "yaml", "json", "json5", "toml", "properties", "conf"];

/// Managed through dedicated UI or runtime data - not shown as editable configs
const HIDDEN_FILES: [&str; 8] = [
    "whitelist.json",
    "ops.json",
    "banned-players.json",
    "banned-ips.json",
    "usercache.json",
    "version_history.json",
    "server.properties",
    "commands.yml",
];

fn is_config_file(path: &Path) -> bool {
    let ext_ok = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| CONFIG_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()));
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    ext_ok && !HIDDEN_FILES.contains(&name)
}

fn collect_configs(root: &Path, dir: &Path, depth: usize, out: &mut Vec<ConfigFile>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            if depth > 0 {
                collect_configs(root, &path, depth - 1, out);
            }
        } else if is_config_file(&path) && meta.len() <= 512 * 1024 {
            if let Ok(rel) = path.strip_prefix(root) {
                out.push(ConfigFile {
                    path: rel.to_string_lossy().replace('\\', "/"),
                    size: meta.len() as f64,
                });
            }
        }
    }
}

/// Config files in the server root, `config/` and each plugin folder
pub fn list_config_files(server_dir: &Path) -> Vec<ConfigFile> {
    let mut files = Vec::new();
    collect_configs(server_dir, server_dir, 0, &mut files);
    collect_configs(server_dir, &server_dir.join("config"), 3, &mut files);
    if let Ok(plugins) = fs::read_dir(server_dir.join("plugins")) {
        for plugin in plugins.flatten().filter(|p| p.path().is_dir()) {
            collect_configs(server_dir, &plugin.path(), 1, &mut files);
        }
    }
    files.sort_by(|a, b| {
        let depth = |p: &str| p.matches('/').count();
        depth(&a.path).cmp(&depth(&b.path)).then_with(|| a.path.cmp(&b.path))
    });
    files
}

/// Resolves a relative config path, refusing anything outside the server folder
fn config_path(server_dir: &Path, rel: &str) -> Result<PathBuf, String> {
    let rel_path = Path::new(rel);
    let safe = rel_path.components().all(|c| matches!(c, Component::Normal(_)));
    if !safe || !is_config_file(rel_path) {
        return Err(format!("Not an editable config file: {rel}"));
    }
    Ok(server_dir.join(rel_path))
}

pub fn read_config_file(server_dir: &Path, rel: &str) -> Result<String, String> {
    fs::read_to_string(config_path(server_dir, rel)?).map_err(|e| format!("Failed to read {rel}: {e}"))
}

pub fn write_config_file(server_dir: &Path, rel: &str, content: &str) -> Result<(), String> {
    let path = config_path(server_dir, rel)?;
    if !path.exists() {
        return Err(format!("{rel} does not exist"));
    }
    fs::write(path, content).map_err(|e| format!("Failed to write {rel}: {e}"))
}

// ─── Access lists: whitelist, operators, bans ─────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum AccessListKind {
    Whitelist,
    Ops,
    Bans,
}

impl AccessListKind {
    fn file_name(self) -> &'static str {
        match self {
            Self::Whitelist => "whitelist.json",
            Self::Ops => "ops.json",
            Self::Bans => "banned-players.json",
        }
    }

    /// Console commands used while the server runs, so it picks the change up live
    pub fn add_command(self, name: &str) -> String {
        match self {
            Self::Whitelist => format!("whitelist add {name}"),
            Self::Ops => format!("op {name}"),
            Self::Bans => format!("ban {name}"),
        }
    }

    pub fn remove_command(self, name: &str) -> String {
        match self {
            Self::Whitelist => format!("whitelist remove {name}"),
            Self::Ops => format!("deop {name}"),
            Self::Bans => format!("pardon {name}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AccessEntry {
    pub name: String,
    pub uuid: String,
    /// Operator permission level (1-4)
    pub level: Option<i32>,
    /// Ban reason
    pub reason: Option<String>,
}

fn read_json_list(path: &Path) -> Vec<serde_json::Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn read_access_list(server_dir: &Path, kind: AccessListKind) -> Vec<AccessEntry> {
    read_json_list(&server_dir.join(kind.file_name()))
        .iter()
        .filter_map(|v| {
            Some(AccessEntry {
                name: v.get("name")?.as_str()?.to_string(),
                uuid: v.get("uuid").and_then(|u| u.as_str()).unwrap_or_default().to_string(),
                level: v.get("level").and_then(|l| l.as_i64()).map(|l| l as i32),
                reason: v.get("reason").and_then(|r| r.as_str()).map(str::to_string),
            })
        })
        .collect()
}

/// Edits the list file directly (server stopped)
pub fn add_access_entry(server_dir: &Path, kind: AccessListKind, name: &str, uuid: &str) -> Result<(), String> {
    let path = server_dir.join(kind.file_name());
    let mut list = read_json_list(&path);
    if list
        .iter()
        .any(|v| v.get("name").and_then(|n| n.as_str()).is_some_and(|n| n.eq_ignore_ascii_case(name)))
    {
        return Ok(());
    }
    list.push(match kind {
        AccessListKind::Whitelist => serde_json::json!({ "uuid": uuid, "name": name }),
        AccessListKind::Ops => serde_json::json!({
            "uuid": uuid, "name": name, "level": 4, "bypassesPlayerLimit": false
        }),
        AccessListKind::Bans => serde_json::json!({
            "uuid": uuid,
            "name": name,
            "created": chrono_like_now(),
            "source": "Ingot",
            "expires": "forever",
            "reason": "Banned by an operator."
        }),
    });
    let raw = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
    fs::write(&path, raw).map_err(|e| format!("Failed to write {}: {e}", kind.file_name()))
}

pub fn remove_access_entry(server_dir: &Path, kind: AccessListKind, name: &str) -> Result<(), String> {
    let path = server_dir.join(kind.file_name());
    let mut list = read_json_list(&path);
    list.retain(|v| !v.get("name").and_then(|n| n.as_str()).is_some_and(|n| n.eq_ignore_ascii_case(name)));
    let raw = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
    fs::write(&path, raw).map_err(|e| format!("Failed to write {}: {e}", kind.file_name()))
}

/// Minecraft's ban date format: "2026-09-26 12:00:00 +0000"
fn chrono_like_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Civil date from days since epoch (Howard Hinnant's algorithm)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} +0000",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_escaping_round_trips() {
        let motd = "§6§lMy Server§r \\ ok";
        let escaped = escape_property(motd);
        assert_eq!(escaped, "\\u00A76\\u00A7lMy Server\\u00A7r \\\\ ok");
        assert_eq!(unescape_property(&escaped), motd);
        assert_eq!(unescape_property("\\u00a7aHi\\:there"), "§aHi:there");
    }

    #[test]
    fn rejects_paths_outside_server() {
        let dir = Path::new("/srv");
        assert!(config_path(dir, "../secret.yml").is_err());
        assert!(config_path(dir, "/etc/passwd.yml").is_err());
        assert!(config_path(dir, "world/level.dat").is_err());
        assert!(config_path(dir, "whitelist.json").is_err());
        assert!(config_path(dir, "config/paper-global.yml").is_ok());
    }

    #[test]
    fn ban_date_format() {
        let d = chrono_like_now();
        assert_eq!(d.len(), 25);
        assert!(d.ends_with(" +0000"));
    }
}
