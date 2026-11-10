use reqwest::{Client, RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;
use thiserror::Error;

use super::Source;

#[derive(Error)]
pub enum SourceError {
    #[error("{provider:?} request failed")]
    Transport {
        provider: Source,
        #[source]
        source: reqwest::Error,
    },
    #[error("{provider:?} returned HTTP {status}")]
    HttpStatus {
        provider: Source,
        status: StatusCode,
    },
    #[error("{provider:?} returned an invalid response")]
    Decode {
        provider: Source,
        #[source]
        source: reqwest::Error,
    },
    #[error("{provider:?} returned invalid {field}")]
    InvalidPayload {
        provider: Source,
        field: &'static str,
    },
    #[error("Invalid {provider:?} project identifier")]
    InvalidProjectId { provider: Source },
}

impl std::fmt::Debug for SourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, formatter)
    }
}

pub struct SourceClient<'a> {
    pub(super) http: &'a Client,
    pub(super) modrinth: String,
    pub(super) curseforge: String,
    pub(super) hangar: String,
}

impl<'a> SourceClient<'a> {
    pub fn new(http: &'a Client) -> Self {
        Self::with_endpoints(
            http,
            "https://api.modrinth.com/v2",
            "https://api.curseforge.com/v1",
            "https://hangar.papermc.io/api/v1",
        )
    }

    pub fn with_endpoints(
        http: &'a Client,
        modrinth: &str,
        curseforge: &str,
        hangar: &str,
    ) -> Self {
        Self {
            http,
            modrinth: modrinth.trim_end_matches('/').to_owned(),
            curseforge: curseforge.trim_end_matches('/').to_owned(),
            hangar: hangar.trim_end_matches('/').to_owned(),
        }
    }
}

async fn response(
    request: RequestBuilder,
    provider: Source,
) -> Result<reqwest::Response, SourceError> {
    request
        .send()
        .await
        .map_err(|source| SourceError::Transport {
            provider,
            source: source.without_url(),
        })
}

async fn decode<T: DeserializeOwned>(
    response: reqwest::Response,
    provider: Source,
) -> Result<T, SourceError> {
    let status = response.status();
    if !status.is_success() {
        return Err(SourceError::HttpStatus { provider, status });
    }
    response.json().await.map_err(|source| SourceError::Decode {
        provider,
        source: source.without_url(),
    })
}

pub(super) async fn required<T: DeserializeOwned>(
    request: RequestBuilder,
    provider: Source,
) -> Result<T, SourceError> {
    decode(response(request, provider).await?, provider).await
}

pub(super) async fn optional<T: DeserializeOwned>(
    request: RequestBuilder,
    provider: Source,
) -> Result<Option<T>, SourceError> {
    let response = response(request, provider).await?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    decode(response, provider).await.map(Some)
}

pub(super) fn nonempty(
    value: &str,
    provider: Source,
    field: &'static str,
) -> Result<(), SourceError> {
    if value.trim().is_empty() {
        return Err(SourceError::InvalidPayload { provider, field });
    }
    Ok(())
}
