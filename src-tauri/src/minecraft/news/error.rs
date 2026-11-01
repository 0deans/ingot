use thiserror::Error;

#[derive(Debug, Error)]
pub enum NewsError {
    #[error("Network error while contacting Mojang content services: {0}")]
    Network(#[from] reqwest::Error),

    #[error("I/O error during news cache access: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse Mojang content response: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("Invalid patch note content path: {0}")]
    InvalidContentPath(String),

    #[error("Article '{0}' not found in news feed")]
    ArticleNotFound(String),

    #[error("Mojang content service returned status {0}")]
    HttpStatus(reqwest::StatusCode),

    #[error("Background task join error: {0}")]
    Task(#[from] tokio::task::JoinError),
}
