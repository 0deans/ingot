use std::{fmt, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VersionChangeError {
    #[error("Failed to {operation} recovery journal at {path}: {source}")]
    JournalIo {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Cannot encode recovery journal")]
    JournalEncode(#[source] serde_json::Error),
    #[error(transparent)]
    Source(#[from] super::sources::SourceError),
    #[error("Unexpected file name: {0}")]
    InvalidFileName(String),
    #[error("Unexpected path: {0}")]
    InvalidPath(String),
    #[error("Failed to {operation} at {path}: {source}")]
    DownloadPrep {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to inspect directory at {path}: {source}")]
    ScanDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to read file at {path}: {source}")]
    ScanFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Download error for {title}: {source}")]
    DownloadFailed {
        title: String,
        #[source]
        source: crate::minecraft::downloader::DownloadError,
    },
    #[error("Version change download task failed: {0}")]
    Task(#[from] tokio::task::JoinError),
    #[error("{0} downloaded incorrectly (checksum mismatch)")]
    ChecksumMismatch(String),
    #[error("{0} doesn't come from a known download server")]
    UnknownHost(String),
    #[error("{0} can't be verified (no checksum)")]
    NoChecksum(String),
    #[error("Plan item error: {0}")]
    PlanError(String),
    #[error("Invalid version target: {0}")]
    InvalidTarget(String),
    #[error("Failed to {operation} at {path}: {source}")]
    FileIo {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Metadata commit failed: {0}")]
    MetadataCommit(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("Recovery journal has an invalid format: {0}")]
    JournalDecode(#[source] serde_json::Error),
    #[error("Recovery is unavailable: {0}")]
    RecoveryUnavailable(&'static str),
    #[error("A transaction in phase {0:?} requires recovery")]
    IncompleteTransaction(super::models::TransactionPhase),
    #[error("No version change is available")]
    NoVersionChange,
    #[error("File is already disabled: {0}")]
    AlreadyDisabled(String),
    #[error("Recovery source is missing: {0}")]
    MissingRecoveryFile(PathBuf),
    #[error("Undo error: {0}")]
    UndoError(String),
    #[error("Some files could not be restored: {0}")]
    RestoreFailures(#[source] RestorationFailures),
    #[error("Migration failed: {primary}. Rollback also failed: {restore}. Checkpoint failure: {checkpoint:?}. Recovery backup preserved at {backup_path}")]
    RollbackFailed {
        #[source]
        primary: Box<VersionChangeError>,
        restore: Box<VersionChangeError>,
        checkpoint: Option<Box<VersionChangeError>>,
        backup_path: PathBuf,
    },
    #[error("Migration failed: {0}. All changes were cleanly rolled back.")]
    RollbackSucceeded(#[source] Box<VersionChangeError>),
    #[error("{operation} failed; recovery material retained at {backup_path}: {source}")]
    Finalization {
        operation: &'static str,
        backup_path: PathBuf,
        #[source]
        source: Box<VersionChangeError>,
    },
    #[error("Files restored, but metadata commit failed: {source}. Recovery material retained at {backup_path}")]
    MetadataAfterRestore {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
        backup_path: PathBuf,
    },
    #[error("Files and metadata changed, but journal finalization failed: {source}. Recovery material retained at {backup_path}")]
    CommitJournal {
        backup_path: PathBuf,
        #[source]
        source: Box<VersionChangeError>,
    },
    #[error("{primary}; additional operation failed: {secondary}")]
    AdditionalFailure {
        #[source]
        primary: Box<VersionChangeError>,
        secondary: Box<VersionChangeError>,
    },
}

#[derive(Debug)]
pub struct RestorationFailures(pub Vec<VersionChangeError>);
impl fmt::Display for RestorationFailures {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, failure) in self.0.iter().enumerate() {
            if index > 0 {
                write!(formatter, "; ")?;
            }
            write!(formatter, "{failure}")?;
        }
        Ok(())
    }
}
impl std::error::Error for RestorationFailures {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.first().map(|error| error as &dyn std::error::Error)
    }
}
pub(super) fn file_io(
    operation: &'static str,
    path: &std::path::Path,
    source: std::io::Error,
) -> VersionChangeError {
    VersionChangeError::FileIo {
        operation,
        path: path.to_owned(),
        source,
    }
}
