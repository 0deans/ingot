use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const WORK_DIR: &str = ".ingot/version-change";
pub const JOURNAL: &str = "journal.json";

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TargetKind {
    Instance,
    Server,
}

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
    pub source: Option<crate::sources::Source>,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedHangarPlugin {
    pub project_id: String,
    pub version_id: String,
    pub title: String,
    pub icon_url: Option<String>,
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Journal {
    pub created_at: u64,
    pub from_game_version: String,
    pub from_loader: String,
    pub from_loader_version: Option<String>,
    pub to_game_version: String,
    pub to_loader: String,
    #[serde(default)]
    pub phase: TransactionPhase,
    pub moved: Vec<(String, String)>,
    pub disabled: Vec<String>,
    pub added: Vec<String>,
    pub configs: Vec<String>,
    #[serde(default)]
    pub absent_configs: Vec<String>,
    pub worlds: Vec<String>,
    pub first_start_checked: bool,
}

pub fn safe_file_name_typed(name: &str, kind: ContentKind) -> Result<&str, String> {
    let ok = !name.is_empty()
        && !name.starts_with('.')
        && !name.contains(['/', '\\', ':'])
        && name.to_ascii_lowercase().ends_with(kind.extension());
    if ok {
        Ok(name)
    } else {
        Err(name.to_string())
    }
}

pub fn safe_rel_typed(rel: &str) -> Result<&str, String> {
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
        Err(rel.to_string())
    }
}
