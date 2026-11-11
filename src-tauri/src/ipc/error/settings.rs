use super::ApiError;

impl From<crate::app::platform::OpenDirectoryError> for ApiError {
    fn from(error: crate::app::platform::OpenDirectoryError) -> Self {
        use crate::app::platform::OpenDirectoryError;
        match error {
            OpenDirectoryError::Target(source) if source.kind() == std::io::ErrorKind::NotFound => {
                Self::not_found(source.to_string())
            }
            OpenDirectoryError::Target(source) => Self::storage_failure(source.to_string()),
            OpenDirectoryError::Spawn(_) | OpenDirectoryError::Unsupported => {
                Self::internal(error.to_string())
            }
        }
    }
}

impl From<ingot_settings::SettingsError> for ApiError {
    fn from(error: ingot_settings::SettingsError) -> Self {
        match error {
            ingot_settings::SettingsError::InvalidMemory { .. }
            | ingot_settings::SettingsError::InvalidWindow { .. }
            | ingot_settings::SettingsError::InvalidBehavior(_) => {
                Self::invalid_input(error.to_string())
            }
            ingot_settings::SettingsError::Decode { .. }
            | ingot_settings::SettingsError::Encode(_)
            | ingot_settings::SettingsError::Io { .. } => Self::storage_failure(error.to_string()),
            ingot_settings::SettingsError::LockPoisoned
            | ingot_settings::SettingsError::StateUnavailable => Self::internal(error.to_string()),
        }
    }
}
