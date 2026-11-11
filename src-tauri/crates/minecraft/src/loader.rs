//! Mod-loader version discovery, profile resolution, and local profile caching.

mod parsing;

use crate::instance::ModLoaderType;
use crate::version::LibraryEntry;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub use parsing::normalize_loader_version;
#[cfg(test)]
use parsing::{convert_prism_library, forge_log4j_library, PrismComponent, PrismLibrary};
use parsing::{
    effective_loader_version, parse_meta_loader_versions, parse_meta_profile,
    parse_prism_loader_versions, parse_prism_profile, profile_cache_file_name,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoaderProfile {
    pub id: String,
    pub main_class: String,
    pub libraries: Vec<LibraryEntry>,
    #[serde(default)]
    pub maven_files: Vec<LibraryEntry>,
    pub jvm_args: Vec<String>,
    pub game_args: Vec<String>,
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum LoaderError {
    #[error("Network request failed: {0}")]
    Network(#[from] reqwest::Error),

    #[error("JSON deserialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Vanilla has no loader profile")]
    VanillaNotSupported,
}

#[derive(Debug, Clone)]
struct LoaderEndpoints {
    fabric_meta_base: String,
    quilt_meta_base: String,
    prism_meta_base: String,
}

impl LoaderEndpoints {
    fn production() -> Self {
        Self {
            fabric_meta_base: "https://meta.fabricmc.net/v2/versions/loader".to_string(),
            quilt_meta_base: "https://meta.quiltmc.org/v3/versions/loader".to_string(),
            prism_meta_base: "https://meta.prismlauncher.org/v1".to_string(),
        }
    }

    #[cfg(test)]
    fn from_base(base_url: &str) -> Self {
        let base = base_url.trim_end_matches('/');
        Self {
            fabric_meta_base: format!("{base}/fabric/v2/versions/loader"),
            quilt_meta_base: format!("{base}/quilt/v3/versions/loader"),
            prism_meta_base: format!("{base}/prism/v1"),
        }
    }
}

async fn fetch_meta_loader_versions(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<String>, LoaderError> {
    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        return Ok(Vec::new());
    }
    let bytes = resp.bytes().await?;
    Ok(parse_meta_loader_versions(&bytes)?)
}

async fn fetch_prism_loader_versions(
    client: &reqwest::Client,
    url: &str,
    game_version: &str,
) -> Result<Vec<String>, LoaderError> {
    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        return Ok(Vec::new());
    }
    let bytes = resp.bytes().await?;
    Ok(parse_prism_loader_versions(&bytes, game_version)?)
}

/// Fetches available loader versions for a specific Minecraft version
pub async fn fetch_loader_versions(
    client: &reqwest::Client,
    loader: &ModLoaderType,
    game_version: &str,
) -> Result<Vec<String>, LoaderError> {
    let endpoints = LoaderEndpoints::production();
    fetch_loader_versions_with_endpoints(client, &endpoints, loader, game_version).await
}

async fn fetch_loader_versions_with_endpoints(
    client: &reqwest::Client,
    endpoints: &LoaderEndpoints,
    loader: &ModLoaderType,
    game_version: &str,
) -> Result<Vec<String>, LoaderError> {
    match loader {
        ModLoaderType::Vanilla | ModLoaderType::Horizon => Ok(Vec::new()),
        ModLoaderType::Fabric => {
            let url = format!("{}/{game_version}", endpoints.fabric_meta_base);
            fetch_meta_loader_versions(client, &url).await
        }
        ModLoaderType::Quilt => {
            let url = format!("{}/{game_version}", endpoints.quilt_meta_base);
            fetch_meta_loader_versions(client, &url).await
        }
        ModLoaderType::NeoForge => {
            let url = format!("{}/net.neoforged/index.json", endpoints.prism_meta_base);
            fetch_prism_loader_versions(client, &url, game_version).await
        }
        ModLoaderType::Forge => {
            let url = format!(
                "{}/net.minecraftforge/index.json",
                endpoints.prism_meta_base
            );
            fetch_prism_loader_versions(client, &url, game_version).await
        }
    }
}

async fn resolve_meta_profile(
    client: &reqwest::Client,
    url: &str,
    profile_id: String,
    default_main_class: &str,
    default_maven_url: &str,
) -> Result<LoaderProfile, LoaderError> {
    let resp = client.get(url).send().await?.error_for_status()?;
    let bytes = resp.bytes().await?;
    Ok(parse_meta_profile(
        &bytes,
        profile_id,
        default_main_class,
        default_maven_url,
    )?)
}

async fn resolve_prism_profile(
    client: &reqwest::Client,
    url: &str,
    profile_id: String,
    default_maven_url: &str,
    include_forgewrapper_log4j: bool,
) -> Result<LoaderProfile, LoaderError> {
    let resp = client.get(url).send().await?.error_for_status()?;
    let bytes = resp.bytes().await?;
    Ok(parse_prism_profile(
        &bytes,
        profile_id,
        default_maven_url,
        include_forgewrapper_log4j,
    )?)
}

/// Resolves a full loader profile, consulting the local cache first.
pub async fn resolve_loader_profile(
    client: &reqwest::Client,
    cache_dir: &Path,
    loader: &ModLoaderType,
    game_version: &str,
    loader_version: &str,
) -> Result<LoaderProfile, LoaderError> {
    let endpoints = LoaderEndpoints::production();
    resolve_loader_profile_with_endpoints(
        client,
        &endpoints,
        cache_dir,
        loader,
        game_version,
        loader_version,
    )
    .await
}

async fn resolve_loader_profile_with_endpoints(
    client: &reqwest::Client,
    endpoints: &LoaderEndpoints,
    cache_dir: &Path,
    loader: &ModLoaderType,
    game_version: &str,
    loader_version: &str,
) -> Result<LoaderProfile, LoaderError> {
    if matches!(loader, ModLoaderType::Vanilla | ModLoaderType::Horizon) {
        return Err(LoaderError::VanillaNotSupported);
    }

    let normalized = normalize_loader_version(game_version, loader_version);
    let effective_ver = effective_loader_version(loader_version, &normalized);

    let profile_cache_dir = cache_dir.join("loader_profiles");
    let _ = fs::create_dir_all(&profile_cache_dir);

    let profile_file =
        profile_cache_dir.join(profile_cache_file_name(loader, game_version, effective_ver));

    if profile_file.exists() {
        if let Ok(data) = fs::read_to_string(&profile_file) {
            if let Ok(prof) = serde_json::from_str::<LoaderProfile>(&data) {
                return Ok(prof);
            }
        }
    }

    let profile = match loader {
        ModLoaderType::Vanilla | ModLoaderType::Horizon => {
            return Err(LoaderError::VanillaNotSupported);
        }
        ModLoaderType::Fabric => {
            let url = format!(
                "{}/{game_version}/{effective_ver}/profile/json",
                endpoints.fabric_meta_base
            );
            resolve_meta_profile(
                client,
                &url,
                format!("fabric-{game_version}-{effective_ver}"),
                "net.fabricmc.loader.impl.launch.knot.KnotClient",
                "https://maven.fabricmc.net/",
            )
            .await?
        }
        ModLoaderType::Quilt => {
            let url = format!(
                "{}/{game_version}/{effective_ver}/profile/json",
                endpoints.quilt_meta_base
            );
            resolve_meta_profile(
                client,
                &url,
                format!("quilt-{game_version}-{effective_ver}"),
                "org.quiltmc.loader.impl.launch.knot.KnotClient",
                "https://maven.quiltmc.org/repository/release/",
            )
            .await?
        }
        ModLoaderType::NeoForge => {
            let url = format!(
                "{}/net.neoforged/{effective_ver}.json",
                endpoints.prism_meta_base
            );
            resolve_prism_profile(
                client,
                &url,
                format!("neoforge-{game_version}-{effective_ver}"),
                "https://maven.neoforged.net/releases/",
                false,
            )
            .await?
        }
        ModLoaderType::Forge => {
            let url = format!(
                "{}/net.minecraftforge/{effective_ver}.json",
                endpoints.prism_meta_base
            );
            resolve_prism_profile(
                client,
                &url,
                format!("forge-{game_version}-{effective_ver}"),
                "https://maven.minecraftforge.net/",
                true,
            )
            .await?
        }
    };

    if let Ok(data) = serde_json::to_string(&profile) {
        let _ = ingot_core::write_file_atomic(&profile_file, data.as_bytes());
    }

    Ok(profile)
}

#[cfg(test)]
mod tests;
