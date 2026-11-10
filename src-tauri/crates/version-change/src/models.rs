use super::sources::Scope;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use crate::artifacts::models::{
    backup_dir, sha1_hex, sha256_hex, work_dir, ContentKind, ItemAction, ItemStatus, Journal,
    PlanItem, TargetFile, TargetKind, TrackedHangarPlugin, TransactionPhase, JOURNAL, WORK_DIR,
};

/// Download hosts files may come from
pub const ALLOWED_HOSTS: [&str; 4] = [
    "cdn.modrinth.com",
    "edge.forgecdn.net",
    "mediafilez.forgecdn.net",
    "hangarcdn.papermc.io",
];
/// Libraries only Fabric and Quilt mods use: pointless after moving to another loader
pub const FABRIC_ONLY: [&str; 4] = ["P7dR8mSH", "qvIfYCYJ", "306612", "634179"];

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WorldInfo {
    /// Folder, relative to the instance or server
    pub folder_name: String,
    #[specta(type = i32)]
    pub bytes: f64,
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

pub use super::error::VersionChangeError;

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
    crate::artifacts::models::safe_file_name_typed(name, kind)
        .map_err(VersionChangeError::InvalidFileName)
}

pub fn safe_file_name(name: &str, kind: ContentKind) -> Result<&str, VersionChangeError> {
    safe_file_name_typed(name, kind)
}

/// A path inside the target given by the frontend or the journal: relative, no ".."
pub fn safe_rel_typed(rel: &str) -> Result<&str, VersionChangeError> {
    crate::artifacts::models::safe_rel_typed(rel).map_err(VersionChangeError::InvalidPath)
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
