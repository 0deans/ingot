use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct HorizonRelease {
    pub tag: String,
    pub name: String,
    pub published_at: Option<String>,
    pub channel: HorizonChannel,
    pub target_game_version: String,
    pub asset_url: Option<String>,
    pub asset_name: Option<String>,
    pub asset_size: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HorizonChannel {
    Nightly,
    Stable,
}

#[derive(Debug, thiserror::Error)]
pub enum HorizonError {
    #[error("Failed to query Horizon releases: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("I/O error during Horizon operation: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("No compatible Horizon asset found for tag {0}")]
    AssetNotFound(String),
    #[error("Failed to extract Horizon engine archive: {0}")]
    Extraction(String),
    #[error("Horizon client executable not found in extracted directory: {0}")]
    ExecutableNotFound(String),
    #[error("Horizon session configuration error: {0}")]
    Session(String),
    #[error("Horizon resource preparation failed: {0}")]
    Preparation(String),
}
