/// What the user needs to finish signing in on microsoft.com/link
#[taurpc::ipc_type]
#[derive(Debug)]
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
#[derive(Debug, Clone)]
pub struct MinecraftSession {
    pub access_token: String,
    /// Unix seconds
    pub expires_at: u64,
    pub refresh_token: String,
    pub uuid: String,
    pub username: String,
    pub skin_url: Option<String>,
}
