use super::ApiError;

impl From<crate::app::platform::OpenDirectoryError> for ApiError {
    fn from(error: crate::app::platform::OpenDirectoryError) -> Self {
        Self::internal(error.to_string())
    }
}

impl From<crate::settings::SettingsError> for ApiError {
    fn from(error: crate::settings::SettingsError) -> Self {
        match error {
            crate::settings::SettingsError::InvalidMemory { .. }
            | crate::settings::SettingsError::InvalidWindow { .. }
            | crate::settings::SettingsError::InvalidBehavior(_) => {
                Self::invalid_input(error.to_string())
            }
            crate::settings::SettingsError::Decode { .. }
            | crate::settings::SettingsError::Encode(_)
            | crate::settings::SettingsError::Io { .. } => Self::storage_failure(error.to_string()),
            crate::settings::SettingsError::LockPoisoned
            | crate::settings::SettingsError::StateUnavailable => Self::internal(error.to_string()),
        }
    }
}
