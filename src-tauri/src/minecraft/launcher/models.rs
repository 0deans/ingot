use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstanceStatusEvent {
    pub instance_id: String,
    pub is_running: bool,
    pub pid: u32,
    pub started_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LaunchProgressEvent {
    pub instance_id: String,
    pub phase: String,
    pub current_step: u64,
    pub total_steps: u64,
    pub percentage: f32,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunningInstanceSummary {
    pub instance_id: String,
    pub pid: u32,
    pub started_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QuickPlayOptions {
    pub server: Option<String>,
    pub world: Option<String>,
}

pub type StatusFn = Arc<dyn Fn(InstanceStatusEvent) + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    #[error(transparent)]
    Metadata(#[from] crate::minecraft::metadata::MetadataError),

    #[error("Instance {0} not found")]
    InstanceNotFound(String),
    #[error("Instance is already running")]
    AlreadyRunning,

    #[error("Instance {0} is not running")]
    NotRunning(String),

    #[error("Loader version not selected")]
    LoaderVersionNotSelected,

    #[error("Failed to get app data directory: {0}")]
    AppDataDir(String),

    #[error("Instance directory error: {0}")]
    InstanceDir(String),

    #[error("Version package error: {0}")]
    VersionPackage(String),

    #[error("Mod loader profile error: {0}")]
    LoaderProfile(String),

    #[error("Java runtime error: {0}")]
    JavaRuntime(String),

    #[error("Download error: {0}")]
    Download(String),

    #[error("Account error: {0}")]
    Account(String),

    #[error("Settings error: {0}")]
    Settings(String),

    #[error("Failed to spawn Minecraft process: {0}")]
    ProcessSpawn(#[source] std::io::Error),

    #[error("Version change transaction is in incomplete state: {0}. Please recover or revert it first.")]
    IncompleteTransaction(String),

    #[error("I/O error during launch: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

impl From<LaunchError> for String {
    fn from(err: LaunchError) -> Self {
        err.to_string()
    }
}
