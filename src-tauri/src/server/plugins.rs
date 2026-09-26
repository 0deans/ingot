//! Server plugins (Paper/Purpur/Folia) and server-side mods (Fabric): searching
//! Modrinth and Hangar, installing with required dependencies, and managing the
//! installed jars.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::config::ServerCoreType;

use crate::USER_AGENT;
const MODRINTH: &str = "https://api.modrinth.com/v2";
const HANGAR: &str = "https://hangar.papermc.io/api/v1";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum PluginSource {
    Modrinth,
    Hangar,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginProject {
    pub id: String,
    pub source: PluginSource,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub icon_url: Option<String>,
    pub downloads: u32,
    pub categories: Vec<String>,
    pub page_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginSearchResult {
    pub items: Vec<PluginProject>,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginDependency {
    /// Modrinth project id or Hangar project slug
    pub project_id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginVersion {
    pub id: String,
    pub version_number: String,
    /// "release" | "beta" | "alpha"
    pub channel: String,
    pub game_versions: Vec<String>,
    pub date: String,
    pub downloads: u32,
    pub file_name: String,
    /// None when the author only links to an external site
    pub download_url: Option<String>,
    pub external_url: Option<String>,
    pub size: u32,
    pub dependencies: Vec<PluginDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPlugin {
    pub file_name: String,
    pub enabled: bool,
    pub size: u32,
    /// From plugin.yml / fabric.mod.json, else the file name
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Vec<String>,
    /// Set when installed through Ingot
    pub source: Option<PluginSource>,
    pub project_id: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginUpdate {
    pub file_name: String,
    pub current_version: Option<String>,
    pub latest: PluginVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstallReport {
    /// Titles of everything installed, dependencies included
    pub installed: Vec<String>,
    pub warnings: Vec<String>,
}

/// What this server core can load
pub struct Platform {
    /// "plugins" or "mods"
    pub folder: &'static str,
    modrinth_type: &'static str,
    modrinth_loaders: &'static [&'static str],
    hangar: bool,
}

pub fn platform(core: &ServerCoreType) -> Option<Platform> {
    let plugins = |loaders: &'static [&'static str], hangar: bool| Platform {
        folder: "plugins",
        modrinth_type: "plugin",
        modrinth_loaders: loaders,
        hangar,
    };
    match core {
        ServerCoreType::Paper => Some(plugins(&["paper", "spigot", "bukkit"], true)),
        ServerCoreType::Purpur => Some(plugins(&["purpur", "paper", "spigot", "bukkit"], true)),
        // Folia needs plugins written for its threading model
        ServerCoreType::Folia => Some(plugins(&["folia"], false)),
        ServerCoreType::Fabric => Some(Platform {
            folder: "mods",
            modrinth_type: "mod",
            modrinth_loaders: &["fabric"],
            hangar: false,
        }),
        ServerCoreType::Vanilla | ServerCoreType::Pumpkin => None,
    }
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))
}

async fn get_json(client: &reqwest::Client, url: &str, query: &[(&str, String)]) -> Result<Value, String> {
    let res = client
        .get(url)
        .query(query)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("{url} returned HTTP {}", res.status()));
    }
    res.json().await.map_err(|e| format!("Invalid response from {url}: {e}"))
}

fn s(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

fn n(v: &Value, key: &str) -> u32 {
    v.get(key).and_then(Value::as_u64).unwrap_or(0).min(u32::MAX as u64) as u32
}

fn strings(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

// ─── Search ───────────────────────────────────────────────────────────────────

pub async fn search(
    core: &ServerCoreType,
    game_version: &str,
    source: PluginSource,
    query: &str,
    sort: &str,
    compatible_only: bool,
    page: u32,
    page_size: u32,
) -> Result<PluginSearchResult, String> {
    let platform = platform(core).ok_or("This server type doesn't support plugins or mods")?;
    let client = client()?;
    let offset = (page * page_size).to_string();
    let limit = page_size.to_string();

    match source {
        PluginSource::Modrinth => {
            let mut facets = vec![
                vec![format!("project_type:{}", platform.modrinth_type)],
                platform.modrinth_loaders.iter().map(|l| format!("categories:{l}")).collect(),
            ];
            if platform.modrinth_type == "mod" {
                facets.push(vec!["server_side:required".into(), "server_side:optional".into()]);
            }
            if compatible_only {
                facets.push(vec![format!("versions:{game_version}")]);
            }
            let index = match sort {
                "relevance" => "relevance",
                "updated" => "updated",
                "newest" => "newest",
                _ if !query.trim().is_empty() => "relevance",
                _ => "downloads",
            };
            let mut params = vec![
                ("facets", serde_json::to_string(&facets).unwrap_or_default()),
                ("index", index.to_string()),
                ("offset", offset),
                ("limit", limit),
            ];
            if !query.trim().is_empty() {
                params.push(("query", query.trim().to_string()));
            }
            let res = get_json(&client, &format!("{MODRINTH}/search"), &params).await?;
            let items = res
                .get("hits")
                .and_then(Value::as_array)
                .map(|hits| {
                    hits.iter()
                        .map(|h| PluginProject {
                            id: s(h, "project_id"),
                            source: PluginSource::Modrinth,
                            slug: s(h, "slug"),
                            title: s(h, "title"),
                            description: s(h, "description"),
                            author: s(h, "author"),
                            icon_url: h.get("icon_url").and_then(Value::as_str).filter(|u| !u.is_empty()).map(str::to_string),
                            downloads: n(h, "downloads"),
                            categories: strings(h, "display_categories"),
                            page_url: format!("https://modrinth.com/{}/{}", platform.modrinth_type, s(h, "slug")),
                        })
                        .collect()
                })
                .unwrap_or_default();
            Ok(PluginSearchResult { items, total: n(&res, "total_hits") })
        }
        PluginSource::Hangar => {
            if !platform.hangar {
                return Ok(PluginSearchResult { items: Vec::new(), total: 0 });
            }
            let sort = match sort {
                "updated" => "-updated",
                "newest" => "-newest",
                "stars" => "-stars",
                _ => "-downloads",
            };
            let mut params = vec![
                ("platform", "PAPER".to_string()),
                ("sort", sort.to_string()),
                ("offset", offset),
                ("limit", limit),
            ];
            if !query.trim().is_empty() {
                params.push(("q", query.trim().to_string()));
            }
            if compatible_only {
                params.push(("version", game_version.to_string()));
            }
            let res = get_json(&client, &format!("{HANGAR}/projects"), &params).await?;
            let items = res
                .get("result")
                .and_then(Value::as_array)
                .map(|projects| projects.iter().map(hangar_project).collect())
                .unwrap_or_default();
            let total = res.get("pagination").map(|p| n(p, "count")).unwrap_or(0);
            Ok(PluginSearchResult { items, total })
        }
    }
}

fn hangar_project(p: &Value) -> PluginProject {
    let ns = p.get("namespace").cloned().unwrap_or(Value::Null);
    let (owner, slug) = (s(&ns, "owner"), s(&ns, "slug"));
    PluginProject {
        id: slug.clone(),
        source: PluginSource::Hangar,
        slug: slug.clone(),
        title: s(p, "name"),
        description: s(p, "description"),
        author: owner.clone(),
        icon_url: p.get("avatarUrl").and_then(Value::as_str).map(str::to_string),
        downloads: p.get("stats").map(|st| n(st, "downloads")).unwrap_or(0),
        categories: vec![s(p, "category").replace('_', " ")],
        page_url: format!("https://hangar.papermc.io/{owner}/{slug}"),
    }
}

/// Full description (Markdown) of a project
pub async fn page(source: PluginSource, project_id: &str) -> Result<String, String> {
    let client = client()?;
    match source {
        PluginSource::Modrinth => {
            let project = get_json(&client, &format!("{MODRINTH}/project/{project_id}"), &[]).await?;
            Ok(s(&project, "body"))
        }
        PluginSource::Hangar => {
            let res = client
                .get(format!("{HANGAR}/pages/main/{project_id}"))
                .send()
                .await
                .map_err(|e| format!("Request failed: {e}"))?;
            res.text().await.map_err(|e| e.to_string())
        }
    }
}

// ─── Versions ─────────────────────────────────────────────────────────────────

/// Versions of a project for this server, newest first
pub async fn versions(
    core: &ServerCoreType,
    game_version: &str,
    source: PluginSource,
    project_id: &str,
    compatible_only: bool,
) -> Result<Vec<PluginVersion>, String> {
    let platform = platform(core).ok_or("This server type doesn't support plugins or mods")?;
    let client = client()?;
    match source {
        PluginSource::Modrinth => {
            let mut params = vec![("loaders", serde_json::to_string(platform.modrinth_loaders).unwrap_or_default())];
            if compatible_only {
                params.push(("game_versions", serde_json::to_string(&[game_version]).unwrap_or_default()));
            }
            let res = get_json(&client, &format!("{MODRINTH}/project/{project_id}/version"), &params).await?;
            Ok(res
                .as_array()
                .map(|vs| vs.iter().filter_map(modrinth_version).collect())
                .unwrap_or_default())
        }
        PluginSource::Hangar => {
            let mut params = vec![("platform", "PAPER".to_string()), ("limit", "25".to_string())];
            if compatible_only {
                params.push(("platformVersion", game_version.to_string()));
            }
            let res = get_json(&client, &format!("{HANGAR}/projects/{project_id}/versions"), &params).await?;
            Ok(res
                .get("result")
                .and_then(Value::as_array)
                .map(|vs| vs.iter().filter_map(hangar_version).collect())
                .unwrap_or_default())
        }
    }
}

fn modrinth_version(v: &Value) -> Option<PluginVersion> {
    let files = v.get("files")?.as_array()?;
    let file = files
        .iter()
        .find(|f| f.get("primary").and_then(Value::as_bool) == Some(true))
        .or_else(|| files.first())?;
    let dependencies = v
        .get("dependencies")
        .and_then(Value::as_array)
        .map(|deps| {
            deps.iter()
                .filter(|d| s(d, "dependency_type") == "required")
                .filter_map(|d| {
                    let id = d.get("project_id").and_then(Value::as_str)?;
                    Some(PluginDependency { project_id: id.to_string(), name: None })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(PluginVersion {
        id: s(v, "id"),
        version_number: s(v, "version_number"),
        channel: v.get("version_type").and_then(Value::as_str).unwrap_or("release").to_string(),
        game_versions: strings(v, "game_versions"),
        date: s(v, "date_published"),
        downloads: n(v, "downloads"),
        file_name: s(file, "filename"),
        download_url: file.get("url").and_then(Value::as_str).map(str::to_string),
        external_url: None,
        size: n(file, "size"),
        dependencies,
    })
}

fn hangar_version(v: &Value) -> Option<PluginVersion> {
    let download = v.get("downloads")?.get("PAPER")?;
    let file = download.get("fileInfo");
    let channel = v
        .get("channel")
        .map(|c| s(c, "name").to_lowercase())
        .map(|c| if c == "release" { c } else { "beta".to_string() })
        .unwrap_or_else(|| "release".to_string());
    let dependencies = v
        .get("pluginDependencies")
        .and_then(|d| d.get("PAPER"))
        .and_then(Value::as_array)
        .map(|deps| {
            deps.iter()
                .filter(|d| d.get("required").and_then(Value::as_bool) == Some(true))
                // Dependencies hosted elsewhere can't be installed automatically
                .filter(|d| d.get("externalUrl").is_none_or(Value::is_null))
                .map(|d| PluginDependency { project_id: s(d, "name"), name: Some(s(d, "name")) })
                .collect()
        })
        .unwrap_or_default();
    Some(PluginVersion {
        id: s(v, "name"),
        version_number: s(v, "name"),
        channel,
        game_versions: v
            .get("platformDependencies")
            .map(|p| strings(p, "PAPER"))
            .unwrap_or_default(),
        date: s(v, "createdAt"),
        downloads: v.get("stats").map(|st| n(st, "totalDownloads")).unwrap_or(0),
        file_name: file.map(|f| s(f, "name")).unwrap_or_default(),
        download_url: download.get("downloadUrl").and_then(Value::as_str).map(str::to_string),
        external_url: download.get("externalUrl").and_then(Value::as_str).map(str::to_string),
        size: file.map(|f| n(f, "sizeBytes")).unwrap_or(0),
        dependencies,
    })
}

/// Newest stable version, falling back to pre-releases
fn pick_version(versions: &[PluginVersion]) -> Option<&PluginVersion> {
    let installable = || versions.iter().filter(|v| v.download_url.is_some());
    installable().find(|v| v.channel == "release").or_else(|| installable().next())
}

// ─── Installed plugins ────────────────────────────────────────────────────────

/// What Ingot remembers about jars it installed (for icons and update checks)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrackedPlugin {
    file_name: String,
    source: PluginSource,
    project_id: String,
    version_id: String,
    version_number: String,
    title: String,
    icon_url: Option<String>,
}

fn tracking_path(server_dir: &Path) -> PathBuf {
    server_dir.join(".ingot").join("plugins.json")
}

fn read_tracking(server_dir: &Path) -> Vec<TrackedPlugin> {
    std::fs::read_to_string(tracking_path(server_dir))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn write_tracking(server_dir: &Path, tracked: &[TrackedPlugin]) {
    let path = tracking_path(server_dir);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(tracked) {
        let _ = std::fs::write(path, raw);
    }
}

/// Only plain jar names inside the plugins/mods folder
fn safe_jar_name(name: &str) -> Result<&str, String> {
    let base = name.strip_suffix(".disabled").unwrap_or(name);
    let plain = Path::new(name).file_name().and_then(|f| f.to_str()) == Some(name);
    if !plain || !base.to_ascii_lowercase().ends_with(".jar") || name.starts_with('.') {
        return Err(format!("Not a plugin file: {name}"));
    }
    Ok(name)
}

fn folder(server_dir: &Path, core: &ServerCoreType) -> Result<PathBuf, String> {
    let platform = platform(core).ok_or("This server type doesn't support plugins or mods")?;
    Ok(server_dir.join(platform.folder))
}

/// name/version/description/authors from plugin.yml, paper-plugin.yml or fabric.mod.json
fn read_jar_metadata(path: &Path) -> Option<(String, Option<String>, Option<String>, Vec<String>)> {
    let file = std::fs::File::open(path).ok()?;
    let mut zip = zip::ZipArchive::new(file).ok()?;
    let mut read = |name: &str| -> Option<String> {
        let mut entry = zip.by_name(name).ok()?;
        let mut text = String::new();
        entry.read_to_string(&mut text).ok()?;
        Some(text)
    };

    if let Some(json) = read("fabric.mod.json").and_then(|t| serde_json::from_str::<Value>(&t).ok()) {
        let authors = json
            .get("authors")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_string).or_else(|| x.get("name")?.as_str().map(str::to_string)))
                    .collect()
            })
            .unwrap_or_default();
        let name = json.get("name").or_else(|| json.get("id"))?.as_str()?.to_string();
        return Some((name, json.get("version").and_then(Value::as_str).map(str::to_string), json.get("description").and_then(Value::as_str).map(str::to_string), authors));
    }

    let yaml = read("paper-plugin.yml").or_else(|| read("plugin.yml"))?;
    // Only top-level scalar keys are needed, so a tiny line parser is enough
    let top = |key: &str| -> Option<String> {
        yaml.lines().find_map(|line| {
            let rest = line.strip_prefix(key)?.strip_prefix(':')?;
            let value = rest.trim().trim_matches(|c| c == '"' || c == '\'').trim();
            (!value.is_empty() && value != "|" && value != ">").then(|| value.to_string())
        })
    };
    let name = top("name")?;
    let mut authors: Vec<String> = top("author").into_iter().collect();
    if let Some(list) = top("authors") {
        authors.extend(
            list.trim_matches(|c| c == '[' || c == ']')
                .split(',')
                .map(|a| a.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
                .filter(|a| !a.is_empty()),
        );
    }
    Some((name, top("version"), top("description"), authors))
}

pub fn list_installed(server_dir: &Path, core: &ServerCoreType) -> Result<Vec<InstalledPlugin>, String> {
    let dir = folder(server_dir, core)?;
    let tracked = read_tracking(server_dir);
    let Ok(entries) = std::fs::read_dir(&dir) else { return Ok(Vec::new()) };
    let mut plugins: Vec<InstalledPlugin> = entries
        .flatten()
        .filter_map(|entry| {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            safe_jar_name(&file_name).ok()?;
            let meta = entry.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            let enabled = !file_name.ends_with(".disabled");
            let jar_name = file_name.trim_end_matches(".disabled");
            let track = tracked.iter().find(|t| t.file_name == jar_name);
            let (name, version, description, authors) = read_jar_metadata(&entry.path())
                .unwrap_or_else(|| (jar_name.trim_end_matches(".jar").to_string(), None, None, Vec::new()));
            Some(InstalledPlugin {
                file_name: file_name.clone(),
                enabled,
                size: meta.len().min(u32::MAX as u64) as u32,
                name,
                version: version.or_else(|| track.map(|t| t.version_number.clone())),
                description,
                authors,
                source: track.map(|t| t.source),
                project_id: track.map(|t| t.project_id.clone()),
                icon_url: track.and_then(|t| t.icon_url.clone()),
            })
        })
        .collect();
    plugins.sort_by_key(|p| p.name.to_lowercase());
    Ok(plugins)
}

pub fn set_enabled(server_dir: &Path, core: &ServerCoreType, file_name: &str, enabled: bool) -> Result<(), String> {
    let dir = folder(server_dir, core)?;
    let file_name = safe_jar_name(file_name)?;
    let jar = file_name.trim_end_matches(".disabled");
    let (from, to) = if enabled {
        (format!("{jar}.disabled"), jar.to_string())
    } else {
        (jar.to_string(), format!("{jar}.disabled"))
    };
    if dir.join(&to).exists() {
        return Ok(());
    }
    std::fs::rename(dir.join(&from), dir.join(&to)).map_err(|e| format!("Failed to rename {from}: {e}"))
}

pub fn remove(server_dir: &Path, core: &ServerCoreType, file_name: &str) -> Result<(), String> {
    let dir = folder(server_dir, core)?;
    let file_name = safe_jar_name(file_name)?;
    std::fs::remove_file(dir.join(file_name)).map_err(|e| format!("Failed to delete {file_name}: {e}"))?;
    let jar = file_name.trim_end_matches(".disabled");
    let mut tracked = read_tracking(server_dir);
    tracked.retain(|t| t.file_name != jar);
    write_tracking(server_dir, &tracked);
    Ok(())
}

// ─── Installing ───────────────────────────────────────────────────────────────

async fn download(client: &reqwest::Client, url: &str, dest: &Path) -> Result<(), String> {
    let res = client.get(url).send().await.map_err(|e| format!("Download failed: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("Download failed: HTTP {}", res.status()));
    }
    let bytes = res.bytes().await.map_err(|e| format!("Download failed: {e}"))?;
    // Write next to the target first so a failed download never leaves a broken jar
    let tmp = dest.with_extension("jar.part");
    std::fs::write(&tmp, &bytes).map_err(|e| format!("Failed to save plugin: {e}"))?;
    std::fs::rename(&tmp, dest).map_err(|e| format!("Failed to save plugin: {e}"))
}

async fn project_title(client: &reqwest::Client, source: PluginSource, id: &str) -> (String, Option<String>) {
    let url = match source {
        PluginSource::Modrinth => format!("{MODRINTH}/project/{id}"),
        PluginSource::Hangar => format!("{HANGAR}/projects/{id}"),
    };
    match get_json(client, &url, &[]).await {
        Ok(p) => match source {
            PluginSource::Modrinth => (s(&p, "title"), p.get("icon_url").and_then(Value::as_str).map(str::to_string)),
            PluginSource::Hangar => (s(&p, "name"), p.get("avatarUrl").and_then(Value::as_str).map(str::to_string)),
        },
        Err(_) => (id.to_string(), None),
    }
}

/// Installs a project (a specific version, or the best compatible one) plus its
/// required dependencies that aren't installed yet
pub async fn install(
    server_dir: &Path,
    core: &ServerCoreType,
    game_version: &str,
    source: PluginSource,
    project_id: &str,
    version_id: Option<&str>,
) -> Result<InstallReport, String> {
    let dir = folder(server_dir, core)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create {}: {e}", dir.display()))?;
    let client = client()?;
    let mut report = InstallReport { installed: Vec::new(), warnings: Vec::new() };
    let installed_names: HashSet<String> = list_installed(server_dir, core)?
        .iter()
        .map(|p| p.name.to_lowercase())
        .collect();

    // Breadth-first over the dependency tree; `seen` stops cycles
    let mut queue: Vec<(String, Option<String>, bool)> = vec![(project_id.to_string(), version_id.map(str::to_string), true)];
    let mut seen = HashSet::new();
    while let Some((id, wanted_version, is_root)) = queue.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let tracked = read_tracking(server_dir);
        if !is_root && (tracked.iter().any(|t| t.project_id == id) || installed_names.contains(&id.to_lowercase())) {
            continue;
        }

        let (title, icon_url) = project_title(&client, source, &id).await;
        if !is_root && installed_names.contains(&title.to_lowercase()) {
            continue;
        }
        let all = versions(core, game_version, source, &id, true).await.unwrap_or_default();
        let version = match &wanted_version {
            Some(v) => {
                let any = versions(core, game_version, source, &id, false).await.unwrap_or_default();
                any.into_iter().find(|x| &x.id == v)
            }
            None => pick_version(&all).cloned(),
        };
        let Some(version) = version else {
            let msg = format!("{title} has no downloadable version for Minecraft {game_version}");
            if is_root {
                return Err(msg);
            }
            report.warnings.push(msg);
            continue;
        };
        let Some(url) = version.download_url.clone() else {
            report.warnings.push(format!("{title} must be downloaded from its website"));
            continue;
        };
        let file_name = safe_jar_name(&version.file_name)?.to_string();

        // Replace older versions of the same project
        let mut tracked = read_tracking(server_dir);
        for old in tracked.iter().filter(|t| t.project_id == id && t.file_name != file_name) {
            let _ = std::fs::remove_file(dir.join(&old.file_name));
            let _ = std::fs::remove_file(dir.join(format!("{}.disabled", old.file_name)));
        }
        download(&client, &url, &dir.join(&file_name)).await?;

        tracked.retain(|t| t.project_id != id && t.file_name != file_name);
        tracked.push(TrackedPlugin {
            file_name,
            source,
            project_id: id.clone(),
            version_id: version.id.clone(),
            version_number: version.version_number.clone(),
            title: title.clone(),
            icon_url,
        });
        write_tracking(server_dir, &tracked);
        report.installed.push(title);

        for dep in &version.dependencies {
            queue.push((dep.project_id.clone(), None, false));
        }
    }
    Ok(report)
}

/// Newer compatible versions of plugins installed through Ingot
pub async fn check_updates(server_dir: &Path, core: &ServerCoreType, game_version: &str) -> Vec<PluginUpdate> {
    let mut updates = Vec::new();
    for t in read_tracking(server_dir) {
        let Ok(all) = versions(core, game_version, t.source, &t.project_id, true).await else { continue };
        if let Some(latest) = pick_version(&all) {
            if latest.id != t.version_id {
                updates.push(PluginUpdate {
                    file_name: t.file_name.clone(),
                    current_version: Some(t.version_number.clone()),
                    latest: latest.clone(),
                });
            }
        }
    }
    updates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jar_names_are_restricted() {
        assert!(safe_jar_name("EssentialsX-2.21.jar").is_ok());
        assert!(safe_jar_name("EssentialsX-2.21.jar.disabled").is_ok());
        assert!(safe_jar_name("../evil.jar").is_err());
        assert!(safe_jar_name("sub/dir.jar").is_err());
        assert!(safe_jar_name("config.yml").is_err());
        assert!(safe_jar_name(".hidden.jar").is_err());
    }

    #[test]
    fn prefers_stable_versions() {
        let v = |id: &str, channel: &str, url: bool| PluginVersion {
            id: id.into(),
            version_number: id.into(),
            channel: channel.into(),
            game_versions: vec![],
            date: String::new(),
            downloads: 0,
            file_name: format!("{id}.jar"),
            download_url: url.then(|| "https://x".into()),
            external_url: None,
            size: 0,
            dependencies: vec![],
        };
        let list = [v("3-beta", "beta", true), v("2", "release", false), v("1", "release", true)];
        assert_eq!(pick_version(&list).map(|v| v.id.as_str()), Some("1"));
        assert_eq!(pick_version(&list[..1]).map(|v| v.id.as_str()), Some("3-beta"));
    }

    /// Hits the real APIs: cargo test -- --ignored plugins_live
    #[tokio::test]
    #[ignore]
    async fn plugins_live() {
        for source in [PluginSource::Modrinth, PluginSource::Hangar] {
            let res = search(&ServerCoreType::Paper, "26.2", source, "luckperms", "", true, 0, 5).await.unwrap();
            println!("{source:?}: {} results, first = {:?}", res.total, res.items.first().map(|p| &p.title));
            let first = &res.items[0];
            let vs = versions(&ServerCoreType::Paper, "26.2", source, &first.id, true).await.unwrap();
            println!("  {} versions, best = {:?}", vs.len(), pick_version(&vs).map(|v| (&v.version_number, &v.file_name)));
        }
    }
}
