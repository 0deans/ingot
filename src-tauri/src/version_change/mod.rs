//! Moving an instance or a server to another Minecraft version (or loader) without
//! breaking it.
//!
//! 1. `check` changes nothing: it identifies every mod, plugin, resource pack and shader
//!    (Modrinth by hash, then CurseForge by fingerprint, and Hangar for plugins Ingot
//!    installed) and works out, for the target, what still works, what has an update,
//!    what's missing, which new dependencies are needed and what conflicts.
//! 2. `apply` carries out the reviewed plan all or nothing: new files are downloaded and
//!    verified in a staging folder first, the current files, configs (and worlds, if
//!    asked) are snapshotted, and only then is everything swapped in. A failure part way
//!    puts everything back.
//! 3. `undo` restores the snapshot of the last change exactly.
//! 4. `crash` watches the first start after a change and names the likely culprits.

pub mod crash;
pub mod sources;
pub mod targets;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

pub use sources::Source;
use sources::{ProjectInfo, Release, Scope};

/// Everything version change keeps lives here, inside the instance or server folder
const WORK_DIR: &str = ".ingot/version-change";
const JOURNAL: &str = "journal.json";
/// Download hosts files may come from
const ALLOWED_HOSTS: [&str; 4] = ["cdn.modrinth.com", "edge.forgecdn.net", "mediafilez.forgecdn.net", "hangarcdn.papermc.io"];
/// Libraries only Fabric and Quilt mods use: pointless after moving to another loader
const FABRIC_ONLY: [&str; 4] = ["P7dR8mSH", "qvIfYCYJ", "306612", "634179"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ContentKind {
    Mod,
    Plugin,
    ResourcePack,
    Shader,
}

impl ContentKind {
    fn extension(self) -> &'static str {
        match self {
            Self::Mod | Self::Plugin => ".jar",
            Self::ResourcePack | Self::Shader => ".zip",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ItemStatus {
    /// The current file already supports the target
    Works,
    /// A version for the target exists
    Update,
    /// Nothing for the target yet (or it can only be downloaded from its website)
    Missing,
    /// Not found on Modrinth, CurseForge or Hangar (added by hand), so it can't be checked
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
    pub sha1: Option<String>,
    pub sha256: Option<String>,
    #[specta(type = i32)]
    pub bytes: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanItem {
    pub kind: ContentKind,
    /// Folder it lives in ("mods", "plugins", "resourcepacks", "shaderpacks")
    pub folder: String,
    /// Current file; None for a new dependency
    pub file_name: Option<String>,
    pub title: String,
    pub icon_url: Option<String>,
    pub page_url: Option<String>,
    pub source: Option<Source>,
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
    /// Folder, relative to the instance or server
    pub folder_name: String,
    #[specta(type = i32)]
    pub bytes: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TargetKind {
    Instance,
    Server,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VersionPlan {
    pub target_kind: TargetKind,
    pub target_id: String,
    pub from_game_version: String,
    /// Mod loader or server core ("fabric", "paper"...)
    pub from_loader: String,
    /// Loader version (instances) or build (servers)
    pub from_loader_version: Option<String>,
    pub to_game_version: String,
    pub to_loader: String,
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
    pub from_loader: String,
    pub to_game_version: String,
    pub to_loader: String,
    #[specta(type = i32)]
    pub created_at: u64,
    pub worlds_backed_up: bool,
    #[specta(type = i32)]
    pub bytes: f64,
}

/// A folder of content to check
pub struct Folder {
    pub kind: ContentKind,
    pub dir: &'static str,
    pub scope: Scope,
}

/// What a change covers: the instance or server folder and what's in it
pub struct Target {
    pub root: PathBuf,
    pub folders: Vec<Folder>,
    /// World folders, relative to root
    pub worlds: Vec<String>,
    /// Files and folders snapshotted before the change and restored by undo
    pub configs: Vec<String>,
    /// Files and folders moved into the backup (a server's jar and installed loader):
    /// they're downloaded again for the new version and put back by undo
    pub core_files: Vec<String>,
    /// Plugins Ingot installed from Hangar: file name -> (project, version, title, icon)
    pub hangar: HashMap<String, (String, String, String, Option<String>)>,
}

/// What `check` compares against
pub struct PlanRequest {
    pub target_kind: TargetKind,
    pub target_id: String,
    pub from_game_version: String,
    pub from_loader: String,
    pub from_loader_version: Option<String>,
    pub to_game_version: String,
    pub to_loader: String,
    pub to_loader_version: Option<String>,
    pub downgrade: bool,
}

/// The version and loader to go back to
pub struct Restored {
    pub game_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
}

/// Everything `apply` changed, in order, so it can be put back
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Journal {
    created_at: u64,
    from_game_version: String,
    from_loader: String,
    from_loader_version: Option<String>,
    to_game_version: String,
    to_loader: String,
    /// Files moved into the backup: (path in the target, path in the backup)
    moved: Vec<(String, String)>,
    /// Files renamed to <name>.disabled
    disabled: Vec<String>,
    /// Files put in place
    added: Vec<String>,
    /// Configs copied into the backup
    configs: Vec<String>,
    worlds: Vec<String>,
    /// Set after the first successful start; until then a crash is reported
    first_start_checked: bool,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn work_dir(root: &Path) -> PathBuf {
    root.join(WORK_DIR)
}

fn backup_dir(root: &Path) -> PathBuf {
    work_dir(root).join("backup")
}

fn sha1_hex(data: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    Sha1::digest(data).iter().map(|b| format!("{b:02x}")).collect()
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect()
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

/// Copies a file or a folder
fn copy_path(from: &Path, to: &Path) -> std::io::Result<()> {
    if from.is_file() {
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(from, to)?;
        return Ok(());
    }
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)?.flatten() {
        let kind = entry.file_type()?;
        let dest = to.join(entry.file_name());
        if kind.is_dir() {
            copy_path(&entry.path(), &dest)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

fn remove_path(path: &Path) -> std::io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
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

/// A path inside the target given by the frontend or the journal: relative, no ".."
fn safe_rel(rel: &str) -> Result<&str, String> {
    let path = Path::new(rel);
    let ok = !rel.is_empty()
        && path.components().all(|c| matches!(c, std::path::Component::Normal(_)));
    ok.then_some(rel).ok_or_else(|| format!("Unexpected path: {rel}"))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Marks a project in a note; replaced by its title once titles are known
fn mention(project: &str) -> String {
    format!("\u{1}{project}\u{1}")
}

// ─── Check ────────────────────────────────────────────────────────────────────

struct LocalFile {
    folder: usize,
    file_name: String,
    sha1: String,
    fingerprint: Option<u32>,
}

fn scan(target: &Target) -> Vec<LocalFile> {
    let mut out = Vec::new();
    for (index, folder) in target.folders.iter().enumerate() {
        let Ok(entries) = fs::read_dir(target.root.join(folder.dir)) else { continue };
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if !entry.path().is_file() || safe_file_name(&file_name, folder.kind).is_err() {
                continue;
            }
            let Ok(data) = fs::read(entry.path()) else { continue };
            out.push(LocalFile {
                folder: index,
                file_name,
                sha1: sha1_hex(&data),
                fingerprint: folder.scope.curseforge.then(|| sources::cf_fingerprint(&data)),
            });
        }
    }
    out.sort_by(|a, b| a.file_name.to_lowercase().cmp(&b.file_name.to_lowercase()));
    out
}

pub fn list_worlds(target: &Target) -> Vec<WorldInfo> {
    target
        .worlds
        .iter()
        .map(|w| WorldInfo { folder_name: w.clone(), bytes: dir_size(&target.root.join(w)) as f64 })
        .collect()
}

/// The version of a project for the target on its source
async fn find_target(client: &reqwest::Client, source: Source, project: &str, scope: &Scope, game_version: &str) -> Option<Release> {
    match source {
        Source::Modrinth => sources::modrinth_target(client, project, game_version, &scope.modrinth).await,
        Source::Curseforge => sources::cf_target(client, project, game_version, scope).await,
        Source::Hangar => {
            let versions = sources::hangar_versions(client, project, game_version).await;
            let pick = versions.iter().find(|v| v.channel == sources::Channel::Release).or(versions.first());
            pick.cloned()
        }
    }
}

fn site(source: Source) -> &'static str {
    match source {
        Source::Modrinth => "Modrinth",
        Source::Curseforge => "CurseForge",
        Source::Hangar => "Hangar",
    }
}

/// Works out what moving to the target means for every file. Changes nothing.
pub async fn check(client: &reqwest::Client, target: &Target, request: PlanRequest) -> Result<VersionPlan, String> {
    let game = request.to_game_version.as_str();
    let loader_changes = request.from_loader != request.to_loader;
    let local = scan(target);

    // Identify: Modrinth by hash (per folder: the loaders differ), then CurseForge
    let mut on_modrinth: HashMap<String, (Release, bool)> = HashMap::new();
    for (index, folder) in target.folders.iter().enumerate() {
        let hashes: Vec<String> = local.iter().filter(|f| f.folder == index).map(|f| f.sha1.clone()).collect();
        on_modrinth.extend(sources::modrinth_identify(client, &hashes, game, &folder.scope.modrinth).await?);
    }
    let mut on_curseforge: HashMap<u32, (Release, bool)> = HashMap::new();
    for (index, folder) in target.folders.iter().enumerate() {
        let prints: Vec<u32> = local
            .iter()
            .filter(|f| f.folder == index && !target.hangar.contains_key(&f.file_name))
            .filter_map(|f| f.fingerprint)
            .collect();
        // CurseForge being down only means fewer files can be checked
        if let Ok(found) = sources::cf_identify(client, &prints, game, &folder.scope).await {
            on_curseforge.extend(found);
        }
    }

    // One entry per file, plus the release it would end up on
    let mut items: Vec<PlanItem> = Vec::new();
    let mut resulting: Vec<Option<Release>> = Vec::new();
    let mut hangar_info: HashMap<String, ProjectInfo> = HashMap::new();
    for file in &local {
        let folder = &target.folders[file.folder];
        let kind = folder.kind;
        let mut item = PlanItem {
            kind,
            folder: folder.dir.to_string(),
            file_name: Some(file.file_name.clone()),
            title: file.file_name.clone(),
            icon_url: None,
            page_url: None,
            source: None,
            project_id: None,
            current_version: None,
            target: None,
            status: ItemStatus::Works,
            action: ItemAction::Keep,
            actions: vec![ItemAction::Keep, ItemAction::Disable],
            note: None,
        };

        // What the file is now, and whether it already fits
        let identified: Option<(Release, bool)> = if let Some(found) = on_modrinth.get(&file.sha1) {
            Some(found.clone())
        } else if let Some(found) = file.fingerprint.and_then(|p| on_curseforge.get(&p)) {
            Some(found.clone())
        } else if let Some((project, version, title, icon)) = target.hangar.get(&file.file_name) {
            hangar_info.insert(
                project.clone(),
                ProjectInfo {
                    title: title.clone(),
                    icon_url: icon.clone(),
                    page_url: Some(format!("https://hangar.papermc.io/search?query={project}")),
                },
            );
            let versions = sources::hangar_versions(client, project, game).await;
            let now = Release {
                source: Source::Hangar,
                project: project.clone(),
                version_id: version.clone(),
                version_number: version.clone(),
                channel: sources::Channel::Release,
                file: None,
                deps: Vec::new(),
            };
            Some(match versions.iter().find(|v| &v.version_id == version) {
                Some(listed) => (listed.clone(), true),
                None => (now, false),
            })
        } else {
            None
        };

        let result = match identified {
            None => {
                if kind == ContentKind::Mod || kind == ContentKind::Plugin {
                    let path = target.root.join(folder.dir).join(&file.file_name);
                    if let Some((name, version, _, _)) = crate::server::plugins::read_jar_metadata(&path) {
                        item.title = name;
                        item.current_version = version;
                    }
                }
                item.status = ItemStatus::Unknown;
                item.note = Some("Not on Modrinth, CurseForge or Hangar, so Ingot can't check it".into());
                // A mod that doesn't match usually stops the game from starting; plugins
                // and packs mostly keep working
                if kind == ContentKind::Mod {
                    item.action = ItemAction::Disable;
                }
                None
            }
            Some((now, fits)) => {
                item.source = Some(now.source);
                item.project_id = Some(now.project.clone());
                item.current_version = Some(now.version_number.clone());
                if fits {
                    Some(now)
                } else {
                    let mut found = find_target(client, now.source, &now.project, &folder.scope, game).await;
                    // A newer file of the same version already installed
                    if found.as_ref().is_some_and(|f| f.version_id == now.version_id) {
                        found = None;
                    }
                    // Not on Modrinth for the target: CurseForge may have it (same file there)
                    if found.as_ref().is_none_or(|f| f.file.is_none()) && now.source == Source::Modrinth {
                        if let Some((cf_now, _)) = file.fingerprint.and_then(|p| on_curseforge.get(&p)) {
                            let cf_found = find_target(client, Source::Curseforge, &cf_now.project, &folder.scope, game).await;
                            if cf_found.as_ref().is_some_and(|f| f.file.is_some() && f.version_id != cf_now.version_id) {
                                found = cf_found;
                            }
                        }
                    }
                    match found {
                        Some(release) if release.file.is_some() => {
                            item.source = Some(release.source);
                            item.project_id = Some(release.project.clone());
                            item.status = ItemStatus::Update;
                            item.target = release.file.clone();
                            item.note = release.unstable_note();
                            item.action = ItemAction::Update;
                            item.actions = vec![ItemAction::Update, ItemAction::Disable, ItemAction::Keep];
                            Some(release)
                        }
                        found => {
                            item.status = ItemStatus::Missing;
                            let target_name = if loader_changes {
                                format!("{} {game}", loader_label(&request.to_loader))
                            } else {
                                format!("Minecraft {game}")
                            };
                            item.note = Some(if found.is_some() {
                                format!("Its {target_name} version can only be downloaded from {}", site(now.source))
                            } else if loader_changes && FABRIC_ONLY.contains(&now.project.as_str()) {
                                "Only needed on Fabric and Quilt".into()
                            } else {
                                match kind {
                                    ContentKind::Mod => format!("No version for {target_name} yet"),
                                    ContentKind::Plugin => format!("Not marked for {game} yet; most plugins still work"),
                                    _ => format!("Not updated for {game}; it usually still works"),
                                }
                            });
                            if kind == ContentKind::Mod {
                                item.action = ItemAction::Disable;
                                item.actions = vec![ItemAction::Disable, ItemAction::Keep];
                                None
                            } else {
                                Some(now)
                            }
                        }
                    }
                }
            }
        };
        items.push(item);
        resulting.push(result);
    }

    // The same project twice in a folder: keep the first, turn the others off
    let mut seen: HashMap<(String, Source, String), String> = HashMap::new();
    for item in &mut items {
        let (Some(source), Some(project)) = (item.source, item.project_id.clone()) else { continue };
        match seen.get(&(item.folder.clone(), source, project.clone())) {
            Some(first) => {
                item.status = ItemStatus::Conflict;
                item.action = ItemAction::Disable;
                item.actions = vec![ItemAction::Disable, ItemAction::Keep];
                item.note = Some(format!("Same as {first}"));
            }
            None => {
                seen.insert((item.folder.clone(), source, project), item.file_name.clone().unwrap_or_default());
            }
        }
    }

    let folders: HashMap<String, &Folder> = target.folders.iter().map(|f| (f.dir.to_string(), f)).collect();
    let mentioned = resolve_dependencies(client, game, &folders, &mut items, &mut resulting).await;

    // Titles, icons and pages (notes mention projects until now)
    let mut ids: HashMap<Source, HashSet<String>> = HashMap::new();
    for item in &items {
        if let (Some(source), Some(project)) = (item.source, &item.project_id) {
            ids.entry(source).or_default().insert(project.clone());
        }
    }
    for (source, project) in mentioned {
        ids.entry(source).or_default().insert(project);
    }
    let list = |source: Source| ids.get(&source).map(|s| s.iter().cloned().collect::<Vec<_>>()).unwrap_or_default();
    let mut info: HashMap<(Source, String), ProjectInfo> = HashMap::new();
    info.extend(sources::modrinth_projects(client, &list(Source::Modrinth)).await.into_iter().map(|(k, v)| ((Source::Modrinth, k), v)));
    info.extend(sources::cf_projects(client, &list(Source::Curseforge)).await.into_iter().map(|(k, v)| ((Source::Curseforge, k), v)));
    info.extend(hangar_info.into_iter().map(|(k, v)| ((Source::Hangar, k), v)));
    let titles: HashMap<String, String> = info.iter().map(|((_, id), p)| (id.clone(), p.title.clone())).collect();
    for item in &mut items {
        if let Some(p) = item.source.zip(item.project_id.clone()).and_then(|key| info.get(&key)) {
            if !p.title.is_empty() {
                item.title = p.title.clone();
            }
            item.icon_url = p.icon_url.clone();
            item.page_url = p.page_url.clone();
        }
        if let Some(note) = &mut item.note {
            let parts: Vec<&str> = note.split('\u{1}').collect();
            *note = parts
                .iter()
                .enumerate()
                .map(|(i, part)| if i % 2 == 1 { titles.get(*part).cloned().unwrap_or_else(|| part.to_string()) } else { part.to_string() })
                .collect();
        }
    }

    Ok(VersionPlan {
        target_kind: request.target_kind,
        target_id: request.target_id,
        from_game_version: request.from_game_version,
        from_loader: request.from_loader,
        from_loader_version: request.from_loader_version,
        to_game_version: request.to_game_version,
        to_loader: request.to_loader,
        to_loader_version: request.to_loader_version,
        downgrade: request.downgrade,
        items,
        worlds: list_worlds(target),
    })
}

pub fn loader_label(loader: &str) -> String {
    match loader {
        "neoforge" => "NeoForge".into(),
        other => {
            let mut chars = other.chars();
            chars.next().map(|c| c.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
        }
    }
}

fn is_active(action: ItemAction) -> bool {
    matches!(action, ItemAction::Keep | ItemAction::Update | ItemAction::Add)
}

/// Adds missing required dependencies and flags incompatible mods, until nothing changes.
/// Returns the projects mentioned in notes that aren't items.
async fn resolve_dependencies(
    client: &reqwest::Client,
    game: &str,
    folders: &HashMap<String, &Folder>,
    items: &mut Vec<PlanItem>,
    resulting: &mut Vec<Option<Release>>,
) -> Vec<(Source, String)> {
    let mut unavailable: HashSet<(Source, String)> = HashSet::new();
    let mut project_of_version: HashMap<String, Option<String>> = HashMap::new();
    for _ in 0..10 {
        let mut changed = false;
        for index in 0..items.len() {
            if !matches!(items[index].kind, ContentKind::Mod | ContentKind::Plugin) || !is_active(items[index].action) {
                continue;
            }
            let Some(release) = resulting[index].clone() else { continue };
            for dep in &release.deps {
                let project = match (&dep.project, &dep.version_id) {
                    (Some(p), _) => Some(p.clone()),
                    (None, Some(version)) if release.source == Source::Modrinth => {
                        if !project_of_version.contains_key(version) {
                            let p = sources::modrinth_project_of(client, version).await;
                            project_of_version.insert(version.clone(), p);
                        }
                        project_of_version[version].clone()
                    }
                    _ => None,
                };
                let Some(project) = project.filter(|p| !p.is_empty()) else { continue };
                let key = (release.source, project.clone());
                let is_project = |i: &PlanItem| i.source == Some(release.source) && i.project_id.as_deref() == Some(project.as_str());
                let active = items.iter().any(|i| is_active(i.action) && is_project(i));

                if !dep.required {
                    if active && items[index].status != ItemStatus::Conflict {
                        let item = &mut items[index];
                        item.status = ItemStatus::Conflict;
                        item.action = ItemAction::Disable;
                        item.actions = vec![ItemAction::Disable, ItemAction::Keep];
                        item.note = Some(format!("Doesn't work together with {}", mention(&project)));
                        changed = true;
                    }
                    continue;
                }
                if active {
                    continue;
                }
                let turned_off = items.iter().any(is_project);
                let Some(folder) = folders.get(&items[index].folder) else { continue };
                let found = if turned_off || unavailable.contains(&key) {
                    None
                } else {
                    find_target(client, release.source, &project, &folder.scope, game).await
                };
                match found.filter(|r| r.file.is_some()) {
                    Some(dep_release) => {
                        let needed_by = items[index]
                            .project_id
                            .as_deref()
                            .map(mention)
                            .unwrap_or_else(|| items[index].title.clone());
                        let note = match dep_release.unstable_note() {
                            Some(channel) => format!("Needed by {needed_by} · {channel}"),
                            None => format!("Needed by {needed_by}"),
                        };
                        items.push(PlanItem {
                            kind: items[index].kind,
                            folder: items[index].folder.clone(),
                            file_name: None,
                            title: project.clone(),
                            icon_url: None,
                            page_url: None,
                            source: Some(release.source),
                            project_id: Some(project.clone()),
                            current_version: None,
                            target: dep_release.file.clone(),
                            status: ItemStatus::NewDependency,
                            action: ItemAction::Add,
                            actions: vec![ItemAction::Add, ItemAction::Skip],
                            note: Some(note),
                        });
                        resulting.push(Some(dep_release));
                    }
                    None => {
                        unavailable.insert(key.clone());
                        let item = &mut items[index];
                        item.status = ItemStatus::Conflict;
                        item.action = ItemAction::Disable;
                        item.actions = vec![ItemAction::Disable, ItemAction::Keep];
                        item.note = Some(if turned_off {
                            format!("Needs {}, which is being turned off", mention(&project))
                        } else {
                            format!("Needs {}, which isn't available for {game}", mention(&project))
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
    unavailable.into_iter().collect()
}

// ─── Apply ────────────────────────────────────────────────────────────────────

fn verify(path: &Path, file: &TargetFile) -> bool {
    let Ok(data) = fs::read(path) else { return false };
    match (&file.sha1, &file.sha256) {
        (Some(sha1), _) => sha1_hex(&data).eq_ignore_ascii_case(sha1),
        (None, Some(sha256)) => sha256_hex(&data).eq_ignore_ascii_case(sha256),
        (None, None) => false,
    }
}

/// Downloads everything to install into the staging folder, checking each file's hash
async fn stage(client: &reqwest::Client, staging: &Path, items: &[&PlanItem], progress: &(impl Fn(String) + Sync)) -> Result<(), String> {
    let _ = fs::remove_dir_all(staging);
    fs::create_dir_all(staging).map_err(|e| format!("Failed to prepare downloads: {e}"))?;
    let mut jobs = tokio::task::JoinSet::new();
    let total = items.len();
    let mut done = 0;
    let mut queue = items.iter();
    loop {
        while jobs.len() < 6 {
            let Some(item) = queue.next() else { break };
            let file = item.target.clone().ok_or_else(|| format!("{} has nothing to download", item.title))?;
            safe_file_name(&file.file_name, item.kind)?;
            let host = reqwest::Url::parse(&file.url).ok().and_then(|u| u.host_str().map(str::to_string));
            if !host.as_deref().is_some_and(|h| ALLOWED_HOSTS.contains(&h)) {
                return Err(format!("{} doesn't come from a known download server", item.title));
            }
            if file.sha1.is_none() && file.sha256.is_none() {
                return Err(format!("{} can't be verified (no checksum)", item.title));
            }
            let dest = staging.join(&item.folder).join(&file.file_name);
            let (client, title) = (client.clone(), item.title.clone());
            jobs.spawn(async move {
                if let Some(parent) = dest.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let result = crate::minecraft::downloader::download_file_chunked(&client, &file.url, &dest, None, None).await;
                match result {
                    Ok(_) if verify(&dest, &file) => Ok(()),
                    Ok(_) => Err(format!("{title} downloaded incorrectly (checksum mismatch)")),
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

/// Carries out the reviewed plan, all or nothing. The caller then saves the new version.
pub async fn apply(
    client: &reqwest::Client,
    target: &Target,
    plan: &VersionPlan,
    backup_worlds: bool,
    progress: impl Fn(String) + Sync,
) -> Result<(), String> {
    let kinds: HashMap<&str, ContentKind> = target.folders.iter().map(|f| (f.dir, f.kind)).collect();
    for item in &plan.items {
        if kinds.get(item.folder.as_str()) != Some(&item.kind) {
            return Err(format!("{} is in an unexpected folder", item.title));
        }
        if !item.actions.contains(&item.action) {
            return Err(format!("{} can't be set to {:?}", item.title, item.action));
        }
        if let Some(name) = &item.file_name {
            safe_file_name(name, item.kind)?;
        }
    }

    // 1. Download and verify first: a failure here changes nothing
    let root = &target.root;
    let staging = work_dir(root).join("staging");
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
    let backup = backup_dir(root);
    let _ = fs::remove_dir_all(&backup);
    fs::create_dir_all(&backup).map_err(|e| format!("Failed to create the backup: {e}"))?;
    let mut journal = Journal {
        created_at: now_secs(),
        from_game_version: plan.from_game_version.clone(),
        from_loader: plan.from_loader.clone(),
        from_loader_version: plan.from_loader_version.clone(),
        to_game_version: plan.to_game_version.clone(),
        to_loader: plan.to_loader.clone(),
        ..Default::default()
    };
    let result = swap(target, &backup, &staging, plan, backup_worlds, &mut journal, &progress);
    let _ = fs::remove_dir_all(&staging);
    if let Err(e) = result {
        let _ = restore(root, &backup, &journal);
        let _ = fs::remove_dir_all(&backup);
        return Err(format!("{e}. Nothing was changed."));
    }
    write_journal(root, &journal)
}

fn write_journal(root: &Path, journal: &Journal) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(journal).map_err(|e| e.to_string())?;
    fs::write(backup_dir(root).join(JOURNAL), raw).map_err(|e| format!("Failed to save the backup: {e}"))
}

fn swap(
    target: &Target,
    backup: &Path,
    staging: &Path,
    plan: &VersionPlan,
    backup_worlds: bool,
    journal: &mut Journal,
    progress: &impl Fn(String),
) -> Result<(), String> {
    let root = &target.root;
    // The game rewrites its options and mods/plugins migrate their configs on first start
    for rel in &target.configs {
        let from = root.join(rel);
        if from.exists() {
            copy_path(&from, &backup.join("configs").join(rel)).map_err(|e| format!("Failed to back up {rel}: {e}"))?;
            journal.configs.push(rel.clone());
        }
    }
    if backup_worlds {
        for (i, world) in target.worlds.iter().enumerate() {
            progress(format!("Backing up worlds ({} of {})...", i + 1, target.worlds.len()));
            copy_path(&root.join(world), &backup.join("worlds").join(world))
                .map_err(|e| format!("Failed to back up the world {world}: {e}"))?;
            journal.worlds.push(world.clone());
        }
    }

    progress("Applying...".into());
    let move_to_backup = |rel: String, journal: &mut Journal| -> Result<(), String> {
        let stored = format!("files/{rel}");
        move_path(&root.join(&rel), &backup.join(&stored))?;
        journal.moved.push((rel, stored));
        Ok(())
    };
    for rel in &target.core_files {
        if root.join(rel).exists() {
            move_to_backup(rel.clone(), journal)?;
        }
    }
    for item in &plan.items {
        let folder = &item.folder;
        match item.action {
            ItemAction::Keep | ItemAction::Skip => {}
            ItemAction::Disable => {
                let Some(name) = &item.file_name else { continue };
                let rel = format!("{folder}/{name}");
                let disabled = root.join(format!("{rel}.disabled"));
                if disabled.exists() {
                    move_to_backup(format!("{rel}.disabled"), journal)?;
                }
                fs::rename(root.join(&rel), &disabled).map_err(|e| format!("Failed to turn off {name}: {e}"))?;
                journal.disabled.push(rel);
            }
            ItemAction::Update | ItemAction::Add => {
                let file = item.target.as_ref().ok_or_else(|| format!("{} has nothing to install", item.title))?;
                if let Some(old) = &item.file_name {
                    move_to_backup(format!("{folder}/{old}"), journal)?;
                }
                let rel = format!("{folder}/{}", file.file_name);
                if root.join(&rel).exists() {
                    // Already there (e.g. a dependency installed by hand); keep theirs aside
                    move_to_backup(rel.clone(), journal)?;
                }
                move_path(&staging.join(&rel), &root.join(&rel))?;
                journal.added.push(rel);
            }
        }
    }
    Ok(())
}

/// Puts back everything the journal lists, newest change first
fn restore(root: &Path, backup: &Path, journal: &Journal) -> Result<(), String> {
    let mut errors = Vec::new();
    for rel in journal.added.iter().rev() {
        if let Err(e) = remove_path(&root.join(rel)) {
            errors.push(format!("{rel}: {e}"));
        }
    }
    for rel in journal.disabled.iter().rev() {
        if let Err(e) = fs::rename(root.join(format!("{rel}.disabled")), root.join(rel)) {
            errors.push(format!("{rel}: {e}"));
        }
    }
    for (rel, stored) in journal.moved.iter().rev() {
        // Something new may have been created in its place (a redownloaded server.jar)
        let _ = remove_path(&root.join(rel));
        if let Err(e) = move_path(&backup.join(stored), &root.join(rel)) {
            errors.push(e);
        }
    }
    for (dir, list) in [("configs", &journal.configs), ("worlds", &journal.worlds)] {
        for rel in list {
            let dest = root.join(rel);
            let _ = remove_path(&dest);
            if let Err(e) = copy_path(&backup.join(dir).join(rel), &dest) {
                errors.push(format!("{rel}: {e}"));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Some files couldn't be restored: {}", errors.join("; ")))
    }
}

// ─── Undo ─────────────────────────────────────────────────────────────────────

fn read_journal(root: &Path) -> Option<Journal> {
    let raw = fs::read_to_string(backup_dir(root).join(JOURNAL)).ok()?;
    serde_json::from_str(&raw).ok()
}

/// The last version change, if it can be undone
pub fn last_backup(root: &Path) -> Option<VersionBackup> {
    let journal = read_journal(root)?;
    Some(VersionBackup {
        from_game_version: journal.from_game_version,
        from_loader: journal.from_loader,
        to_game_version: journal.to_game_version,
        to_loader: journal.to_loader,
        created_at: journal.created_at,
        worlds_backed_up: !journal.worlds.is_empty(),
        bytes: dir_size(&backup_dir(root)) as f64,
    })
}

/// Restores everything as it was before the last change. `game_version` and `loader` are
/// the current ones: the backup only applies to the version it changed to.
pub fn undo(root: &Path, game_version: &str, loader: &str) -> Result<Restored, String> {
    let journal = read_journal(root).ok_or("There's no version change to undo")?;
    if game_version != journal.to_game_version || loader != journal.to_loader {
        return Err(format!(
            "It's on {} {game_version} now, not {} {}, so this backup no longer matches",
            loader_label(loader),
            loader_label(&journal.to_loader),
            journal.to_game_version
        ));
    }
    let backup = backup_dir(root);
    restore(root, &backup, &journal)?;
    let _ = fs::remove_dir_all(&backup);
    Ok(Restored {
        game_version: journal.from_game_version,
        loader: journal.from_loader,
        loader_version: journal.from_loader_version,
    })
}

/// Deletes the backup of the last change (frees the space; it can't be undone after)
pub fn discard_backup(root: &Path) -> Result<(), String> {
    let backup = backup_dir(root);
    if backup.exists() {
        fs::remove_dir_all(&backup).map_err(|e| format!("Failed to delete the backup: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
