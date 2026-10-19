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
    ContentRange(#[from] ContentRangeError),

    #[error("Chunk length mismatch: expected {expected} bytes, got {actual} bytes")]
    ChunkLengthMismatch { expected: u64, actual: u64 },

    #[error("SHA-1 verification failed for downloaded file: {0}")]
    Sha1Mismatch(PathBuf),

    #[error("Download concurrency limit is closed")]
    Semaphore(#[from] tokio::sync::AcquireError),

    #[error("Asset metadata contains an invalid SHA-1 hash")]
    InvalidAssetHash,
}

impl From<reqwest::Error> for DownloadError {
    fn from(source: reqwest::Error) -> Self {
        Self::Transport(source.without_url())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentRangeField {
    Start,
    End,
    Total,
}

impl std::fmt::Display for ContentRangeField {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Start => "start offset",
            Self::End => "end offset",
            Self::Total => "total length",
        };
        formatter.write_str(name)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ContentRangeError {
    #[error("Missing Content-Range header in partial response")]
    MissingHeader,
    #[error("Content-Range header is not ASCII")]
    NonAscii(#[source] reqwest::header::ToStrError),
    #[error("Unsupported Content-Range unit")]
    UnsupportedUnit,
    #[error("Content-Range is missing the total separator")]
    MissingTotalSeparator,
    #[error("Content-Range is missing the end separator")]
    MissingEndSeparator,
    #[error("Invalid Content-Range {field}")]
    InvalidNumber {
        field: ContentRangeField,
        #[source]
        source: std::num::ParseIntError,
    },
    #[error("Content-Range mismatch: requested {expected_start}-{expected_end}, got {actual_start}-{actual_end}")]
    RangeMismatch {
        expected_start: u64,
        expected_end: u64,
        actual_start: u64,
        actual_end: u64,
    },
    #[error("Content-Range total mismatch: expected {expected}, got {actual}")]
    TotalMismatch { expected: u64, actual: u64 },
}
