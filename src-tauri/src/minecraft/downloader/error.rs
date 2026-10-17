use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("Duplicate download destination: {0}")]
    Duplicate(PathBuf),

    #[error("Download transport failed")]
    Transport(#[source] reqwest::Error),

    #[error("Downloaded content does not match its declared {0}")]
    Integrity(&'static str),

    #[error("Download filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("Download task failed: {0}")]
    Task(#[from] tokio::task::JoinError),

    #[error("Download provider returned HTTP {status}")]
    HttpStatus { status: reqwest::StatusCode },

    #[error("Content-Range header error: {0}")]
    ContentRange(String),

    #[error("Chunk length mismatch: expected {expected} bytes, got {actual} bytes")]
    ChunkLengthMismatch { expected: u64, actual: u64 },

    #[error("SHA-1 verification failed for downloaded file: {0}")]
    Sha1Mismatch(PathBuf),

    #[error("{0}")]
    Other(String),
}

impl From<reqwest::Error> for DownloadError {
    fn from(source: reqwest::Error) -> Self {
        Self::Transport(source.without_url())
    }
}
