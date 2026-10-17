//! Provider contracts and typed planning inputs. Missing releases are distinct from failed queries.
mod curseforge;
mod fingerprint;
mod hangar;
mod http;
mod modrinth;
#[cfg(test)]
mod tests;

use super::TargetFile;
pub use fingerprint::cf_fingerprint;
pub use http::{SourceClient, SourceError};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    Modrinth,
    Curseforge,
    Hangar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Release,
    Beta,
    Alpha,
}

/// A dependency of a release, on the same source
#[derive(Debug, Clone)]
pub struct Dep {
    /// Required; otherwise it's marked incompatible
    pub required: bool,
    pub project: Option<String>,
    /// Modrinth sometimes names a version instead of a project
    pub version_id: Option<String>,
}

/// One version of a project, reduced to what planning needs
#[derive(Debug, Clone)]
pub struct Release {
    pub source: Source,
    pub project: String,
    pub version_id: String,
    pub version_number: String,
    pub channel: Channel,
    /// None when the author doesn't allow downloads outside their site
    pub file: Option<TargetFile>,
    pub deps: Vec<Dep>,
}

impl Release {
    /// A warning when the only version for the target is a beta or an alpha
    pub fn unstable_note(&self) -> Option<String> {
        match self.channel {
            Channel::Release => None,
            Channel::Beta => Some("Beta version, may be unstable".into()),
            Channel::Alpha => Some("Alpha version, may be unstable".into()),
        }
    }
}

/// Title, icon and web page of a project
#[derive(Debug, Clone, Default)]
pub struct ProjectInfo {
    pub title: String,
    pub icon_url: Option<String>,
    pub page_url: Option<String>,
}

/// Which files a folder may hold on each source, for the target loader
#[derive(Debug, Clone, Default)]
pub struct Scope {
    /// Modrinth loader names ("fabric", "paper", "minecraft", "iris"...)
    pub modrinth: Vec<&'static str>,
    /// CurseForge: search it at all, and the mod loader type for mods
    /// (Forge 1, Fabric 4, Quilt 5, NeoForge 6)
    pub curseforge: bool,
    pub cf_loader: Option<u32>,
    /// CurseForge lists loaders by name among the game versions ("Fabric", "NeoForge")
    pub cf_loader_names: Vec<&'static str>,
}
