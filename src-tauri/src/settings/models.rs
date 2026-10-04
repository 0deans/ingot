use serde::{Deserialize, Serialize};

use super::SettingsError;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LauncherBehavior {
    #[default]
    KeepOpen,
    HideToTray,
    Close,
}

impl LauncherBehavior {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::KeepOpen => "keepOpen",
            Self::HideToTray => "hideToTray",
            Self::Close => "close",
        }
    }
}

impl TryFrom<&str> for LauncherBehavior {
    type Error = SettingsError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "keepOpen" => Ok(Self::KeepOpen),
            "hideToTray" => Ok(Self::HideToTray),
            "close" => Ok(Self::Close),
            _ => Err(SettingsError::InvalidBehavior(value.to_owned())),
        }
    }
}

#[taurpc::ipc_type]
#[derive(Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemorySettings {
    pub min_ram_mb: u32,
    pub max_ram_mb: u32,
}

impl MemorySettings {
    /// Preserve the existing setter's normalization of reversed limits.
    pub(crate) fn normalized(min_mb: u32, max_mb: u32) -> Result<Self, SettingsError> {
        let settings = Self {
            min_ram_mb: min_mb.min(max_mb),
            max_ram_mb: min_mb.max(max_mb),
        };
        settings.validate()?;
        Ok(settings)
    }

    pub(crate) fn validate(&self) -> Result<(), SettingsError> {
        if self.min_ram_mb == 0 || self.min_ram_mb > self.max_ram_mb {
            return Err(SettingsError::InvalidMemory {
                min_mb: self.min_ram_mb,
                max_mb: self.max_ram_mb,
            });
        }
        Ok(())
    }
}

impl Default for MemorySettings {
    fn default() -> Self {
        Self {
            min_ram_mb: 2048,
            max_ram_mb: 4096,
        }
    }
}

#[taurpc::ipc_type]
#[derive(Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WindowSettings {
    pub fullscreen: bool,
    pub width: u32,
    pub height: u32,
}

impl WindowSettings {
    pub(crate) fn validate(&self) -> Result<(), SettingsError> {
        if self.width == 0 || self.height == 0 {
            return Err(SettingsError::InvalidWindow {
                width: self.width,
                height: self.height,
            });
        }
        Ok(())
    }
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            fullscreen: false,
            width: 854,
            height: 480,
        }
    }
}

#[taurpc::ipc_type]
#[derive(Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncSettings {
    pub sync_options: bool,
    pub sync_servers: bool,
    pub sync_resource_packs: bool,
    pub sync_command_history: bool,
    pub sync_creative_hotbars: bool,
    #[serde(default)]
    pub initialized_categories: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherSettings {
    pub memory: MemorySettings,
    #[serde(default)]
    pub launcher_behavior: LauncherBehavior,
    #[serde(default)]
    pub window: WindowSettings,
    #[serde(default)]
    pub sync: SyncSettings,
}

impl LauncherSettings {
    pub(crate) fn validate(&self) -> Result<(), SettingsError> {
        self.memory.validate()?;
        self.window.validate()
    }
}
