use super::models::WorldDimension;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorldError {
    #[error("Instance not found: {0}")]
    InstanceNotFound(String),

    #[error("World not found: {0}")]
    NotFound(String),

    #[error("Instance is currently running or in maintenance")]
    InstanceRunning,

    #[error("Invalid world folder name: {0}")]
    InvalidFolderName(String),

    #[error("Dimension not found for reset: {0:?}")]
    DimensionNotFound(WorldDimension),

    #[error("Archive error: {0}")]
    Archive(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Gzip error: {0}")]
    Gz(String),

    #[error("NBT parsing error: {0}")]
    Nbt(String),

    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Task join error: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),

    #[error("Failed to load instances: {0}")]
    LoadInstances(#[source] crate::minecraft::instance::InstanceError),

    #[error("Failed to locate instance directory: {0}")]
    InstanceDir(#[source] crate::minecraft::instance::InstanceError),
}
