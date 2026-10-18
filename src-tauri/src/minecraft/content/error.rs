//! Typed error models for content searching, details retrieval, and installation.
use reqwest::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum ContentError {
    #[error("Content pagination exceeds the supported range")]
    InvalidPagination,
    #[error("{provider} content request failed: {source}")]
    Provider {
        provider: &'static str,
        #[source]
        source: Box<ContentError>,
    },
    #[error("Both content providers failed: Modrinth: {modrinth}; CurseForge: {curseforge}")]
    ProvidersUnavailable {
        #[source]
        modrinth: Box<ContentError>,
        curseforge: Box<ContentError>,
    },
    #[error("Instance metadata publication failed: {0}")]
    Metadata(#[source] crate::minecraft::metadata::MetadataError),
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

    #[error("Modpack archive missing required manifest ({0})")]
    MissingManifest(&'static str),

    #[error("Instance error: {0}")]
    Instance(#[source] crate::minecraft::instance::InstanceError),

    #[error("Modpack contains an invalid or unsafe path: {0}")]
    UnsafePath(String),

    #[error("Modpack download failed integrity validation: {0}")]
    Integrity(&'static str),

    #[error("Modpack blocking task failed: {0}")]
    Task(#[source] tokio::task::JoinError),

    #[error("Modpack failed ({primary}); staging cleanup also failed: {cleanup}")]
    Cleanup {
        #[source]
        primary: Box<ContentError>,
        cleanup: std::io::Error,
    },
}
