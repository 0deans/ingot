#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error(transparent)]
    Port(#[from] super::models::PortError),
    #[error("Server name cannot be empty")]
    EmptyName,
    #[error("Invalid server memory range: {minimum_mb}–{maximum_mb} MiB")]
    InvalidMemory { minimum_mb: u32, maximum_mb: u32 },
    #[error(transparent)]
    Metadata(#[from] crate::minecraft::metadata::MetadataError),
    #[error("Server file I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid server identifier")]
    InvalidId,
    #[error("Server directory resolution failed: {0}")]
    Directory(String),
    #[error(transparent)]
    File(#[from] crate::server::files::ServerFileError),
    #[error("Invalid server port")]
    InvalidPort,
}
