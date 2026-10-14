//! Typed error models for Ely.by catalog, web session, and skin management.
use reqwest::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum ElyError {
    #[error("Incorrect Ely.by password")]
    IncorrectPassword,

    #[error("Ely.by Two-Factor Authentication (TOTP) is required on this account")]
    TwoFactorRequired,

    #[error("Account error: {0}")]
    Account(String),

    #[error("Ely.by login redirect not found")]
    MissingRedirect,

    #[error("Missing access_token in Ely.by response")]
    MissingAccessToken,

    #[error("Ely.by responded with HTTP status {0}")]
    HttpStatus(StatusCode),

    #[error("Ely.by OAuth completion error ({status}): {message}")]
    OAuthCompletion { status: StatusCode, message: String },

    #[error("Missing redirectUri from Ely.by OAuth response: {0}")]
    MissingRedirectUri(String),

    #[error("Failed to construct multipart form: {0}")]
    Multipart(#[source] reqwest::Error),

    #[error("Failed to parse skins catalog response: {0}")]
    ParseCatalog(#[source] reqwest::Error),

    #[error("Failed to parse response: {0}")]
    ParseResponse(#[source] reqwest::Error),

    #[error("Ely.by API error: {0}")]
    Api(String),

    #[error("Network transport failure: {0}")]
    Transport(#[source] reqwest::Error),

    #[error("Failed to create HTTP client: {0}")]
    ClientInit(#[source] reqwest::Error),
}

impl From<ElyError> for String {
    fn from(error: ElyError) -> Self {
        error.to_string()
    }
}
