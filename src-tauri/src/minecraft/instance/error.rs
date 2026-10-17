use crate::minecraft::metadata::MetadataError;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum InstanceError {
    #[error(transparent)]
    Metadata(#[from] MetadataError),
    #[error("Instance file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Instance path resolution failed: {0}")]
    Directory(String),
    #[error("Invalid instance identifier")]
    InvalidId,
    #[error("Give the instance a name")]
    EmptyName,
    #[error("Instance not found: {0}")]
    NotFound(String),
    #[error("{primary}; staging cleanup also failed: {cleanup}")]
    Cleanup {
        #[source]
        primary: Box<InstanceError>,
        cleanup: std::io::Error,
    },
    #[error("Instance metadata was deleted, but files remain at {path}: {source}")]
    FilesCleanupCommitted {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
