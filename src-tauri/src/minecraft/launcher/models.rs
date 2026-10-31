use crate::account::{recovery::RecoveryError, session::SessionError};
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

    #[error("Instance maintenance is in progress")]
    Maintenance,

    #[error("Loader version not selected")]
    LoaderVersionNotSelected,

    #[error("Failed to get app data directory: {0}")]
    AppDataDir(#[source] std::io::Error),

    #[error("Instance directory error: {0}")]
    InstanceDir(#[source] crate::minecraft::instance::InstanceError),

    #[error("Version package error: {0}")]
    VersionPackage(#[source] crate::minecraft::version::VersionError),

    #[error("Mod loader profile error: {0}")]
    LoaderProfile(#[source] crate::minecraft::loader::LoaderError),

    #[error("Java runtime error: {0}")]
    JavaRuntime(#[source] crate::minecraft::java::JavaError),

    #[error("Download error: {0}")]
    Download(#[from] crate::minecraft::downloader::DownloadError),

    #[error("Account error: {0}")]
    Account(#[source] LaunchAccountError),

    #[error("Settings error: {0}")]
    Settings(#[source] crate::settings::SettingsError),

    #[error("Failed to spawn Minecraft process: {0}")]
    ProcessSpawn(#[source] std::io::Error),

    #[error("Version change transaction is in incomplete state: {0:?}. Please recover or revert it first.")]
    IncompleteTransaction(crate::version_change::TransactionPhase),

    #[error("Cannot inspect version change recovery state: {0}")]
    TransactionInspection(#[source] Box<crate::version_change::VersionChangeError>),

    #[error("I/O error during launch: {0}")]
    Io(#[from] std::io::Error),

    #[error("Game process registry is poisoned")]
    RegistryPoisoned,

    #[error("Launch reservation was cancelled or replaced")]
    ReservationCancelled,

    #[error("Game termination failed: {0}")]
    Termination(#[from] super::termination::TerminationError),

    #[error("Game process worker failed: {0}")]
    Task(#[from] tokio::task::JoinError),

    #[error("Horizon engine error: {0}")]
    Horizon(#[from] crate::minecraft::horizon::HorizonError),
}

#[derive(Debug, thiserror::Error)]
#[error("{source}")]
pub struct LaunchAccountError {
    #[source]
    pub(crate) source: Box<LaunchAccountCause>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum LaunchAccountCause {
    #[error("Account loading failed: {0}")]
    Recovery(#[source] RecoveryError),
    #[error("Account authentication failed: {0}")]
    Session(#[source] SessionError),
}

impl From<RecoveryError> for LaunchAccountError {
    fn from(source: RecoveryError) -> Self {
        Self {
            source: Box::new(LaunchAccountCause::Recovery(source)),
        }
    }
}

impl From<SessionError> for LaunchAccountError {
    fn from(source: SessionError) -> Self {
        Self {
            source: Box::new(LaunchAccountCause::Session(source)),
        }
    }
}
