//! Typed error models for Ely.by catalog, web session, and skin management.
use reqwest::StatusCode;

#[derive(thiserror::Error)]
pub enum ElyError {
    #[error("Incorrect Ely.by password")]
    IncorrectPassword,

    #[error("Ely.by Two-Factor Authentication (TOTP) is required on this account")]
    TwoFactorRequired,

    #[error("Ely.by authentication failed")]
    AuthenticationRejected,

    #[error("Ely.by login redirect not found")]
    MissingRedirect,

    #[error("Missing access_token in Ely.by response")]
    MissingAccessToken,

    #[error("Ely.by responded with HTTP status {0}")]
    HttpStatus(StatusCode),

    #[error("Ely.by OAuth completion failed (HTTP {status})")]
    OAuthCompletion { status: StatusCode },

    #[error("Missing redirectUri from Ely.by OAuth response")]
    MissingRedirectUri,

    #[error("Failed to construct multipart form")]
    Multipart(#[source] reqwest::Error),

    #[error("Failed to parse skins catalog response")]
    ParseCatalog(#[source] reqwest::Error),

    #[error("Failed to parse response")]
    ParseResponse(#[source] reqwest::Error),

    #[error("Ely.by rejected the skin operation")]
    ApiRejected,

    #[error("Skin {skin_id} was uploaded, but could not be applied: {source}")]
    UploadedNotApplied {
        skin_id: u64,
        #[source]
        source: Box<ElyError>,
    },

    #[error("Network transport failure")]
    Transport(#[source] reqwest::Error),

    #[error("Failed to create HTTP client")]
    ClientInit(#[source] reqwest::Error),
}

impl std::fmt::Debug for ElyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Response decode sources can contain credential-bearing JSON values.
        formatter
            .debug_struct("ElyError")
            .field("message", &self.to_string())
            .finish()
    }
}
