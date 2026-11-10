use super::{publish_file, ServerFileError};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

// ─── Access lists: whitelist, operators, bans ─────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum AccessListKind {
    Whitelist,
    Ops,
    Bans,
}

impl AccessListKind {
    pub fn file_name(self) -> &'static str {
        match self {
            Self::Whitelist => "whitelist.json",
            Self::Ops => "ops.json",
            Self::Bans => "banned-players.json",
        }
    }

    pub fn file_path(self, server_dir: &Path) -> std::path::PathBuf {
        match self {
            Self::Whitelist => {
                if server_dir.join("allowlist.json").exists() {
                    server_dir.join("allowlist.json")
                } else {
                    server_dir.join("whitelist.json")
                }
            }
            Self::Ops => {
                if server_dir.join("permissions.json").exists() {
                    server_dir.join("permissions.json")
                } else {
                    server_dir.join("ops.json")
                }
            }
            Self::Bans => server_dir.join("banned-players.json"),
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

fn read_json_list(path: &Path) -> Result<Vec<serde_json::Value>, ServerFileError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(ServerFileError::Io(error)),
    };
    let values: Vec<serde_json::Value> = serde_json::from_slice(&bytes)?;
    for value in &values {
        if !value.is_object() {
            return Err(ServerFileError::InvalidAccessEntry(
                "entry must be an object",
            ));
        }
        let identity = ["name", "uuid", "xuid"].iter().any(|key| {
            value
                .get(key)
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| !value.is_empty())
        });
        if !identity {
            return Err(ServerFileError::InvalidAccessEntry("missing identity"));
        }
    }
    Ok(values)
}

pub fn read_access_list(
    server_dir: &Path,
    kind: AccessListKind,
) -> Result<Vec<AccessEntry>, ServerFileError> {
    let path = kind.file_path(server_dir);
    Ok(read_json_list(&path)?
        .iter()
        .filter_map(|v| {
            let name = v
                .get("name")
                .and_then(|n| n.as_str())
                .map(|n| n.to_string());
            let uuid = v
                .get("uuid")
                .or_else(|| v.get("xuid"))
                .and_then(|u| u.as_str())
                .unwrap_or_default()
                .to_string();
            let level = v
                .get("level")
                .and_then(|l| l.as_i64())
                .map(|l| l as i32)
                .or_else(|| {
                    if v.get("permission").and_then(|p| p.as_str()) == Some("operator") {
                        Some(4)
                    } else {
                        None
                    }
                });
            let reason = v.get("reason").and_then(|r| r.as_str()).map(str::to_string);
            let name = name.unwrap_or_else(|| {
                if !uuid.is_empty() {
                    uuid.clone()
                } else {
                    String::new()
                }
            });
            if name.is_empty() {
                return None;
            }
            Some(AccessEntry {
                name,
                uuid,
                level,
                reason,
            })
        })
        .collect())
}

/// Edits the list file directly (server stopped)
pub fn add_access_entry(
    server_dir: &Path,
    kind: AccessListKind,
    name: &str,
    uuid: &str,
) -> Result<(), ServerFileError> {
    let path = kind.file_path(server_dir);
    let is_bedrock_allowlist = path.file_name().and_then(|n| n.to_str()) == Some("allowlist.json");
    let is_bedrock_permissions =
        path.file_name().and_then(|n| n.to_str()) == Some("permissions.json");

    let mut list = read_json_list(&path)?;
    if list.iter().any(|v| {
        v.get("name")
            .and_then(|n| n.as_str())
            .is_some_and(|n| n.eq_ignore_ascii_case(name))
            || (is_bedrock_permissions
                && v.get("xuid")
                    .and_then(|x| x.as_str())
                    .is_some_and(|x| x == uuid))
    }) {
        return Ok(());
    }
    list.push(match kind {
        AccessListKind::Whitelist => {
            if is_bedrock_allowlist {
                serde_json::json!({ "ignoresPlayerLimit": false, "name": name, "xuid": uuid })
            } else {
                serde_json::json!({ "uuid": uuid, "name": name })
            }
        }
        AccessListKind::Ops => {
            if is_bedrock_permissions {
                serde_json::json!({ "permission": "operator", "xuid": uuid })
            } else {
                serde_json::json!({
                    "uuid": uuid, "name": name, "level": 4, "bypassesPlayerLimit": false
                })
            }
        }
        AccessListKind::Bans => serde_json::json!({
            "uuid": uuid,
            "name": name,
            "created": chrono_like_now(),
            "source": "Ingot",
            "expires": "forever",
            "reason": "Banned by an operator."
        }),
    });
    let raw = serde_json::to_string_pretty(&list)?;
    publish_file(&path, raw.as_bytes())
}

pub fn remove_access_entry(
    server_dir: &Path,
    kind: AccessListKind,
    name: &str,
) -> Result<(), ServerFileError> {
    let path = kind.file_path(server_dir);
    let mut list = read_json_list(&path)?;
    list.retain(|v| {
        let matches_name = v
            .get("name")
            .and_then(|n| n.as_str())
            .is_some_and(|n| n.eq_ignore_ascii_case(name));
        let matches_xuid = v
            .get("xuid")
            .and_then(|x| x.as_str())
            .is_some_and(|x| x.eq_ignore_ascii_case(name));
        !matches_name && !matches_xuid
    });
    let raw = serde_json::to_string_pretty(&list)?;
    publish_file(&path, raw.as_bytes())
}

/// Minecraft's ban date format: "2026-09-26 12:00:00 +0000"
pub(super) fn chrono_like_now() -> String {
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
