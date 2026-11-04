use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum InstanceContentError {
    #[error("Instance '{0}' does not exist")]
    InstanceNotFound(String),

    #[error("Instance '{0}' is currently running")]
    InstanceRunning(String),

    #[error("Invalid filename '{filename}': {reason}")]
    InvalidFileName { filename: String, reason: String },

    #[error("Target file '{0}' already exists")]
    NameConflict(String),

    #[error("Content file '{0}' not found")]
    NotFound(String),

    #[error("I/O error at '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("World '{0}' not found in instance")]
    WorldNotFound(String),

    #[error("Operation cannot be performed: {0}")]
    InvalidOperation(String),
}

impl InstanceContentError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    pub fn invalid_file_name(filename: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidFileName {
            filename: filename.into(),
            reason: reason.into(),
        }
    }
}
