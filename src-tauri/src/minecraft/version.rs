use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const MOJANG_MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct VersionManifestEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub sha1: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct VersionManifestLatest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct VersionManifest {
    pub latest: VersionManifestLatest,
    pub versions: Vec<VersionManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadArtifact {
    pub path: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDownloads {
    pub client: Option<DownloadArtifact>,
    pub server: Option<DownloadArtifact>,
}

pub use crate::minecraft::launcher::rules::{OsRule, RuleEntry};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<DownloadArtifact>,
    pub classifiers: Option<HashMap<String, DownloadArtifact>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryEntry {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    pub rules: Option<Vec<RuleEntry>>,
    pub natives: Option<HashMap<String, String>>,
    pub url: Option<String>, // Custom maven base url if downloads is not present
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndexRef {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    #[serde(rename = "totalSize")]
    pub total_size: Option<u64>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaVersionInfo {
    pub component: Option<String>,
    #[serde(rename = "majorVersion")]
    pub major_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    Simple(String),
    Structured {
        rules: Option<Vec<RuleEntry>>,
        value: serde_json::Value, // Can be string or array of strings
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VersionArguments {
    pub game: Option<Vec<ArgumentValue>>,
    pub jvm: Option<Vec<ArgumentValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionPackage {
    pub id: String,
    pub downloads: Option<VersionDownloads>,
    pub libraries: Vec<LibraryEntry>,
    #[serde(rename = "assetIndex")]
    pub asset_index: Option<AssetIndexRef>,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub arguments: Option<VersionArguments>,
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
    #[serde(rename = "javaVersion")]
    pub java_version: Option<JavaVersionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
}

pub use crate::minecraft::launcher::classpath::maven_coordinate_to_path as maven_to_path;
pub use crate::minecraft::launcher::rules::check_rules;

pub fn should_include_library(lib: &LibraryEntry) -> bool {
    crate::minecraft::launcher::rules::should_include_library_rules(lib.rules.as_deref())
}

/// Fetches the Mojang version manifest, caching it locally for 1 hour
pub async fn fetch_version_manifest(
    client: &reqwest::Client,
    cache_dir: &Path,
) -> Result<VersionManifest, String> {
    let manifest_file = cache_dir.join("version_manifest_v2.json");

    if manifest_file.exists() {
        if let Ok(metadata) = fs::metadata(&manifest_file) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    if elapsed.as_secs() < 3600 {
                        if let Ok(data) = fs::read_to_string(&manifest_file) {
                            if let Ok(manifest) = serde_json::from_str::<VersionManifest>(&data) {
                                return Ok(manifest);
                            }
                        }
                    }
                }
            }
        }
    }

    let manifest = client
        .get(MOJANG_MANIFEST_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch version manifest: {e}"))?
        .json::<VersionManifest>()
        .await
        .map_err(|e| format!("Failed to parse version manifest: {e}"))?;

    let _ = fs::create_dir_all(cache_dir);
    if let Ok(data) = serde_json::to_string(&manifest) {
        let _ = fs::write(&manifest_file, data);
    }

    Ok(manifest)
}

/// Fetches the detailed VersionPackage for a specific game version
pub async fn fetch_version_package(
    client: &reqwest::Client,
    cache_dir: &Path,
    game_version: &str,
) -> Result<VersionPackage, String> {
    let versions_dir = cache_dir.join("versions").join(game_version);
    let pkg_file = versions_dir.join(format!("{}.json", game_version));

    if pkg_file.exists() {
        if let Ok(data) = fs::read_to_string(&pkg_file) {
            if let Ok(pkg) = serde_json::from_str::<VersionPackage>(&data) {
                return Ok(pkg);
            }
        }
    }

    let manifest = fetch_version_manifest(client, cache_dir).await?;
    let entry = manifest
        .versions
        .iter()
        .find(|v| v.id == game_version)
        .ok_or_else(|| format!("Minecraft version '{}' not found in manifest", game_version))?;

    let pkg = client
        .get(&entry.url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch version package for {}: {e}", game_version))?
        .json::<VersionPackage>()
        .await
        .map_err(|e| format!("Failed to parse version package for {}: {e}", game_version))?;

    let _ = fs::create_dir_all(&versions_dir);
    if let Ok(data) = serde_json::to_string(&pkg) {
        let _ = fs::write(&pkg_file, data);
    }

    Ok(pkg)
}

/// Fetches the asset index for a version package
pub async fn fetch_asset_index(
    client: &reqwest::Client,
    assets_dir: &Path,
    asset_ref: &AssetIndexRef,
) -> Result<AssetIndex, String> {
    let indexes_dir = assets_dir.join("indexes");
    let index_file = indexes_dir.join(format!("{}.json", asset_ref.id));

    if index_file.exists() {
        if let Ok(data) = fs::read_to_string(&index_file) {
            if let Ok(index) = serde_json::from_str::<AssetIndex>(&data) {
                return Ok(index);
            }
        }
    }

    let index = client
        .get(&asset_ref.url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch asset index {}: {e}", asset_ref.id))?
        .json::<AssetIndex>()
        .await
        .map_err(|e| format!("Failed to parse asset index {}: {e}", asset_ref.id))?;

    let _ = fs::create_dir_all(&indexes_dir);
    if let Ok(data) = serde_json::to_string(&index) {
        let _ = fs::write(&index_file, data);
    }

    Ok(index)
}
