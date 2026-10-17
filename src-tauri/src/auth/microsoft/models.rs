/// What the user needs to finish signing in on microsoft.com/link
#[taurpc::ipc_type]
#[serde(rename_all = "camelCase")]
pub struct MicrosoftDeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    pub device_code: String,
    /// Seconds between polls
    pub interval: u32,
    /// Seconds until the code expires
    pub expires_in: u32,
}

/// A signed-in Minecraft session, ready to launch the game with
#[derive(Clone)]
pub struct MinecraftSession {
    pub access_token: String,
    /// Unix seconds
    pub expires_at: u64,
    pub refresh_token: String,
    pub uuid: String,
    pub username: String,
    pub skin_url: Option<String>,
}

impl std::fmt::Debug for MicrosoftDeviceCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MicrosoftDeviceCode")
            .field("interval", &self.interval)
            .field("expires_in", &self.expires_in)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for MinecraftSession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MinecraftSession")
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authentication_debug_excludes_tokens_and_sign_in_codes() {
        let code = MicrosoftDeviceCode {
            user_code: "private-user-code".into(),
            device_code: "private-device-code".into(),
            verification_uri: "https://example.test/?secret=private-uri".into(),
            interval: 5,
            expires_in: 900,
        };
        let session = MinecraftSession {
            access_token: "private-access".into(),
            refresh_token: "private-refresh".into(),
            expires_at: 123,
            uuid: "uuid".into(),
            username: "Player".into(),
            skin_url: None,
        };
        let diagnostic = format!("{code:?} {session:?}");
        assert!(!diagnostic.contains("private-"));
        assert!(diagnostic.contains("123"));
    }
}
