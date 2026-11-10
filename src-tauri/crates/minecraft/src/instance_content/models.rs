use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ContentCategory {
    Mod,
    ResourcePack,
    ShaderPack,
    DataPack,
}

impl ContentCategory {
    pub fn default_subfolder(self) -> &'static str {
        match self {
            Self::Mod => "mods",
            Self::ResourcePack => "resourcepacks",
            Self::ShaderPack => "shaderpacks",
            Self::DataPack => "datapacks",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ContentState {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContentDependency {
    pub id: String,
    pub name: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContentMetadata {
    pub name: String,
    pub mod_id: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub icon_data_url: Option<String>,
    pub dependencies: Vec<ContentDependency>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ProviderSource {
    Modrinth,
    CurseForge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TrackedOrigin {
    pub source: ProviderSource,
    pub project_id: String,
    pub project_slug: Option<String>,
    pub version_id: String,
    pub version_number: String,
    pub title: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ContentOrigin {
    Tracked(TrackedOrigin),
    Identified {
        source: ProviderSource,
        project_id: String,
        project_slug: Option<String>,
        version_id: String,
        version_number: String,
        title: String,
        icon_url: Option<String>,
    },
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstalledContent {
    pub file_name: String,
    pub category: ContentCategory,
    pub state: ContentState,
    pub size_bytes: u64,
    pub modified_at: u64,
    pub metadata: ContentMetadata,
    pub origin: ContentOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContentDependencyIssue {
    pub mod_id: String,
    pub mod_name: String,
    pub missing_dependency_id: String,
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContentListResult {
    pub items: Vec<InstalledContent>,
    pub dependency_issues: Vec<ContentDependencyIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContentUpdateCandidate {
    pub file_name: String,
    pub current_version: Option<String>,
    pub latest_version: String,
    pub latest_version_id: String,
    pub title: String,
    pub download_url: String,
    pub new_file_name: String,
    pub channel: String,
    pub source: ProviderSource,
    pub sha1: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub updates: Vec<ContentUpdateCandidate>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContentApplyReport {
    pub updated: Vec<String>,
    pub failed: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct IdentifyResult {
    pub identified_count: u32,
    pub total_count: u32,
    pub warnings: Vec<String>,
}
