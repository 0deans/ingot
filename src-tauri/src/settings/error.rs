use std::{io, path::PathBuf};

/// Internal failures retain their causes; the legacy RPC adapter renders them.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("Failed to {operation} settings at {}: {source}", path.display())]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Failed to parse settings at {}: {source}", path.display())]
    Decode {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("Failed to serialize settings: {0}")]
    Encode(#[from] serde_json::Error),
    #[error("Memory limits must be positive and ordered (min: {min_mb}, max: {max_mb})")]
    InvalidMemory { min_mb: u32, max_mb: u32 },
    #[error("Window dimensions must be positive (width: {width}, height: {height})")]
    InvalidWindow { width: u32, height: u32 },
    #[error("Invalid launcher behavior: {0}")]
    InvalidBehavior(String),
    #[error("Settings store lock poisoned")]
    LockPoisoned,
    #[error("Application settings state is not initialized")]
    StateUnavailable,
}
