use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ModLoaderType {
    #[default]
    Vanilla,
    Fabric,
    Quilt,
    NeoForge,
    Forge,
    Horizon,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstanceConfig {
    pub id: String,
    pub name: String,
    pub game_version: String,
    pub loader: ModLoaderType,
    pub loader_version: Option<String>,
    pub java_path: Option<String>,
    pub memory_min_mb: Option<u32>,
    pub memory_max_mb: Option<u32>,
    pub icon: Option<String>,
    pub created_at: u64,
    pub last_played: Option<u64>,
    pub total_play_time_seconds: u64,
    pub jvm_args: Option<Vec<String>>,
    #[serde(default)]
    pub fullscreen: Option<bool>,
    #[serde(default)]
    pub window_width: Option<u32>,
    #[serde(default)]
    pub window_height: Option<u32>,
    #[serde(default)]
    pub sync_options: Option<bool>,
    #[serde(default)]
    pub sync_servers: Option<bool>,
    #[serde(default)]
    pub sync_resource_packs: Option<bool>,
    #[serde(default)]
    pub sync_command_history: Option<bool>,
    #[serde(default)]
    pub sync_creative_hotbars: Option<bool>,
    #[serde(default)]
    pub last_synced_at: Option<u64>,
}
