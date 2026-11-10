use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ExportFormat {
    Modrinth,
    CurseForge,
    Prism,
}

impl ExportFormat {
    pub fn file_extension(self) -> &'static str {
        match self {
            Self::Modrinth => "mrpack",
            Self::CurseForge => "zip",
            Self::Prism => "zip",
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum EnvSupport {
    #[default]
    Required,
    Optional,
    Unsupported,
}

impl EnvSupport {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::Optional => "optional",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FileSide {
    pub path: String,
    pub client: EnvSupport,
    pub server: EnvSupport,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub summary: Option<String>,
    pub included_paths: Vec<String>,
    pub file_sides: Vec<FileSide>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExportPreviewEntry {
    pub path: String,
    pub size_bytes: u64,
    pub default_included: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExportPreview {
    pub default_name: String,
    pub default_version: String,
    pub entries: Vec<ExportPreviewEntry>,
    pub total_size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    pub output_path: String,
    pub total_files: usize,
    pub resolved_files: usize,
    pub embedded_files: usize,
    pub embedded_warnings: Vec<String>,
    pub warnings: Vec<String>,
}
