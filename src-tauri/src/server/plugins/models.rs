use crate::server::config::ServerCoreType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum PluginSource {
    Modrinth,
    Hangar,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginProject {
    pub id: String,
    pub source: PluginSource,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub icon_url: Option<String>,
    pub downloads: u32,
    pub categories: Vec<String>,
    pub page_url: String,
    /// A mod players must install too to join (it adds content their game must know)
    pub players_need_it: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginSearchResult {
    pub items: Vec<PluginProject>,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginDependency {
    /// Modrinth project id or Hangar project slug
    pub project_id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginVersion {
    pub id: String,
    pub version_number: String,
    /// "release" | "beta" | "alpha"
    pub channel: String,
    pub game_versions: Vec<String>,
    pub date: String,
    pub downloads: u32,
    pub file_name: String,
    /// None when the author only links to an external site
    pub download_url: Option<String>,
    pub external_url: Option<String>,
    pub size: u32,
    pub dependencies: Vec<PluginDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPlugin {
    pub file_name: String,
    pub enabled: bool,
    pub size: u32,
    /// From plugin.yml / fabric.mod.json, else the file name
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Vec<String>,
    /// Set when installed through Ingot
    pub source: Option<PluginSource>,
    pub project_id: Option<String>,
    pub icon_url: Option<String>,
    /// Ingot's own companion plugin (shown separately, never auto-updated)
    pub system: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginUpdate {
    pub file_name: String,
    pub current_version: Option<String>,
    pub latest: PluginVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstallReport {
    /// Titles of everything installed, dependencies included
    pub installed: Vec<String>,
    pub warnings: Vec<String>,
}

/// What Ingot remembers about jars it installed (for icons and update checks)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedPlugin {
    pub file_name: String,
    pub source: PluginSource,
    pub project_id: String,
    pub version_id: String,
    pub version_number: String,
    pub title: String,
    pub icon_url: Option<String>,
}

/// What this server core can load
pub struct Platform {
    /// "plugins" or "mods"
    pub folder: &'static str,
    pub modrinth_type: &'static str,
    pub(crate) modrinth_loaders: &'static [&'static str],
    pub hangar: bool,
}

pub fn platform(core: &ServerCoreType) -> Option<Platform> {
    let plugins = |loaders: &'static [&'static str], hangar: bool| Platform {
        folder: "plugins",
        modrinth_type: "plugin",
        modrinth_loaders: loaders,
        hangar,
    };
    match core {
        ServerCoreType::Paper => Some(plugins(&["paper", "spigot", "bukkit"], true)),
        ServerCoreType::Purpur => Some(plugins(&["purpur", "paper", "spigot", "bukkit"], true)),
        ServerCoreType::Folia => Some(plugins(&["folia"], false)),
        ServerCoreType::Fabric => Some(Platform {
            folder: "mods",
            modrinth_type: "mod",
            modrinth_loaders: &["fabric"],
            hangar: false,
        }),
        ServerCoreType::NeoForge => Some(Platform {
            folder: "mods",
            modrinth_type: "mod",
            modrinth_loaders: &["neoforge"],
            hangar: false,
        }),
        ServerCoreType::Forge => Some(Platform {
            folder: "mods",
            modrinth_type: "mod",
            modrinth_loaders: &["forge"],
            hangar: false,
        }),
        ServerCoreType::Quilt => Some(Platform {
            folder: "mods",
            modrinth_type: "mod",
            modrinth_loaders: &["quilt", "fabric"],
            hangar: false,
        }),
        ServerCoreType::Vanilla | ServerCoreType::Pumpkin | ServerCoreType::Bedrock => None,
    }
}

/// Type alias for raw jar metadata tuple to avoid complex type clippy warnings
pub type RawJarMetadata = (String, Option<String>, Option<String>, Vec<String>);

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("This server type doesn't support plugins or mods")]
    UnsupportedCore,
    #[error("Not a plugin file: {0}")]
    InvalidJarName(String),
    #[error("Failed to create HTTP client: {0}")]
    HttpClient(String),
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),
    #[error("{0} returned HTTP {1}")]
    HttpStatus(String, reqwest::StatusCode),
    #[error("Invalid response from {0}: {1}")]
    InvalidResponse(String, String),
    #[error("Download failed: {0}")]
    Download(String),
    #[error("{title} has no downloadable version for Minecraft {game_version}")]
    NoCompatibleVersion { title: String, game_version: String },
    #[error("{0} must be downloaded from its website")]
    ManualDownloadRequired(String),
    #[error("Publication failed: {primary}. Rollback failed: {rollback}. Previous version is retained at {backup}")]
    PublicationRollbackFailed {
        primary: String,
        rollback: String,
        backup: String,
    },
    #[error("Publication failed: {primary}. Previous version was restored.")]
    PublicationFailed {
        primary: String,
    },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

impl From<PluginError> for String {
    fn from(e: PluginError) -> Self {
        e.to_string()
    }
}
