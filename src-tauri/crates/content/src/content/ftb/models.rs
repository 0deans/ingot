//! Raw FTB API payload deserialization models.
use crate::content::serde_helpers::{deserialize_opt_u64_or_string, deserialize_u64_or_string};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(transparent)]
pub(super) struct FtbPackId(
    #[serde(deserialize_with = "deserialize_u64_or_string")] pub(super) u64,
);

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum FtbPackListResponse {
    Object { packs: Vec<FtbPackId> },
    Array(Vec<FtbPackId>),
}

impl FtbPackListResponse {
    pub(super) fn into_ids(self) -> Vec<u64> {
        match self {
            Self::Object { packs } => packs.into_iter().map(|p| p.0).collect(),
            Self::Array(list) => list.into_iter().map(|p| p.0).collect(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct FtbArtItem {
    #[serde(default)]
    pub(super) r#type: String,
    pub(super) url: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub(super) enum FtbAuthorItem {
    Obj { name: String },
    Str(String),
}

impl FtbAuthorItem {
    pub(super) fn name(&self) -> &str {
        match self {
            Self::Obj { name } => name.as_str(),
            Self::Str(name) => name.as_str(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub(super) enum FtbTagItem {
    Obj { name: String },
    Str(String),
}

impl FtbTagItem {
    pub(super) fn name(&self) -> &str {
        match self {
            Self::Obj { name } => name.as_str(),
            Self::Str(name) => name.as_str(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct FtbTarget {
    #[serde(default)]
    pub(super) name: String,
    #[serde(default)]
    pub(super) r#type: String,
    pub(super) version: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct FtbVersionSummary {
    #[serde(deserialize_with = "deserialize_u64_or_string")]
    pub(super) id: u64,
    pub(super) name: String,
    pub(super) r#type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_u64_or_string")]
    pub(super) updated: Option<u64>,
    #[serde(default)]
    pub(super) targets: Vec<FtbTarget>,
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct FtbPackDetail {
    #[serde(deserialize_with = "deserialize_u64_or_string")]
    pub(super) id: u64,
    pub(super) name: String,
    pub(super) slug: Option<String>,
    pub(super) synopsis: Option<String>,
    pub(super) description: Option<String>,
    #[serde(default)]
    pub(super) art: Vec<FtbArtItem>,
    #[serde(default)]
    pub(super) authors: Vec<FtbAuthorItem>,
    #[serde(default)]
    pub(super) tags: Vec<FtbTagItem>,
    #[serde(default)]
    pub(super) targets: Vec<FtbTarget>,
    #[serde(default)]
    pub(super) versions: Vec<FtbVersionSummary>,
    #[serde(default, deserialize_with = "deserialize_opt_u64_or_string")]
    pub(super) installs: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_opt_u64_or_string")]
    pub(super) plays: Option<u64>,
}
