//! Moving an instance to another Minecraft version without breaking it.
//!
//! 1. `check` changes nothing: it identifies every mod, resource pack and shader by its
//!    hash on Modrinth and works out, for the target version, what still works, what has
//!    an update, what's missing, which new dependencies are needed and what conflicts.
//! 2. `apply` carries out the reviewed plan all or nothing: new files are downloaded and
//!    verified in a staging folder first, the current files, configs (and worlds, if
//!    asked) are snapshotted, and only then is everything swapped in. A failure part way
//!    puts everything back.
//! 3. `undo` restores the snapshot of the last change exactly.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::instance::{InstanceConfig, ModLoaderType};

const MODRINTH: &str = "https://api.modrinth.com/v2";
/// Everything version change keeps lives here, inside the instance
const WORK_DIR: &str = ".ingot/version-change";
const JOURNAL: &str = "journal.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ContentKind {
    Mod,
    ResourcePack,
    Shader,
}

impl ContentKind {
    const ALL: [ContentKind; 3] = [ContentKind::Mod, ContentKind::ResourcePack, ContentKind::Shader];

    fn folder(self) -> &'static str {
        match self {
            Self::Mod => "mods",
            Self::ResourcePack => "resourcepacks",
            Self::Shader => "shaderpacks",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Mod => ".jar",
            Self::ResourcePack | Self::Shader => ".zip",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ItemStatus {
    /// The current file already supports the target version
    Works,
    /// A version for the target exists
    Update,
    /// Nothing for the target yet
    Missing,
    /// Not found on Modrinth (added by hand), so it can't be checked
    Unknown,
    /// Needed by an updated mod and not installed yet
    NewDependency,
    /// Same mod twice, incompatible with another mod, or needs a mod that isn't available
    Conflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ItemAction {
    /// Leave the file as it is
    Keep,
    /// Replace it with the target version
    Update,
    /// Turn it off (renamed to .disabled, nothing is deleted)
    Disable,
    /// Install a new dependency
    Add,
    /// Don't install a new dependency
    Skip,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TargetFile {
    pub version_id: String,
    pub version_number: String,
    pub file_name: String,
    pub url: String,
    pub sha1: String,
    #[specta(type = i32)]
    pub bytes: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanItem {
    pub kind: ContentKind,
    /// Current file; None for a new dependency
    pub file_name: Option<String>,
    pub title: String,
    pub icon_url: Option<String>,
    pub project_id: Option<String>,
    pub current_version: Option<String>,
    /// What an update or a new dependency installs
    pub target: Option<TargetFile>,
    pub status: ItemStatus,
    /// Recommended action (the user may pick another from `actions`)
    pub action: ItemAction,
    pub actions: Vec<ItemAction>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WorldInfo {
    pub folder_name: String,
    #[specta(type = i32)]
    pub bytes: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VersionPlan {
    pub instance_id: String,
    pub loader: ModLoaderType,
    pub from_game_version: String,
    pub from_loader_version: Option<String>,
    pub to_game_version: String,
    pub to_loader_version: Option<String>,
    /// Worlds opened in a newer version can't be opened in an older one
    pub downgrade: bool,
    pub items: Vec<PlanItem>,
    pub worlds: Vec<WorldInfo>,
}

/// The last change, for undo
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VersionBackup {
    pub from_game_version: String,
    pub to_game_version: String,
    #[specta(type = i32)]
    pub created_at: u64,
    pub worlds_backed_up: bool,
    #[specta(type = i32)]
    pub bytes: f64,
}

/// Everything `apply` changed, in order, so it can be put back
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Journal {
    created_at: u64,
    from_game_version: String,
    from_loader_version: Option<String>,
    to_game_version: String,
    /// Files moved into the backup: (path in the instance, path in the backup)
    moved: Vec<(String, String)>,
    /// Files renamed to <name>.disabled
    disabled: Vec<String>,
    /// Files put in place
    added: Vec<String>,
    config_backed_up: bool,
    options_backed_up: bool,
    worlds: Vec<String>,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn work_dir(instance_dir: &Path) -> PathBuf {
    instance_dir.join(WORK_DIR)
}

fn backup_dir(instance_dir: &Path) -> PathBuf {
    work_dir(instance_dir).join("backup")
}

fn sha1_of(path: &Path) -> Option<String> {
    use sha1::{Digest, Sha1};
    let data = fs::read(path).ok()?;
    Some(Sha1::digest(&data).iter().map(|b| format!("{b:02x}")).collect())
}

fn dir_size(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else { return 0 };
    if meta.is_file() {
        return meta.len();
    }
    if !meta.is_dir() {
        return 0;
    }
    fs::read_dir(path).map(|e| e.flatten().map(|e| dir_size(&e.path())).sum()).unwrap_or(0)
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)?.flatten() {
        let kind = entry.file_type()?;
        let dest = to.join(entry.file_name());
        if kind.is_dir() {
            copy_dir(&entry.path(), &dest)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

/// Moves a file or folder, creating the destination's parent
fn move_path(from: &Path, to: &Path) -> Result<(), String> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create {}: {e}", parent.display()))?;
    }
    fs::rename(from, to).map_err(|e| format!("Failed to move {}: {e}", from.display()))
}

/// Only a plain file name (no folders), with the expected extension
fn safe_file_name(name: &str, kind: ContentKind) -> Result<&str, String> {
    let ok = !name.is_empty()
        && !name.starts_with('.')
        && !name.contains(['/', '\\', ':'])
        && name.to_ascii_lowercase().ends_with(kind.extension());
    ok.then_some(name).ok_or_else(|| format!("Unexpected file name: {name}"))
}

/// Modrinth loader names for mods on this loader (Quilt also runs Fabric mods)
fn mod_loaders(loader: &ModLoaderType) -> Vec<&'static str> {
    match loader {
        ModLoaderType::Fabric => vec!["fabric"],
        ModLoaderType::Quilt => vec!["quilt", "fabric"],
        ModLoaderType::NeoForge => vec!["neoforge"],
        ModLoaderType::Forge => vec!["forge"],
        ModLoaderType::Vanilla => vec![],
    }
}

fn loaders_for(kind: ContentKind, loader: &ModLoaderType) -> Vec<&'static str> {
    match kind {
        ContentKind::Mod => mod_loaders(loader),
        ContentKind::ResourcePack => vec!["minecraft"],
        ContentKind::Shader => vec!["iris", "optifine", "canvas", "vanilla"],
    }
}

fn strings(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

fn text(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

/// The version's main file
fn target_file(version: &Value) -> Option<TargetFile> {
    let files = version.get("files")?.as_array()?;
    let file = files.iter().find(|f| f.get("primary").and_then(Value::as_bool) == Some(true)).or(files.first())?;
    Some(TargetFile {
        version_id: text(version, "id"),
        version_number: text(version, "version_number"),
        file_name: text(file, "filename"),
        url: text(file, "url"),
        sha1: file.get("hashes").map(|h| text(h, "sha1")).unwrap_or_default(),
        bytes: file.get("size").and_then(Value::as_f64).unwrap_or(0.0),
    })
}

/// A warning when the only version for the target is a beta or an alpha
fn unstable(version: &Value) -> Option<String> {
    match text(version, "version_type").as_str() {
        "beta" => Some("Beta version, may be unstable".into()),
        "alpha" => Some("Alpha version, may be unstable".into()),
        _ => None,
    }
}

fn supports(version: &Value, game_version: &str, loaders: &[&str]) -> bool {
    strings(version, "game_versions").iter().any(|g| g == game_version)
        && (loaders.is_empty() || strings(version, "loaders").iter().any(|l| loaders.contains(&l.as_str())))
}

async fn post(client: &reqwest::Client, path: &str, body: Value) -> Result<Value, String> {
    client
        .post(format!("{MODRINTH}{path}"))
        .json(&body)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Modrinth request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Invalid response from Modrinth: {e}"))
}

async fn get(client: &reqwest::Client, path: &str, query: &[(&str, String)]) -> Result<Value, String> {
    client
        .get(format!("{MODRINTH}{path}"))
        .query(query)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Modrinth request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Invalid response from Modrinth: {e}"))
}

/// Newest release (else newest of any kind) of a project for the target
async fn best_version(client: &reqwest::Client, project_id: &str, game_version: &str, loaders: &[&str]) -> Option<Value> {
    let list = get(
        client,
        &format!("/project/{project_id}/version"),
        &[
            ("loaders", serde_json::to_string(loaders).ok()?),
            ("game_versions", serde_json::to_string(&[game_version]).ok()?),
        ],
    )
    .await
    .ok()?;
    let list = list.as_array()?;
    list.iter()
        .find(|v| text(v, "version_type") == "release")
        .or(list.first())
        .cloned()
}

// ─── Check ────────────────────────────────────────────────────────────────────

struct LocalFile {
    kind: ContentKind,
    file_name: String,
    sha1: String,
}

fn scan(instance_dir: &Path, loader: &ModLoaderType) -> Vec<LocalFile> {
    let mut out = Vec::new();
    for kind in ContentKind::ALL {
        // Mods don't load on vanilla, so there's nothing to check
        if kind == ContentKind::Mod && *loader == ModLoaderType::Vanilla {
            continue;
        }
        let Ok(entries) = fs::read_dir(instance_dir.join(kind.folder())) else { continue };
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if !entry.path().is_file() || safe_file_name(&file_name, kind).is_err() {
                continue;
            }
            if let Some(sha1) = sha1_of(&entry.path()) {
                out.push(LocalFile { kind, file_name, sha1 });
            }
        }
    }
    out.sort_by(|a, b| a.file_name.to_lowercase().cmp(&b.file_name.to_lowercase()));
    out
}

pub fn list_worlds(instance_dir: &Path) -> Vec<WorldInfo> {
    let Ok(entries) = fs::read_dir(instance_dir.join("saves")) else { return Vec::new() };
    let mut worlds: Vec<WorldInfo> = entries
        .flatten()
        .filter(|e| e.path().join("level.dat").is_file())
        .map(|e| WorldInfo { folder_name: e.file_name().to_string_lossy().into_owned(), bytes: dir_size(&e.path()) as f64 })
        .collect();
    worlds.sort_by(|a, b| a.folder_name.cmp(&b.folder_name));
    worlds
}

/// Works out what moving to `to_game_version` means for every file. Changes nothing.
pub async fn check(
    client: &reqwest::Client,
    instance_dir: &Path,
    instance: &InstanceConfig,
    to_game_version: &str,
    to_loader_version: Option<String>,
    downgrade: bool,
) -> Result<VersionPlan, String> {
    let loader = &instance.loader;
    let local = scan(instance_dir, loader);
    let hashes: Vec<&str> = local.iter().map(|f| f.sha1.as_str()).collect();

    // What each file is now, and its version for the target (per kind: different loaders)
    let current = if hashes.is_empty() {
        Value::Null
    } else {
        post(client, "/version_files", json!({ "hashes": hashes, "algorithm": "sha1" })).await?
    };
    let mut updates: HashMap<String, Value> = HashMap::new();
    for kind in ContentKind::ALL {
        let kind_hashes: Vec<&str> = local.iter().filter(|f| f.kind == kind).map(|f| f.sha1.as_str()).collect();
        if kind_hashes.is_empty() {
            continue;
        }
        let found = post(
            client,
            "/version_files/update",
            json!({
                "hashes": kind_hashes,
                "algorithm": "sha1",
                "loaders": loaders_for(kind, loader),
                "game_versions": [to_game_version],
            }),
        )
        .await?;
        if let Some(map) = found.as_object() {
            updates.extend(map.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
    }

    // One entry per file, plus the Modrinth version it would end up on
    let mut items: Vec<PlanItem> = Vec::new();
    let mut resulting: Vec<Option<Value>> = Vec::new();
    for file in &local {
        let loaders = loaders_for(file.kind, loader);
        let now = current.get(&file.sha1).filter(|v| v.is_object());
        let mut update = updates.get(&file.sha1).filter(|v| supports(v, to_game_version, &loaders)).cloned();
        // The bulk lookup returns the newest version, alphas included: prefer a release
        if let Some(found) = update.as_ref().filter(|u| text(u, "version_type") != "release") {
            if let Some(release) = best_version(client, &text(found, "project_id"), to_game_version, &loaders)
                .await
                .filter(|r| text(r, "version_type") == "release")
            {
                update = Some(release);
            }
        }
        let update = update.as_ref();
        let is_mod = file.kind == ContentKind::Mod;
        let mut item = PlanItem {
            kind: file.kind,
            file_name: Some(file.file_name.clone()),
            title: file.file_name.clone(),
            icon_url: None,
            project_id: now.map(|v| text(v, "project_id")),
            current_version: now.map(|v| text(v, "version_number")),
            target: None,
            status: ItemStatus::Works,
            action: ItemAction::Keep,
            actions: vec![ItemAction::Keep, ItemAction::Disable],
            note: None,
        };
        let result = match now {
            None => {
                if is_mod {
                    if let Some((name, version, _, _)) =
                        crate::server::plugins::read_jar_metadata(&instance_dir.join("mods").join(&file.file_name))
                    {
                        item.title = name;
                        item.current_version = version;
                    }
                }
                item.status = ItemStatus::Unknown;
                item.note = Some("Not on Modrinth, so Ingot can't check it".into());
                // A mod that doesn't match the version usually stops the game from starting
                if is_mod {
                    item.action = ItemAction::Disable;
                }
                None
            }
            Some(now) if supports(now, to_game_version, &loaders) => Some(now.clone()),
            Some(now) => match update {
                Some(update) => {
                    item.status = ItemStatus::Update;
                    item.target = target_file(update);
                    item.note = unstable(update);
                    item.action = ItemAction::Update;
                    item.actions = vec![ItemAction::Update, ItemAction::Disable, ItemAction::Keep];
                    Some(update.clone())
                }
                None if is_mod => {
                    item.status = ItemStatus::Missing;
                    item.action = ItemAction::Disable;
                    item.actions = vec![ItemAction::Disable, ItemAction::Keep];
                    item.note = Some(format!("No version for Minecraft {to_game_version} yet"));
                    None
                }
                None => {
                    // Packs and shaders usually still work; the game only warns
                    item.status = ItemStatus::Missing;
                    item.note = Some(format!("Not updated for {to_game_version}; it usually still works"));
                    Some(now.clone())
                }
            },
        };
        items.push(item);
        resulting.push(result);
    }

    // The same mod twice: keep the first, turn the others off
    let mut seen_projects: HashMap<String, String> = HashMap::new();
    for item in items.iter_mut().filter(|i| i.kind == ContentKind::Mod) {
        let Some(project) = item.project_id.clone() else { continue };
        match seen_projects.get(&project) {
            Some(first) => {
                item.status = ItemStatus::Conflict;
                item.action = ItemAction::Disable;
                item.actions = vec![ItemAction::Disable, ItemAction::Keep];
                item.note = Some(format!("Same mod as {first}"));
            }
            None => {
                seen_projects.insert(project, item.file_name.clone().unwrap_or_default());
            }
        }
    }

    let mentioned = resolve_dependencies(client, to_game_version, loader, &mut items, &mut resulting).await;

    // Titles and icons (notes name projects by id until now)
    let ids: Vec<String> = items
        .iter()
        .filter_map(|i| i.project_id.clone())
        .chain(mentioned)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    if !ids.is_empty() {
        if let Ok(projects) = get(client, "/projects", &[("ids", serde_json::to_string(&ids).unwrap_or_default())]).await {
            let by_id: HashMap<String, &Value> =
                projects.as_array().into_iter().flatten().map(|p| (text(p, "id"), p)).collect();
            for item in &mut items {
                if let Some(p) = item.project_id.as_ref().and_then(|id| by_id.get(id)) {
                    item.title = text(p, "title");
                    item.icon_url = p.get("icon_url").and_then(Value::as_str).filter(|u| !u.is_empty()).map(str::to_string);
                }
            }
            let titles: HashMap<String, String> = by_id.iter().map(|(id, p)| (id.clone(), text(p, "title"))).collect();
            for item in &mut items {
                if let Some(note) = &mut item.note {
                    for (id, title) in &titles {
                        if note.contains(id.as_str()) {
                            *note = note.replace(id.as_str(), title);
                        }
                    }
                }
            }
        }
    }

    Ok(VersionPlan {
        instance_id: instance.id.clone(),
        loader: loader.clone(),
        from_game_version: instance.game_version.clone(),
        from_loader_version: instance.loader_version.clone(),
        to_game_version: to_game_version.to_string(),
        to_loader_version,
        downgrade,
        items,
        worlds: list_worlds(instance_dir),
    })
}

/// Adds missing required dependencies and flags incompatible mods, until nothing changes.
/// Notes name projects by id; returns the ids mentioned that aren't items.
async fn resolve_dependencies(
    client: &reqwest::Client,
    to_game_version: &str,
    loader: &ModLoaderType,
    items: &mut Vec<PlanItem>,
    resulting: &mut Vec<Option<Value>>,
) -> HashSet<String> {
    let loaders = mod_loaders(loader);
    let mut unavailable: HashSet<String> = HashSet::new();
    let mut project_of_version: HashMap<String, Option<String>> = HashMap::new();
    for _ in 0..10 {
        let active = |items: &[PlanItem]| -> HashSet<String> {
            items
                .iter()
                .filter(|i| matches!(i.action, ItemAction::Keep | ItemAction::Update | ItemAction::Add))
                .filter_map(|i| i.project_id.clone())
                .collect()
        };
        let mut changed = false;
        for index in 0..items.len() {
            if items[index].kind != ContentKind::Mod
                || !matches!(items[index].action, ItemAction::Keep | ItemAction::Update | ItemAction::Add)
            {
                continue;
            }
            let Some(version) = resulting[index].clone() else { continue };
            let deps = version.get("dependencies").and_then(Value::as_array).cloned().unwrap_or_default();
            for dep in deps {
                let kind = text(&dep, "dependency_type");
                if kind != "required" && kind != "incompatible" {
                    continue;
                }
                let project = match dep.get("project_id").and_then(Value::as_str) {
                    Some(p) => Some(p.to_string()),
                    None => {
                        let version_id = text(&dep, "version_id");
                        if version_id.is_empty() {
                            None
                        } else if let Some(p) = project_of_version.get(&version_id) {
                            p.clone()
                        } else {
                            let p = get(client, &format!("/version/{version_id}"), &[]).await.ok().map(|v| text(&v, "project_id"));
                            project_of_version.insert(version_id, p.clone());
                            p
                        }
                    }
                };
                let Some(project) = project.filter(|p| !p.is_empty()) else { continue };
                let active_now = active(items);

                if kind == "incompatible" {
                    if active_now.contains(&project) && items[index].status != ItemStatus::Conflict {
                        let item = &mut items[index];
                        item.status = ItemStatus::Conflict;
                        item.action = ItemAction::Disable;
                        item.actions = vec![ItemAction::Disable, ItemAction::Keep];
                        item.note = Some(format!("Doesn't work together with {project}"));
                        changed = true;
                    }
                    continue;
                }

                if active_now.contains(&project) {
                    continue;
                }
                let turned_off = items.iter().any(|i| i.project_id.as_deref() == Some(project.as_str()));
                let found = if turned_off || unavailable.contains(&project) {
                    None
                } else {
                    best_version(client, &project, to_game_version, &loaders).await
                };
                match found.as_ref().and_then(|v| target_file(v).map(|t| (v, t))) {
                    Some((version, target)) => {
                        let needed_by = items[index].project_id.clone().unwrap_or_else(|| items[index].title.clone());
                        items.push(PlanItem {
                            kind: ContentKind::Mod,
                            file_name: None,
                            title: project.clone(),
                            icon_url: None,
                            project_id: Some(project.clone()),
                            current_version: None,
                            target: Some(target),
                            status: ItemStatus::NewDependency,
                            action: ItemAction::Add,
                            actions: vec![ItemAction::Add, ItemAction::Skip],
                            note: Some(match unstable(version) {
                                Some(channel) => format!("Needed by {needed_by} · {channel}"),
                                None => format!("Needed by {needed_by}"),
                            }),
                        });
                        resulting.push(Some(version.clone()));
                    }
                    None => {
                        unavailable.insert(project.clone());
                        let item = &mut items[index];
                        item.status = ItemStatus::Conflict;
                        item.action = ItemAction::Disable;
                        item.actions = vec![ItemAction::Disable, ItemAction::Keep];
                        item.note = Some(if turned_off {
                            format!("Needs {project}, which is being turned off")
                        } else {
                            format!("Needs {project}, which isn't available for {to_game_version}")
                        });
                    }
                }
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    unavailable
}

// ─── Apply ────────────────────────────────────────────────────────────────────

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Downloads everything to install into the staging folder, checking each file's hash
async fn stage(
    client: &reqwest::Client,
    staging: &Path,
    items: &[&PlanItem],
    progress: &(impl Fn(String) + Sync),
) -> Result<(), String> {
    let _ = fs::remove_dir_all(staging);
    fs::create_dir_all(staging).map_err(|e| format!("Failed to prepare downloads: {e}"))?;
    let mut jobs = tokio::task::JoinSet::new();
    let total = items.len();
    let mut done = 0;
    let mut queue = items.iter();
    loop {
        while jobs.len() < 6 {
            let Some(item) = queue.next() else { break };
            let target = item.target.clone().ok_or_else(|| format!("{} has nothing to download", item.title))?;
            safe_file_name(&target.file_name, item.kind)?;
            let host = reqwest::Url::parse(&target.url).ok().and_then(|u| u.host_str().map(str::to_string));
            if host.as_deref() != Some("cdn.modrinth.com") || target.sha1.len() != 40 {
                return Err(format!("{} doesn't come from Modrinth's download server", item.title));
            }
            let dest = staging.join(item.kind.folder()).join(&target.file_name);
            let (client, title) = (client.clone(), item.title.clone());
            jobs.spawn(async move {
                if let Some(parent) = dest.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let result = crate::minecraft::downloader::download_file_chunked(&client, &target.url, &dest, Some(&target.sha1), None).await;
                match result {
                    Ok(_) if crate::minecraft::downloader::verify_file_sha1(&dest, &target.sha1) => Ok(()),
                    Ok(_) => Err(format!("{title} downloaded incorrectly (hash mismatch)")),
                    Err(e) => Err(format!("Failed to download {title}: {e}")),
                }
            });
        }
        let Some(result) = jobs.join_next().await else { break };
        result.map_err(|e| e.to_string())??;
        done += 1;
        progress(format!("Downloading {done} of {total}..."));
    }
    Ok(())
}

/// Carries out the reviewed plan, all or nothing. Returns the updated instance.
pub async fn apply(
    client: &reqwest::Client,
    instance_dir: &Path,
    instance: &InstanceConfig,
    plan: &VersionPlan,
    backup_worlds: bool,
    progress: impl Fn(String) + Sync,
) -> Result<InstanceConfig, String> {
    if plan.instance_id != instance.id || plan.from_game_version != instance.game_version {
        return Err("The instance changed since it was checked. Check again.".into());
    }
    for item in &plan.items {
        if !item.actions.contains(&item.action) {
            return Err(format!("{} can't be set to {:?}", item.title, item.action));
        }
        if let Some(name) = &item.file_name {
            safe_file_name(name, item.kind)?;
        }
    }

    // 1. Download and verify first: a failure here changes nothing
    let staging = work_dir(instance_dir).join("staging");
    let to_download: Vec<&PlanItem> =
        plan.items.iter().filter(|i| matches!(i.action, ItemAction::Update | ItemAction::Add)).collect();
    if !to_download.is_empty() {
        progress("Downloading...".into());
        if let Err(e) = stage(client, &staging, &to_download, &progress).await {
            let _ = fs::remove_dir_all(&staging);
            return Err(e);
        }
    }

    // 2. Snapshot, then swap; any failure puts back what was done so far
    progress("Backing up...".into());
    let backup = backup_dir(instance_dir);
    let _ = fs::remove_dir_all(&backup);
    fs::create_dir_all(&backup).map_err(|e| format!("Failed to create the backup: {e}"))?;
    let mut journal = Journal {
        created_at: now_secs(),
        from_game_version: instance.game_version.clone(),
        from_loader_version: instance.loader_version.clone(),
        to_game_version: plan.to_game_version.clone(),
        ..Default::default()
    };
    let result = swap(instance_dir, &backup, &staging, plan, backup_worlds, &mut journal, &progress);
    let _ = fs::remove_dir_all(&staging);
    if let Err(e) = result {
        let _ = restore(instance_dir, &backup, &journal);
        let _ = fs::remove_dir_all(&backup);
        return Err(format!("{e}. Nothing was changed."));
    }
    let raw = serde_json::to_string_pretty(&journal).map_err(|e| e.to_string())?;
    fs::write(backup.join(JOURNAL), raw).map_err(|e| format!("Failed to save the backup: {e}"))?;

    let mut updated = instance.clone();
    updated.game_version = plan.to_game_version.clone();
    updated.loader_version = plan.to_loader_version.clone();
    Ok(updated)
}

fn swap(
    instance_dir: &Path,
    backup: &Path,
    staging: &Path,
    plan: &VersionPlan,
    backup_worlds: bool,
    journal: &mut Journal,
    progress: &impl Fn(String),
) -> Result<(), String> {
    // Minecraft rewrites options.txt and mods migrate their configs on first start
    let config = instance_dir.join("config");
    if config.is_dir() {
        copy_dir(&config, &backup.join("config")).map_err(|e| format!("Failed to back up configs: {e}"))?;
        journal.config_backed_up = true;
    }
    let options = instance_dir.join("options.txt");
    if options.is_file() {
        fs::copy(&options, backup.join("options.txt")).map_err(|e| format!("Failed to back up options: {e}"))?;
        journal.options_backed_up = true;
    }
    if backup_worlds {
        let worlds = list_worlds(instance_dir);
        for (i, world) in worlds.iter().enumerate() {
            progress(format!("Backing up worlds ({} of {})...", i + 1, worlds.len()));
            copy_dir(&instance_dir.join("saves").join(&world.folder_name), &backup.join("saves").join(&world.folder_name))
                .map_err(|e| format!("Failed to back up the world {}: {e}", world.folder_name))?;
            journal.worlds.push(world.folder_name.clone());
        }
    }

    progress("Applying...".into());
    let move_to_backup = |rel: String, journal: &mut Journal| -> Result<(), String> {
        let stored = format!("files/{rel}");
        move_path(&instance_dir.join(&rel), &backup.join(&stored))?;
        journal.moved.push((rel, stored));
        Ok(())
    };
    for item in &plan.items {
        let folder = item.kind.folder();
        match item.action {
            ItemAction::Keep | ItemAction::Skip => {}
            ItemAction::Disable => {
                let Some(name) = &item.file_name else { continue };
                let rel = format!("{folder}/{name}");
                let disabled = instance_dir.join(format!("{rel}.disabled"));
                if disabled.exists() {
                    move_to_backup(format!("{rel}.disabled"), journal)?;
                }
                fs::rename(instance_dir.join(&rel), &disabled).map_err(|e| format!("Failed to turn off {name}: {e}"))?;
                journal.disabled.push(rel);
            }
            ItemAction::Update | ItemAction::Add => {
                let target = item.target.as_ref().ok_or_else(|| format!("{} has nothing to install", item.title))?;
                if let Some(old) = &item.file_name {
                    move_to_backup(format!("{folder}/{old}"), journal)?;
                }
                let rel = format!("{folder}/{}", target.file_name);
                if instance_dir.join(&rel).exists() {
                    // Already there (e.g. a dependency the user installed); keep theirs aside
                    move_to_backup(rel.clone(), journal)?;
                }
                move_path(&staging.join(&rel), &instance_dir.join(&rel))?;
                journal.added.push(rel);
            }
        }
    }
    Ok(())
}

/// Puts back everything the journal lists, newest change first
fn restore(instance_dir: &Path, backup: &Path, journal: &Journal) -> Result<(), String> {
    let mut errors = Vec::new();
    for rel in journal.added.iter().rev() {
        if let Err(e) = fs::remove_file(instance_dir.join(rel)) {
            if e.kind() != std::io::ErrorKind::NotFound {
                errors.push(format!("{rel}: {e}"));
            }
        }
    }
    for rel in journal.disabled.iter().rev() {
        if let Err(e) = fs::rename(instance_dir.join(format!("{rel}.disabled")), instance_dir.join(rel)) {
            errors.push(format!("{rel}: {e}"));
        }
    }
    for (rel, stored) in journal.moved.iter().rev() {
        if let Err(e) = move_path(&backup.join(stored), &instance_dir.join(rel)) {
            errors.push(e);
        }
    }
    if journal.config_backed_up {
        let config = instance_dir.join("config");
        let _ = fs::remove_dir_all(&config);
        if let Err(e) = copy_dir(&backup.join("config"), &config) {
            errors.push(format!("configs: {e}"));
        }
    }
    if journal.options_backed_up {
        if let Err(e) = fs::copy(backup.join("options.txt"), instance_dir.join("options.txt")) {
            errors.push(format!("options.txt: {e}"));
        }
    }
    for world in &journal.worlds {
        let dest = instance_dir.join("saves").join(world);
        let _ = fs::remove_dir_all(&dest);
        if let Err(e) = copy_dir(&backup.join("saves").join(world), &dest) {
            errors.push(format!("world {world}: {e}"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Some files couldn't be restored: {}", errors.join("; ")))
    }
}

// ─── Undo ─────────────────────────────────────────────────────────────────────

fn read_journal(instance_dir: &Path) -> Option<Journal> {
    let raw = fs::read_to_string(backup_dir(instance_dir).join(JOURNAL)).ok()?;
    serde_json::from_str(&raw).ok()
}

/// The last version change, if it can be undone
pub fn last_backup(instance_dir: &Path) -> Option<VersionBackup> {
    let journal = read_journal(instance_dir)?;
    Some(VersionBackup {
        from_game_version: journal.from_game_version,
        to_game_version: journal.to_game_version,
        created_at: journal.created_at,
        worlds_backed_up: !journal.worlds.is_empty(),
        bytes: dir_size(&backup_dir(instance_dir)) as f64,
    })
}

/// Restores the instance exactly as it was before the last change. Returns the instance
/// with its previous version (other settings stay as they are now).
pub fn undo(instance_dir: &Path, instance: &InstanceConfig) -> Result<InstanceConfig, String> {
    let journal = read_journal(instance_dir).ok_or("There's no version change to undo")?;
    if instance.game_version != journal.to_game_version {
        return Err(format!(
            "The instance is on {} now, not {}, so this backup no longer matches",
            instance.game_version, journal.to_game_version
        ));
    }
    let backup = backup_dir(instance_dir);
    restore(instance_dir, &backup, &journal)?;
    let _ = fs::remove_dir_all(&backup);
    let mut restored = instance.clone();
    restored.game_version = journal.from_game_version;
    restored.loader_version = journal.from_loader_version;
    Ok(restored)
}

/// Deletes the backup of the last change (frees the space; it can't be undone after)
pub fn discard_backup(instance_dir: &Path) -> Result<(), String> {
    let backup = backup_dir(instance_dir);
    if backup.exists() {
        fs::remove_dir_all(&backup).map_err(|e| format!("Failed to delete the backup: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instance(game_version: &str) -> InstanceConfig {
        serde_json::from_value(json!({
            "id": "test", "name": "Test", "gameVersion": game_version, "loader": "fabric",
            "loaderVersion": "0.17.0", "javaPath": null, "memoryMinMb": null, "memoryMaxMb": null,
            "icon": null, "createdAt": 0, "lastPlayed": null, "totalPlayTimeSeconds": 0, "jvmArgs": null
        }))
        .unwrap()
    }

    fn item(kind: ContentKind, file: Option<&str>, action: ItemAction, target: Option<&str>) -> PlanItem {
        PlanItem {
            kind,
            file_name: file.map(str::to_string),
            title: file.or(target).unwrap_or_default().to_string(),
            icon_url: None,
            project_id: None,
            current_version: None,
            target: target.map(|name| TargetFile {
                version_id: "v".into(),
                version_number: "2".into(),
                file_name: name.into(),
                url: String::new(),
                sha1: String::new(),
                bytes: 0.0,
            }),
            status: ItemStatus::Works,
            action,
            actions: vec![action],
            note: None,
        }
    }

    /// Swap + undo on real files, without downloads (staging prepared by hand)
    #[test]
    fn swaps_and_undoes_exactly() {
        let dir = std::env::temp_dir().join(format!("ingot-version-change-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        for (path, text) in [
            ("mods/keep.jar", "keep"),
            ("mods/old-1.jar", "old"),
            ("mods/gone.jar", "gone"),
            ("resourcepacks/pack.zip", "pack"),
            ("config/mod.toml", "a = 1"),
            ("options.txt", "version:1"),
            ("saves/World/level.dat", "world"),
        ] {
            fs::create_dir_all(dir.join(path).parent().unwrap()).unwrap();
            fs::write(dir.join(path), text).unwrap();
        }
        let staging = work_dir(&dir).join("staging");
        for (path, text) in [("mods/old-2.jar", "new"), ("mods/dep.jar", "dep")] {
            fs::create_dir_all(staging.join(path).parent().unwrap()).unwrap();
            fs::write(staging.join(path), text).unwrap();
        }
        let plan = VersionPlan {
            instance_id: "test".into(),
            loader: ModLoaderType::Fabric,
            from_game_version: "1.21.4".into(),
            from_loader_version: Some("0.17.0".into()),
            to_game_version: "26.1".into(),
            to_loader_version: Some("0.18.0".into()),
            downgrade: false,
            items: vec![
                item(ContentKind::Mod, Some("keep.jar"), ItemAction::Keep, None),
                item(ContentKind::Mod, Some("old-1.jar"), ItemAction::Update, Some("old-2.jar")),
                item(ContentKind::Mod, Some("gone.jar"), ItemAction::Disable, None),
                item(ContentKind::Mod, None, ItemAction::Add, Some("dep.jar")),
                item(ContentKind::ResourcePack, Some("pack.zip"), ItemAction::Keep, None),
            ],
            worlds: list_worlds(&dir),
        };
        let backup = backup_dir(&dir);
        fs::create_dir_all(&backup).unwrap();
        let mut journal = Journal {
            from_game_version: "1.21.4".into(),
            from_loader_version: Some("0.17.0".into()),
            to_game_version: "26.1".into(),
            ..Default::default()
        };
        swap(&dir, &backup, &staging, &plan, true, &mut journal, &|_| {}).unwrap();
        fs::write(backup.join(JOURNAL), serde_json::to_string(&journal).unwrap()).unwrap();

        let exists = |p: &str| dir.join(p).exists();
        assert!(exists("mods/keep.jar") && exists("mods/old-2.jar") && exists("mods/dep.jar"));
        assert!(!exists("mods/old-1.jar") && !exists("mods/gone.jar") && exists("mods/gone.jar.disabled"));
        assert_eq!(last_backup(&dir).map(|b| (b.to_game_version, b.worlds_backed_up)), Some(("26.1".into(), true)));

        // The game upgrades the world and configs; undo brings the old ones back
        fs::write(dir.join("saves/World/level.dat"), "upgraded").unwrap();
        fs::write(dir.join("config/mod.toml"), "a = 2").unwrap();
        fs::write(dir.join("config/new.toml"), "").unwrap();
        assert!(undo(&dir, &instance("1.21.4")).is_err(), "only undoes from the version it changed to");
        let restored = undo(&dir, &instance("26.1")).unwrap();
        assert_eq!((restored.game_version.as_str(), restored.loader_version.as_deref()), ("1.21.4", Some("0.17.0")));

        let read = |p: &str| fs::read_to_string(dir.join(p)).unwrap();
        assert_eq!(read("mods/old-1.jar"), "old");
        assert_eq!(read("mods/gone.jar"), "gone");
        assert_eq!(read("saves/World/level.dat"), "world");
        assert_eq!(read("config/mod.toml"), "a = 1");
        assert!(!exists("config/new.toml") && !exists("mods/old-2.jar") && !exists("mods/dep.jar") && !exists("mods/gone.jar.disabled"));
        assert!(last_backup(&dir).is_none());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_unsafe_file_names() {
        assert!(safe_file_name("sodium-0.6.jar", ContentKind::Mod).is_ok());
        assert!(safe_file_name("../evil.jar", ContentKind::Mod).is_err());
        assert!(safe_file_name("C:evil.jar", ContentKind::Mod).is_err());
        assert!(safe_file_name("pack.jar", ContentKind::ResourcePack).is_err());
    }
}
