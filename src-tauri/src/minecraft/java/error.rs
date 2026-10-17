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
    #[error("Java {actual} does not match required Java {required}")]
    VersionMismatch { required: u32, actual: u32 },
    #[error("Java runtime location is invalid")]
    InvalidLocation,
    #[error("Java runtime worker failed")]
    Task(#[from] tokio::task::JoinError),
}
