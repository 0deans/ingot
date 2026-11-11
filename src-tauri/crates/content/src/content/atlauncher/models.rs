//! Raw ATLauncher API payload deserialization models.
use crate::content::serde_helpers::deserialize_opt_u64_or_string;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub(super) enum AtLauncherPackListResponse {
    Data { data: Vec<AtLauncherPackSummary> },
    List(Vec<AtLauncherPackSummary>),
}

impl AtLauncherPackListResponse {
    pub(super) fn into_vec(self) -> Vec<AtLauncherPackSummary> {
        match self {
            Self::Data { data } => data,
            Self::List(list) => list,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct AtLauncherPackSummary {
    #[serde(default, deserialize_with = "deserialize_opt_u64_or_string")]
    pub(super) id: Option<u64>,
    pub(super) position: Option<u64>,
    pub(super) name: String,
    #[serde(rename = "type")]
    pub(super) pack_type: Option<String>,
    #[serde(rename = "safeName")]
    pub(super) safe_name: Option<String>,
    pub(super) description: Option<String>,
    pub(super) system: Option<String>,
    pub(super) versions: Option<Vec<AtLauncherVersionRef>>,
    pub(super) downloads: Option<u64>,
    #[serde(rename = "supportURL")]
    pub(super) support_url: Option<String>,
    #[serde(rename = "websiteURL")]
    pub(super) website_url: Option<String>,
    #[serde(default)]
    pub(super) categories: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct AtLauncherVersionRef {
    pub(super) version: String,
    pub(super) minecraft: Option<String>,
    pub(super) published: Option<u64>,
    #[serde(rename = "type")]
    pub(super) version_type: Option<String>,
    #[serde(rename = "hasLoader")]
    pub(super) has_loader: Option<bool>,
    #[serde(rename = "downloadUrl")]
    pub(super) download_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub(super) enum AtLauncherPackDetailResponse {
    Data { data: AtLauncherPackDetail },
    Direct(AtLauncherPackDetail),
}

impl AtLauncherPackDetailResponse {
    pub(super) fn into_detail(self) -> AtLauncherPackDetail {
        match self {
            Self::Data { data } => data,
            Self::Direct(detail) => detail,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct AtLauncherPackDetail {
    pub(super) name: String,
    #[serde(rename = "safeName")]
    pub(super) safe_name: Option<String>,
    pub(super) description: Option<String>,
    pub(super) body: Option<String>,
    pub(super) versions: Option<Vec<AtLauncherDetailVersion>>,
    pub(super) downloads: Option<u64>,
    #[serde(default)]
    pub(super) categories: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct AtLauncherDetailVersion {
    pub(super) version: String,
    pub(super) minecraft: Option<String>,
    pub(super) loader: Option<String>,
    pub(super) published: Option<u64>,
    #[serde(rename = "type")]
    pub(super) version_type: Option<String>,
    #[serde(rename = "downloadUrl")]
    pub(super) download_url: Option<String>,
}
