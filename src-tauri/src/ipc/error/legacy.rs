use super::ApiError;

impl From<String> for ApiError {
    fn from(message: String) -> Self {
        if message.starts_with("Failed to load accounts at")
            || message.starts_with("Failed to parse accounts at")
            || message.starts_with("Failed to write accounts at")
            || message.starts_with("Failed to store upload")
            || message.starts_with("Corrupt settings at")
            || message.contains("Stored account credentials are invalid")
        {
            Self::storage_failure(message)
        } else if message == "Account not found"
            || message.starts_with("Server not found")
            || message.starts_with("Instance not found")
            || message == "No credentials found in OS Keyring"
            || message.contains("not found")
            || message.contains("Not found")
        {
            Self::not_found(message)
        } else if message.contains("already in use")
            || message.contains("still running")
            || message.contains("Stop the server")
            || message.contains("Stop the instance")
        {
            Self::conflict(message)
        } else if message.starts_with("Invalid")
            || message.contains("cannot be empty")
            || message.contains("is empty")
            || message.contains("not a valid")
            || message.contains("must be positive")
            || message.contains("is invalid")
            || message.contains("ticket is invalid")
            || message.contains("Pending Microsoft sign-in expired")
            || message.contains("Pending Microsoft sign-in record is invalid")
            || message.contains("This older Microsoft sign-in cannot be resumed")
        {
            Self::invalid_input(message)
        } else if message.contains("Cannot read account credentials")
            || message.contains("Cannot write account credentials")
            || message.contains("Cannot delete account credentials")
        {
            Self::provider_unavailable(message)
        } else {
            Self::internal(message)
        }
    }
}

impl From<&str> for ApiError {
    fn from(message: &str) -> Self {
        Self::from(message.to_string())
    }
}
