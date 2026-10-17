//! Serializable API error union types for TauRPC result envelopes.
use serde::{Deserialize, Serialize};
use specta::Type;
use thiserror::Error;

/// Standardized, typed error envelope returned across all TauRPC APIs.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Error, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ApiError {
    #[error("Invalid input: {message}")]
    InvalidInput { message: String },

    #[error("Not found: {message}")]
    NotFound { message: String },

    #[error("Conflict: {message}")]
    Conflict { message: String },

    #[error("Authentication required: {message}")]
    Unauthorized { message: String },

    #[error("Operation cancelled: {message}")]
    Cancelled { message: String },

    #[error("Provider unavailable: {message}")]
    ProviderUnavailable { message: String },

    #[error("Storage failure: {message}")]
    StorageFailure { message: String },

    #[error("Incomplete recovery: {message}")]
    IncompleteRecovery { message: String },

    #[error("Internal failure: {message}")]
    Internal { message: String },
}

impl ApiError {
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput {
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            message: message.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Unauthorized {
            message: message.into(),
        }
    }

    pub fn cancelled(message: impl Into<String>) -> Self {
        Self::Cancelled {
            message: message.into(),
        }
    }

    pub fn provider_unavailable(message: impl Into<String>) -> Self {
        Self::ProviderUnavailable {
            message: message.into(),
        }
    }

    pub fn storage_failure(message: impl Into<String>) -> Self {
        Self::StorageFailure {
            message: message.into(),
        }
    }

    pub fn incomplete_recovery(message: impl Into<String>) -> Self {
        Self::IncompleteRecovery {
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    pub fn from_display(error: impl std::fmt::Display) -> Self {
        Self::internal(error.to_string())
    }
}

mod accounts;
mod instances;
mod servers;
mod settings;

#[cfg(test)]
mod tests;
