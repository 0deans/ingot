//! Raw Technic Platform and Solder API payload deserialization models.
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub(super) enum TechnicImage {
    Obj { url: Option<String> },
    Str(String),
}

impl TechnicImage {
    pub(super) fn to_url(&self) -> Option<String> {
        match self {
            Self::Obj { url } => url.clone(),
            Self::Str(s) => Some(s.clone()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct TechnicSearchResponse {
    pub(super) modpacks: Vec<TechnicSearchEntry>,
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct TechnicSearchEntry {
    pub(super) name: String,
    pub(super) slug: Option<String>,
    #[serde(alias = "displayName")]
    pub(super) display_name: Option<String>,
    pub(super) user: Option<String>,
    pub(super) description: Option<String>,
    #[serde(alias = "iconUrl")]
    pub(super) icon_url: Option<String>,
    pub(super) icon: Option<TechnicImage>,
    pub(super) logo: Option<TechnicImage>,
    #[serde(alias = "installs")]
    pub(super) downloads: Option<u64>,
    pub(super) runs: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct TechnicPackDetail {
    pub(super) name: String,
    #[serde(alias = "displayName")]
    pub(super) display_name: Option<String>,
    #[serde(alias = "platformUrl")]
    pub(super) platform_url: Option<String>,
    pub(super) user: Option<String>,
    pub(super) description: Option<String>,
    pub(super) overview: Option<String>,
    pub(super) url: Option<String>,
    pub(super) solder: Option<String>,
    pub(super) version: Option<String>,
    pub(super) minecraft: Option<String>,
    pub(super) icon: Option<TechnicImage>,
    pub(super) logo: Option<TechnicImage>,
    pub(super) background: Option<TechnicImage>,
    #[serde(alias = "installs")]
    pub(super) downloads: Option<u64>,
    pub(super) runs: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(super) struct SolderPackResponse {
    pub(super) recommended: Option<String>,
    pub(super) builds: Vec<String>,
}
