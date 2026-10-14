//! Typed error models for content searching, details retrieval, and installation.
use reqwest::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum ContentError {
    #[error("Network transport failure: {0}")]
    Transport(#[source] reqwest::Error),

    #[error("Provider responded with HTTP status {0}")]
    HttpStatus(StatusCode),

    #[error("Failed to parse provider response: {0}")]
    Response(#[source] reqwest::Error),

    #[error("JSON deserialization error: {0}")]
    Json(#[source] serde_json::Error),

    #[error("Filesystem I/O error: {0}")]
    Io(#[source] std::io::Error),

    #[error("Archive error: {0}")]
    Zip(#[source] zip::result::ZipError),

    #[error("Unsupported or unknown content source: '{0}'")]
    UnknownSource(String),

    #[error("Missing required field: '{0}'")]
    MissingField(&'static str),

    #[error("Invalid filename: '{0}'")]
    InvalidFilename(String),

    #[error("Failed to build HTTP client: {0}")]
    ClientInit(#[source] reqwest::Error),

    #[error("Modpack archive missing required manifest ({0})")]
    MissingManifest(&'static str),

    #[error("Instance error: {0}")]
    Instance(String),
}

impl From<ContentError> for String {
    fn from(error: ContentError) -> Self {
        error.to_string()
    }
}
