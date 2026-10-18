use super::sources::{Scope, Source};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Everything version change keeps lives here, inside the instance or server folder
pub const WORK_DIR: &str = ".ingot/version-change";
pub const JOURNAL: &str = "journal.json";
/// Download hosts files may come from
pub const ALLOWED_HOSTS: [&str; 4] = [
    "cdn.modrinth.com",
    "edge.forgecdn.net",
    "mediafilez.forgecdn.net",
    "hangarcdn.papermc.io",
];
/// Libraries only Fabric and Quilt mods use: pointless after moving to another loader
pub const FABRIC_ONLY: [&str; 4] = ["P7dR8mSH", "qvIfYCYJ", "306612", "634179"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ContentKind {
    Mod,
    Plugin,
    ResourcePack,
    Shader,
}

impl ContentKind {
    pub fn extension(self) -> &'static str {
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

/// Transaction and recovery status for an instance or server
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VersionTransactionStatus {
    pub phase: Option<TransactionPhase>,
    pub is_incomplete: bool,
    pub can_recover: bool,
    pub backup: Option<VersionBackup>,
    pub error: Option<String>,
}

/// A folder of content to check
#[derive(Clone)]
pub struct Folder {
    pub kind: ContentKind,
    pub dir: &'static str,
    pub scope: Scope,
}

/// What a change covers: the instance or server folder and what's in it
#[derive(Clone)]
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
    pub hangar: HashMap<String, TrackedHangarPlugin>,
}

#[derive(Clone)]
pub struct TrackedHangarPlugin {
    pub project_id: String,
    pub version_id: String,
    pub title: String,
    pub icon_url: Option<String>,
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

/// Phase of an active or completed version change transaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TransactionPhase {
    Prepared,
    Swapping,
    #[default]
    Committed,
    RollbackFailed,
    Restoring,
    Restored,
}

#[derive(Debug, Error)]
pub enum VersionChangeError {
    #[error("Failed to {operation} recovery journal at {path}: {source}")]
    JournalIo {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Cannot encode recovery journal")]
    JournalEncode(#[source] serde_json::Error),
    #[error(transparent)]
    Source(#[from] super::sources::SourceError),
    #[error("Unexpected file name: {0}")]
    InvalidFileName(String),
    #[error("Unexpected path: {0}")]
    InvalidPath(String),
    #[error("Failed to {operation} at {path}: {source}")]
    DownloadPrep {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to inspect directory at {path}: {source}")]
    ScanDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to read file at {path}: {source}")]
    ScanFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Download error for {title}: {source}")]
    DownloadFailed {
        title: String,
        #[source]
        source: crate::minecraft::downloader::DownloadError,
    },
    #[error("Version change download task failed: {0}")]
    Task(#[from] tokio::task::JoinError),
    #[error("{0} downloaded incorrectly (checksum mismatch)")]
    ChecksumMismatch(String),
    #[error("{0} doesn't come from a known download server")]
    UnknownHost(String),
    #[error("{0} can't be verified (no checksum)")]
    NoChecksum(String),
    #[error("Plan item error: {0}")]
    PlanError(String),
    #[error("Invalid version target: {0}")]
    InvalidTarget(String),
    #[error("Failed to create backup: {0}")]
    BackupError(String),
    #[error("File operation error: {0}")]
    FileOp(String),
    #[error("Undo error: {0}")]
    UndoError(String),
    #[error("Some files couldn't be restored: {0}")]
    RestoreError(String),
    #[error("Migration failed: {primary}. Rollback also failed: {restore}. Recovery backup preserved at {backup_path}")]
    RollbackFailed {
        primary: String,
        restore: String,
        backup_path: PathBuf,
    },
    #[error("Migration failed: {0}. All changes were cleanly rolled back.")]
    RollbackSucceeded(String),
    #[error("{operation} failed; recovery material retained at {backup_path}: {source}")]
    Finalization {
        operation: &'static str,
        backup_path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Files restored, but metadata commit failed: {message}. Recovery material retained at {backup_path}")]
    MetadataAfterRestore {
        message: String,
        backup_path: PathBuf,
    },
    #[error("Files and metadata changed, but journal finalization failed: {source}. Recovery material retained at {backup_path}")]
    CommitJournal {
        backup_path: PathBuf,
        #[source]
        source: Box<VersionChangeError>,
    },
    #[error("{0}")]
    Custom(String),
}

pub fn work_dir(root: &Path) -> PathBuf {
    root.join(WORK_DIR)
}

pub fn backup_dir(root: &Path) -> PathBuf {
    work_dir(root).join("backup")
}

pub fn sha1_hex(data: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    Sha1::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn dir_size_typed(path: &Path) -> Result<u64, std::io::Error> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e),
    };
    if meta.is_file() {
        return Ok(meta.len());
    }
    if !meta.is_dir() {
        return Ok(0);
    }
    let mut total = 0u64;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        total += dir_size_typed(&entry.path())?;
    }
    Ok(total)
}

/// Only a plain file name (no folders), with the expected extension
pub fn safe_file_name_typed(name: &str, kind: ContentKind) -> Result<&str, VersionChangeError> {
    let ok = !name.is_empty()
        && !name.starts_with('.')
        && !name.contains(['/', '\\', ':'])
        && name.to_ascii_lowercase().ends_with(kind.extension());
    if ok {
        Ok(name)
    } else {
        Err(VersionChangeError::InvalidFileName(name.to_string()))
    }
}

pub fn safe_file_name(name: &str, kind: ContentKind) -> Result<&str, VersionChangeError> {
    safe_file_name_typed(name, kind)
}

/// A path inside the target given by the frontend or the journal: relative, no ".."
pub fn safe_rel_typed(rel: &str) -> Result<&str, VersionChangeError> {
    let path = Path::new(rel);
    let ok = !rel.is_empty()
        && !rel.contains([':', '\0'])
        && !rel.starts_with(['/', '\\'])
        && rel
            .split(['/', '\\'])
            .all(|part| !matches!(part, "" | "." | ".."))
        && path
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)));
    if ok {
        Ok(rel)
    } else {
        Err(VersionChangeError::InvalidPath(rel.to_string()))
    }
}

pub fn safe_rel(rel: &str) -> Result<&str, VersionChangeError> {
    safe_rel_typed(rel)
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Marks a project in a note; replaced by its title once titles are known
pub fn mention(project: &str) -> String {
    format!("\u{1}{project}\u{1}")
}

pub fn loader_label(loader: &str) -> String {
    match loader {
        "neoforge" => "NeoForge".into(),
        other => {
            let mut chars = other.chars();
            chars
                .next()
                .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        }
    }
}
