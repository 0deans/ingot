use super::ApiError;

impl From<crate::server::config::service::ConfigError> for ApiError {
    fn from(error: crate::server::config::service::ConfigError) -> Self {
        use crate::server::config::service::ConfigError;
        match error {
            ConfigError::Metadata(error) => Self::from(error),
            ConfigError::File(error) => Self::from(error),
            ConfigError::InvalidId | ConfigError::InvalidPort => {
                Self::invalid_input(error.to_string())
            }
            ConfigError::FilesCleanupCommitted { .. } => {
                Self::incomplete_recovery(error.to_string())
            }
            ConfigError::Directory(_) | ConfigError::Io(_) => {
                Self::storage_failure(error.to_string())
            }
        }
    }
}

impl From<crate::server::files::ServerFileError> for ApiError {
    fn from(error: crate::server::files::ServerFileError) -> Self {
        use crate::server::files::ServerFileError;
        match error {
            ServerFileError::InvalidProperty(_) | ServerFileError::InvalidPath(_) => {
                Self::invalid_input(error.to_string())
            }
            ServerFileError::MissingConfig(_) => Self::not_found(error.to_string()),
            ServerFileError::Io(_)
            | ServerFileError::Json(_)
            | ServerFileError::InvalidAccessEntry(_) => Self::storage_failure(error.to_string()),
        }
    }
}

impl From<crate::server::plugins::tracking::TrackingError> for ApiError {
    fn from(error: crate::server::plugins::tracking::TrackingError) -> Self {
        Self::storage_failure(error.to_string())
    }
}

impl From<crate::server::process::models::ProcessError> for ApiError {
    fn from(error: crate::server::process::models::ProcessError) -> Self {
        use crate::server::process::models::ProcessError;
        match error {
            ProcessError::NotRunning(_) => Self::not_found(error.to_string()),
            ProcessError::AlreadyRunning
            | ProcessError::AlreadyStarting
            | ProcessError::StillStarting
            | ProcessError::ServerStopped => Self::conflict(error.to_string()),
            ProcessError::JarDownload(_) | ProcessError::NoReply(_) => {
                Self::provider_unavailable(error.to_string())
            }
            ProcessError::Config(_) | ProcessError::StdinWrite(_) | ProcessError::StdinFlush(_) => {
                Self::storage_failure(error.to_string())
            }
            _ => Self::internal(error.to_string()),
        }
    }
}

impl From<crate::server::plugins::PluginError> for ApiError {
    fn from(error: crate::server::plugins::PluginError) -> Self {
        use crate::server::plugins::PluginError;
        match error {
            PluginError::UnsupportedCore
            | PluginError::InvalidJarName(_)
            | PluginError::NoCompatibleVersion { .. }
            | PluginError::ManualDownloadRequired(_) => Self::invalid_input(error.to_string()),
            PluginError::Tracking(error) => Self::from(error),
            PluginError::Io(error) => Self::storage_failure(error.to_string()),
            PluginError::PublicationRollbackFailed { .. } | PluginError::Cleanup { .. } => {
                Self::incomplete_recovery(error.to_string())
            }
            PluginError::PublicationFailed { primary } => Self::from(*primary),
            PluginError::HttpClient(_)
            | PluginError::RequestFailed(_)
            | PluginError::HttpStatus(_)
            | PluginError::InvalidResponse(_)
            | PluginError::InvalidPayload(_)
            | PluginError::EmptyDownload => Self::provider_unavailable(error.to_string()),
            PluginError::Json(_) | PluginError::Task(_) => Self::internal(error.to_string()),
        }
    }
}
