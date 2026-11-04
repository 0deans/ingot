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

impl ModLoaderType {
    pub fn supports_mods(&self) -> bool {
        matches!(
            self,
            ModLoaderType::Fabric
                | ModLoaderType::Quilt
                | ModLoaderType::NeoForge
                | ModLoaderType::Forge
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InstanceColorTag {
    Red,
    Orange,
    Amber,
    Emerald,
    Cyan,
    Blue,
    Violet,
    Pink,
    Rose,
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
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub color_tag: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mod_loader_supports_mods() {
        assert!(!ModLoaderType::Vanilla.supports_mods());
        assert!(!ModLoaderType::Horizon.supports_mods());
        assert!(ModLoaderType::Fabric.supports_mods());
        assert!(ModLoaderType::Quilt.supports_mods());
        assert!(ModLoaderType::NeoForge.supports_mods());
        assert!(ModLoaderType::Forge.supports_mods());
    }
}
