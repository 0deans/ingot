//! Where files come from: Modrinth (by SHA-1), CurseForge (by fingerprint) and Hangar
//! (server plugins Ingot installed). Each is reduced to the same `Release` shape, so the
//! planner doesn't care where a file came from.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

use super::TargetFile;

const MODRINTH: &str = "https://api.modrinth.com/v2";
const CURSEFORGE: &str = "https://api.curseforge.com/v1";
const HANGAR: &str = "https://hangar.papermc.io/api/v1";
/// CurseForge's id for Minecraft
const CF_MINECRAFT: u32 = 432;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    Modrinth,
    Curseforge,
    Hangar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Release,
    Beta,
    Alpha,
}

/// A dependency of a release, on the same source
#[derive(Debug, Clone)]
pub struct Dep {
    /// Required; otherwise it's marked incompatible
    pub required: bool,
    pub project: Option<String>,
    /// Modrinth sometimes names a version instead of a project
    pub version_id: Option<String>,
}

/// One version of a project, reduced to what planning needs
#[derive(Debug, Clone)]
pub struct Release {
    pub source: Source,
    pub project: String,
    pub version_id: String,
    pub version_number: String,
    pub channel: Channel,
    /// None when the author doesn't allow downloads outside their site
    pub file: Option<TargetFile>,
    pub deps: Vec<Dep>,
}

impl Release {
    /// A warning when the only version for the target is a beta or an alpha
    pub fn unstable_note(&self) -> Option<String> {
        match self.channel {
            Channel::Release => None,
            Channel::Beta => Some("Beta version, may be unstable".into()),
            Channel::Alpha => Some("Alpha version, may be unstable".into()),
        }
    }
}

/// Title, icon and web page of a project
#[derive(Debug, Clone, Default)]
pub struct ProjectInfo {
    pub title: String,
    pub icon_url: Option<String>,
    pub page_url: Option<String>,
}

/// Which files a folder may hold on each source, for the target loader
#[derive(Debug, Clone, Default)]
pub struct Scope {
    /// Modrinth loader names ("fabric", "paper", "minecraft", "iris"...)
    pub modrinth: Vec<&'static str>,
    /// CurseForge: search it at all, and the mod loader type for mods
    /// (Forge 1, Fabric 4, Quilt 5, NeoForge 6)
    pub curseforge: bool,
    pub cf_loader: Option<u32>,
    /// CurseForge lists loaders by name among the game versions ("Fabric", "NeoForge")
    pub cf_loader_names: Vec<&'static str>,
}

fn text(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

fn strings(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

async fn send(req: reqwest::RequestBuilder, name: &str) -> Result<Value, String> {
    req.send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("{name} request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Invalid response from {name}: {e}"))
}

// ─── Modrinth ─────────────────────────────────────────────────────────────────

fn modrinth_release(v: &Value) -> Release {
    let files = v.get("files").and_then(Value::as_array).cloned().unwrap_or_default();
    let file = files
        .iter()
        .find(|f| f.get("primary").and_then(Value::as_bool) == Some(true))
        .or(files.first());
    Release {
        source: Source::Modrinth,
        project: text(v, "project_id"),
        version_id: text(v, "id"),
        version_number: text(v, "version_number"),
        channel: match text(v, "version_type").as_str() {
            "beta" => Channel::Beta,
            "alpha" => Channel::Alpha,
            _ => Channel::Release,
        },
        file: file.map(|f| TargetFile {
            version_id: text(v, "id"),
            version_number: text(v, "version_number"),
            file_name: text(f, "filename"),
            url: text(f, "url"),
            sha1: f.get("hashes").map(|h| text(h, "sha1")).filter(|h| !h.is_empty()),
            sha256: None,
            bytes: f.get("size").and_then(Value::as_f64).unwrap_or(0.0),
        }),
        deps: v
            .get("dependencies")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|d| {
                let required = match text(d, "dependency_type").as_str() {
                    "required" => true,
                    "incompatible" => false,
                    _ => return None,
                };
                Some(Dep {
                    required,
                    project: d.get("project_id").and_then(Value::as_str).map(str::to_string),
                    version_id: d.get("version_id").and_then(Value::as_str).map(str::to_string),
                })
            })
            .collect(),
    }
}

fn modrinth_supports(v: &Value, game_version: &str, loaders: &[&str]) -> bool {
    strings(v, "game_versions").iter().any(|g| g == game_version)
        && (loaders.is_empty() || strings(v, "loaders").iter().any(|l| loaders.contains(&l.as_str())))
}

/// What each hash is on Modrinth now, and whether that version already fits the target
pub async fn modrinth_identify(
    client: &reqwest::Client,
    hashes: &[String],
    game_version: &str,
    loaders: &[&str],
) -> Result<HashMap<String, (Release, bool)>, String> {
    if hashes.is_empty() {
        return Ok(HashMap::new());
    }
    let found = send(
        client.post(format!("{MODRINTH}/version_files")).json(&json!({ "hashes": hashes, "algorithm": "sha1" })),
        "Modrinth",
    )
    .await?;
    Ok(found
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(_, v)| v.is_object())
        .map(|(hash, v)| (hash.clone(), (modrinth_release(v), modrinth_supports(v, game_version, loaders))))
        .collect())
}

/// The version for the target of each project (stable preferred)
pub async fn modrinth_target(
    client: &reqwest::Client,
    project: &str,
    game_version: &str,
    loaders: &[&str],
) -> Option<Release> {
    if loaders.is_empty() {
        return None;
    }
    let list = send(
        client.get(format!("{MODRINTH}/project/{project}/version")).query(&[
            ("loaders", serde_json::to_string(loaders).ok()?),
            ("game_versions", serde_json::to_string(&[game_version]).ok()?),
        ]),
        "Modrinth",
    )
    .await
    .ok()?;
    let list = list.as_array()?;
    list.iter()
        .find(|v| text(v, "version_type") == "release")
        .or(list.first())
        .map(modrinth_release)
}

/// The project a Modrinth version belongs to
pub async fn modrinth_project_of(client: &reqwest::Client, version_id: &str) -> Option<String> {
    let v = send(client.get(format!("{MODRINTH}/version/{version_id}")), "Modrinth").await.ok()?;
    Some(text(&v, "project_id")).filter(|p| !p.is_empty())
}

pub async fn modrinth_projects(client: &reqwest::Client, ids: &[String]) -> HashMap<String, ProjectInfo> {
    if ids.is_empty() {
        return HashMap::new();
    }
    let Ok(list) = send(
        client.get(format!("{MODRINTH}/projects")).query(&[("ids", serde_json::to_string(ids).unwrap_or_default())]),
        "Modrinth",
    )
    .await
    else {
        return HashMap::new();
    };
    list.as_array()
        .into_iter()
        .flatten()
        .map(|p| {
            let info = ProjectInfo {
                title: text(p, "title"),
                icon_url: p.get("icon_url").and_then(Value::as_str).filter(|u| !u.is_empty()).map(str::to_string),
                page_url: Some(format!("https://modrinth.com/project/{}", text(p, "slug"))),
            };
            (text(p, "id"), info)
        })
        .collect()
}

// ─── CurseForge ───────────────────────────────────────────────────────────────

/// CurseForge's file fingerprint: 32-bit MurmurHash2 (seed 1) of the file without
/// whitespace bytes (tab, newline, carriage return, space)
pub fn cf_fingerprint(data: &[u8]) -> u32 {
    let bytes: Vec<u8> = data.iter().copied().filter(|b| !matches!(b, 9 | 10 | 13 | 32)).collect();
    const M: u32 = 0x5bd1_e995;
    let mut h: u32 = 1 ^ (bytes.len() as u32);
    let mut chunks = bytes.chunks_exact(4);
    for chunk in &mut chunks {
        let mut k = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        k = k.wrapping_mul(M);
        k ^= k >> 24;
        k = k.wrapping_mul(M);
        h = h.wrapping_mul(M) ^ k;
    }
    let tail = chunks.remainder();
    if !tail.is_empty() {
        if tail.len() >= 3 {
            h ^= (tail[2] as u32) << 16;
        }
        if tail.len() >= 2 {
            h ^= (tail[1] as u32) << 8;
        }
        h ^= tail[0] as u32;
        h = h.wrapping_mul(M);
    }
    h ^= h >> 13;
    h = h.wrapping_mul(M);
    h ^= h >> 15;
    h
}

fn cf(req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    req.header("x-api-key", crate::minecraft::content::CURSEFORGE_API_KEY)
}

fn cf_release(mod_id: u64, f: &Value) -> Release {
    let hashes = f.get("hashes").and_then(Value::as_array).cloned().unwrap_or_default();
    let sha1 = hashes
        .iter()
        .find(|h| h.get("algo").and_then(Value::as_u64) == Some(1))
        .map(|h| text(h, "value"))
        .filter(|h| !h.is_empty());
    let version_id = f.get("id").and_then(Value::as_u64).unwrap_or(0).to_string();
    let version_number = text(f, "displayName");
    let url = text(f, "downloadUrl");
    Release {
        source: Source::Curseforge,
        project: mod_id.to_string(),
        version_id: version_id.clone(),
        version_number: version_number.clone(),
        channel: match f.get("releaseType").and_then(Value::as_u64) {
            Some(2) => Channel::Beta,
            Some(3) => Channel::Alpha,
            _ => Channel::Release,
        },
        // Authors can forbid downloads outside CurseForge: then there's no URL
        file: (!url.is_empty()).then(|| TargetFile {
            version_id,
            version_number,
            file_name: text(f, "fileName"),
            url,
            sha1,
            sha256: None,
            bytes: f.get("fileLength").and_then(Value::as_f64).unwrap_or(0.0),
        }),
        deps: f
            .get("dependencies")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|d| {
                // 3 = required, 5 = incompatible (1 embedded, 2 optional, 4 tool, 6 include)
                let required = match d.get("relationType").and_then(Value::as_u64) {
                    Some(3) => true,
                    Some(5) => false,
                    _ => return None,
                };
                let id = d.get("modId").and_then(Value::as_u64)?;
                Some(Dep { required, project: Some(id.to_string()), version_id: None })
            })
            .collect(),
    }
}

fn cf_supports(f: &Value, game_version: &str, scope: &Scope) -> bool {
    let versions = strings(f, "gameVersions");
    versions.iter().any(|g| g == game_version)
        && (scope.cf_loader_names.is_empty()
            || versions.iter().any(|g| scope.cf_loader_names.iter().any(|l| g.eq_ignore_ascii_case(l))))
}

/// What each fingerprint is on CurseForge, and whether it already fits the target
pub async fn cf_identify(
    client: &reqwest::Client,
    fingerprints: &[u32],
    game_version: &str,
    scope: &Scope,
) -> Result<HashMap<u32, (Release, bool)>, String> {
    if fingerprints.is_empty() {
        return Ok(HashMap::new());
    }
    let found = send(
        cf(client.post(format!("{CURSEFORGE}/fingerprints/{CF_MINECRAFT}"))).json(&json!({ "fingerprints": fingerprints })),
        "CurseForge",
    )
    .await?;
    let matches = found.pointer("/data/exactMatches").and_then(Value::as_array).cloned().unwrap_or_default();
    Ok(matches
        .iter()
        .filter_map(|m| {
            let mod_id = m.get("id").and_then(Value::as_u64)?;
            let file = m.get("file")?;
            let fingerprint = file.get("fileFingerprint").and_then(Value::as_u64)? as u32;
            Some((fingerprint, (cf_release(mod_id, file), cf_supports(file, game_version, scope))))
        })
        .collect())
}

/// The newest file of a mod for the target (stable preferred)
pub async fn cf_target(client: &reqwest::Client, mod_id: &str, game_version: &str, scope: &Scope) -> Option<Release> {
    let mut query = vec![("gameVersion", game_version.to_string()), ("pageSize", "50".to_string())];
    if let Some(loader) = scope.cf_loader {
        query.push(("modLoaderType", loader.to_string()));
    }
    let list = send(cf(client.get(format!("{CURSEFORGE}/mods/{mod_id}/files")).query(&query)), "CurseForge")
        .await
        .ok()?;
    let mut files: Vec<Value> = list
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|f| cf_supports(f, game_version, scope))
        .cloned()
        .collect();
    // Newest first
    files.sort_by(|a, b| text(b, "fileDate").cmp(&text(a, "fileDate")));
    let mod_id: u64 = mod_id.parse().ok()?;
    let pick = |kind: u64| files.iter().find(|f| f.get("releaseType").and_then(Value::as_u64) == Some(kind));
    pick(1).or_else(|| pick(2)).or_else(|| pick(3)).map(|f| cf_release(mod_id, f))
}

pub async fn cf_projects(client: &reqwest::Client, ids: &[String]) -> HashMap<String, ProjectInfo> {
    let numeric: Vec<u64> = ids.iter().filter_map(|i| i.parse().ok()).collect();
    if numeric.is_empty() {
        return HashMap::new();
    }
    let Ok(found) = send(
        cf(client.post(format!("{CURSEFORGE}/mods"))).json(&json!({ "modIds": numeric })),
        "CurseForge",
    )
    .await
    else {
        return HashMap::new();
    };
    found
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|m| {
            let id = m.get("id").and_then(Value::as_u64)?.to_string();
            let info = ProjectInfo {
                title: text(m, "name"),
                icon_url: m.pointer("/logo/thumbnailUrl").and_then(Value::as_str).map(str::to_string),
                page_url: m.pointer("/links/websiteUrl").and_then(Value::as_str).map(str::to_string),
            };
            Some((id, info))
        })
        .collect()
}

// ─── Hangar ───────────────────────────────────────────────────────────────────

fn hangar_release(project: &str, v: &Value) -> Option<Release> {
    let download = v.pointer("/downloads/PAPER")?;
    let info = download.get("fileInfo");
    let url = text(download, "downloadUrl");
    let name = text(v, "name");
    Some(Release {
        source: Source::Hangar,
        project: project.to_string(),
        version_id: name.clone(),
        version_number: name.clone(),
        channel: if v.pointer("/channel/name").and_then(Value::as_str).is_some_and(|c| c.eq_ignore_ascii_case("release")) {
            Channel::Release
        } else {
            Channel::Beta
        },
        // Hosted elsewhere: can't be downloaded automatically
        file: (!url.is_empty()).then(|| TargetFile {
            version_id: name.clone(),
            version_number: name.clone(),
            file_name: info.map(|i| text(i, "name")).unwrap_or_default(),
            url,
            sha1: None,
            sha256: info.map(|i| text(i, "sha256Hash")).filter(|h| !h.is_empty()),
            bytes: info.and_then(|i| i.get("sizeBytes")).and_then(Value::as_f64).unwrap_or(0.0),
        }),
        deps: v
            .pointer("/pluginDependencies/PAPER")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|d| d.get("required").and_then(Value::as_bool) == Some(true))
            .filter(|d| d.get("externalUrl").is_none_or(Value::is_null))
            .map(|d| Dep { required: true, project: Some(text(d, "name")), version_id: None })
            .collect(),
    })
}

/// Versions of a Hangar project for the target, newest first
pub async fn hangar_versions(client: &reqwest::Client, project: &str, game_version: &str) -> Vec<Release> {
    let Ok(found) = send(
        client.get(format!("{HANGAR}/projects/{project}/versions")).query(&[
            ("platform", "PAPER"),
            ("platformVersion", game_version),
            ("limit", "25"),
        ]),
        "Hangar",
    )
    .await
    else {
        return Vec::new();
    };
    found
        .get("result")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|v| hangar_release(project, v))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curseforge_fingerprint() {
        // Whitespace doesn't count
        assert_eq!(cf_fingerprint(b"a b\tc\r\nd"), cf_fingerprint(b"abcd"));
        assert_eq!(cf_fingerprint(b""), cf_fingerprint(b" \n\t"));
        // Reference values (MurmurHash2, seed 1)
        assert_eq!(cf_fingerprint(b""), 0x5bd1_5e36);
        assert_eq!(cf_fingerprint(b"abcd"), cf_fingerprint(b"abcd"));
        assert_ne!(cf_fingerprint(b"abcd"), cf_fingerprint(b"abce"));
    }
}
