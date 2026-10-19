//! Data models and DTOs for launcher detection, instance import, and reporting.
use crate::minecraft::instance::ModLoaderType;

#[taurpc::ipc_type]
#[derive(Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DetectedLauncher {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub path: String,
    pub instances_count: u32,
    pub available: bool,
}

#[taurpc::ipc_type]
#[derive(Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportableInstance {
    pub id: String,
    pub launcher_id: String,
    pub name: String,
    pub game_version: String,
    pub loader: ModLoaderType,
    pub loader_version: Option<String>,
    pub icon: Option<String>,
    pub source_path: String,
    pub mods_count: u32,
    pub saves_count: u32,
    pub screenshots_count: u32,
}

#[taurpc::ipc_type]
#[derive(Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportInstanceOptions {
    pub source_path: String,
    pub launcher_id: String,
    pub name: String,
    pub game_version: String,
    pub loader: ModLoaderType,
    pub loader_version: Option<String>,
    pub copy_mods: bool,
    pub copy_configs: bool,
    pub copy_saves: bool,
    pub copy_resource_packs: bool,
    pub copy_options: bool,
    pub copy_servers: bool,
    pub copy_screenshots: bool,
    pub copy_extra_data: bool,
}

#[taurpc::ipc_type]
#[derive(Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub success: bool,
    pub instance_id: String,
    pub name: String,
    pub mods_copied: u32,
    pub saves_copied: u32,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ImportError {
    #[error("Import source must not contain the destination instances directory")]
    OverlappingPaths,
    #[error("Import background task failed")]
    Task(#[from] tokio::task::JoinError),

    #[error("Import metadata publication failed: {0}")]
    Metadata(#[source] crate::minecraft::metadata::MetadataError),

    #[error("{primary}; import staging cleanup also failed: {cleanup}")]
    Cleanup {
        #[source]
        primary: Box<ImportError>,
        cleanup: std::io::Error,
    },
    #[error("Source directory does not exist: {0}")]
    SourceNotFound(String),

    #[error("Instance creation failed: {0}")]
    InstanceCreate(#[source] crate::minecraft::instance::InstanceError),

    #[error("Failed to resolve instance directory: {0}")]
    InstanceDir(#[source] crate::minecraft::instance::InstanceError),

    #[error("Failed to resolve native import paths: {0}")]
    NativePath(#[source] std::io::Error),

    #[error("I/O error during import: {0}")]
    Io(#[source] std::io::Error),

    #[error("Unknown launcher ID: '{0}'")]
    UnknownLauncher(String),
}
