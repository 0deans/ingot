use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum JavaError {
    #[error("Java runtime filesystem operation failed")]
    Io(#[from] std::io::Error),
    #[error("Java runtime download failed")]
    Http(#[from] reqwest::Error),
    #[error("Java runtime archive is invalid")]
    Zip(#[from] zip::result::ZipError),
    #[error("Java runtime archive contains an unsafe entry")]
    UnsafeArchive,
    #[error("Java runtime contains no executable")]
    MissingBinary,
    #[error("Java executable did not report a valid version")]
    InvalidVersion,
    #[error("Java executable exited unsuccessfully")]
    ProbeFailed,
    #[error("Java probing was cancelled")]
    ProbeCancelled,
    #[error("Java probe timed out after {0:?}")]
    ProbeTimeout(Duration),
    #[error("Failed to spawn Java probe process")]
    ProbeSpawn(#[source] std::io::Error),
    #[error(
        "Failed to clean up Java probe process: {cleanup}{}",
        format_primary(primary)
    )]
    ProbeCleanup {
        primary: Option<Box<JavaError>>,
        #[source]
        cleanup: std::io::Error,
    },
    #[error("Java {actual} does not match required Java {required}")]
    VersionMismatch { required: u32, actual: u32 },
    #[error("Java runtime location is invalid")]
    InvalidLocation,
    #[error("Java runtime worker failed")]
    Task(#[from] tokio::task::JoinError),
}

fn format_primary(primary: &Option<Box<JavaError>>) -> String {
    match primary {
        Some(err) => format!(" (after primary failure: {err})"),
        None => String::new(),
    }
}
