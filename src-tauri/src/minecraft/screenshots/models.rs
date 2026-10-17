//! Data models and typed errors for screenshots management.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotInfo {
    pub id: String,
    pub instance_id: String,
    pub instance_name: String,
    pub file_name: String,
    pub file_path: String,
    pub thumbnail_path: Option<String>,
    pub file_size_bytes: u64,
    pub created_at: u64,
    pub modified_at: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum ScreenshotError {
    #[error("Screenshot metadata changed; reload the screenshot list")]
    StaleMetadata,

    #[error("More than one screenshot matches '{0}'; select a unique capture")]
    AmbiguousFilename(String),

    #[error("Screenshot background task failed")]
    Task(#[from] tokio::task::JoinError),
    #[error("Screenshot file not found: {0}")]
    NotFound(String),

    #[error("Invalid screenshot file name: '{0}'")]
    InvalidFilename(String),

    #[error("I/O error: {0}")]
    Io(#[source] std::io::Error),

    #[error("Image processing error for {path}: {source}")]
    Image {
        path: String,
        #[source]
        source: image::ImageError,
    },

    #[error("Failed to resolve instance directory: {0}")]
    InstanceDir(String),

    #[error("Failed to load instances: {0}")]
    LoadInstances(String),

    #[error("Cache directory resolution error: {0}")]
    CacheDir(String),
}

impl From<ScreenshotError> for String {
    fn from(err: ScreenshotError) -> Self {
        err.to_string()
    }
}
