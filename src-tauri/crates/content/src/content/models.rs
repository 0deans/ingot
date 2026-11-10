//! Data models and DTOs for unified content searching, versioning, and details.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedContentItem {
    pub id: String,
    pub source: String,       // "modrinth" | "curseforge"
    pub project_type: String, // "modpack" | "mod" | "resourcepack" | "shader"
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub follows: Option<u64>,
    pub categories: Vec<String>,
    pub latest_version: Option<String>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub website_url: Option<String>,
    pub date_modified: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContentSearchResult {
    pub items: Vec<UnifiedContentItem>,
    pub total_hits: u64,
    pub offset: u32,
    pub limit: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContentScreenshot {
    pub url: String,
    pub title: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedContentVersion {
    pub id: String,
    pub name: String,
    pub version_number: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub version_type: String, // "release" | "beta" | "alpha"
    pub date_published: String,
    pub downloads: u64,
    pub filename: String,
    pub download_url: String,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedContentDetails {
    pub id: String,
    pub source: String,
    pub project_type: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub body: String,
    pub author: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub follows: Option<u64>,
    pub categories: Vec<String>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub website_url: Option<String>,
    pub issues_url: Option<String>,
    pub source_url: Option<String>,
    pub wiki_url: Option<String>,
    pub screenshots: Vec<ContentScreenshot>,
    pub versions: Vec<UnifiedContentVersion>,
}

/// Request parameters for searching content across providers.
#[derive(Debug, Clone)]
pub struct ContentSearchQuery<'a> {
    pub project_type: &'a str,
    pub query: Option<&'a str>,
    pub game_version: Option<&'a str>,
    pub loader: Option<&'a str>,
    pub sort: Option<&'a str>,
    pub offset: u32,
    pub limit: u32,
}
