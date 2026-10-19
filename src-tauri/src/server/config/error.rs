#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error(transparent)]
    Port(#[from] super::models::PortError),
    #[error("Server name cannot be empty")]
    EmptyName,
    #[error("Invalid server memory range: {minimum_mb}–{maximum_mb} MiB")]
    InvalidMemory { minimum_mb: u32, maximum_mb: u32 },
    #[error(transparent)]
    Metadata(crate::minecraft::metadata::MetadataError),
    #[error("Server file I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid server identifier")]
    InvalidId,
    #[error("Server directory resolution failed: {0}")]
    Directory(#[source] std::io::Error),
    #[error(transparent)]
    File(#[from] crate::server::files::ServerFileError),
    #[error("Invalid server port")]
    InvalidPort,
}

impl From<crate::minecraft::metadata::MetadataError> for ConfigError {
    fn from(error: crate::minecraft::metadata::MetadataError) -> Self {
        use crate::minecraft::metadata::MetadataError;
        match error {
            MetadataError::Validation(source) => {
                match source.downcast::<super::models::PortError>() {
                    Ok(error) => Self::Port(*error),
                    Err(source) => Self::Metadata(MetadataError::Validation(source)),
                }
            }
            error => Self::Metadata(error),
        }
    }
}
