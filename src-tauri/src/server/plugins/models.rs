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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    HttpClient(#[source] reqwest::Error),
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[source] reqwest::Error),
    #[error("Provider returned HTTP {0}")]
    HttpStatus(reqwest::StatusCode),
    #[error("Invalid provider response: {0}")]
    InvalidResponse(#[source] reqwest::Error),
    #[error("Provider response missing or invalid field: {0}")]
    InvalidPayload(&'static str),
    #[error("{title} has no downloadable version for Minecraft {game_version}")]
    NoCompatibleVersion { title: String, game_version: String },
    #[error("{0} must be downloaded from its website")]
    ManualDownloadRequired(String),
    #[error("Downloaded plugin is empty")]
    EmptyDownload,
    #[error("Publication failed: {primary}. Rollback failed: {rollback:?}. Previous version is retained at {backup:?}")]
    PublicationRollbackFailed {
        #[source]
        primary: Box<PluginError>,
        rollback: Vec<std::io::Error>,
        backup: Option<std::path::PathBuf>,
    },
    #[error("Publication failed: {primary}. Previous version was restored.")]
    PublicationFailed {
        #[source]
        primary: Box<PluginError>,
    },
    #[error("Plugin was published but obsolete-file cleanup failed at {path}: {source}")]
    Cleanup {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Plugin tracking error: {0}")]
    Tracking(#[from] super::tracking::TrackingError),
    #[error("Invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Plugin recovery is blocked: {0}")]
    RecoveryBlocked(&'static str),
    #[error("Plugin blocking task failed: {0}")]
    Task(#[from] tokio::task::JoinError),
}
