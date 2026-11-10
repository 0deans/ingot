use std::path::PathBuf;
use thiserror::Error;

use crate::artifacts::sources::SourceError;
use crate::minecraft::instance::ModLoaderType;
use crate::minecraft::instance_content::error::InstanceContentError;

#[derive(Error, Debug)]
pub enum InstanceExportError {
    #[error("Unsupported mod loader: {0:?}")]
    UnsupportedLoader(ModLoaderType),

    #[error("Unsafe or invalid path: {0}")]
    UnsafePath(String),

    #[error("Instance name cannot be empty")]
    EmptyName,

    #[error("IO error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Zip archive error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Provider error: {0}")]
    Provider(#[source] SourceError),

    #[error("Instance content error: {0}")]
    Content(#[from] InstanceContentError),

    #[error("Async task join error: {0}")]
    Task(#[from] tokio::task::JoinError),

    #[error("Instance is currently running")]
    InstanceRunning,

    #[error("Instance not found: {0}")]
    NotFound(String),
}

impl InstanceExportError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
