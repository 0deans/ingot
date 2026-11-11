use crate::minecraft::instance::ModLoaderType;
use crate::minecraft::version::{maven_to_path, DownloadArtifact, LibraryDownloads, LibraryEntry};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

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

#[derive(Debug, Deserialize)]
struct MetaLoaderVersion {
    version: String,
}

#[derive(Debug, Deserialize)]
struct MetaVersionEntry {
    loader: MetaLoaderVersion,
}

#[derive(Debug, Deserialize)]
struct PrismRequirement {
    equals: Option<String>,
    uid: String,
}

#[derive(Debug, Deserialize)]
struct PrismVersionEntry {
    version: String,
    requires: Option<Vec<PrismRequirement>>,
}

#[derive(Debug, Deserialize)]
struct PrismIndex {
    versions: Vec<PrismVersionEntry>,
}

#[derive(Debug, Deserialize)]
struct PrismLibrary {
    name: String,
    url: Option<String>,
    downloads: Option<LibraryDownloads>,
    rules: Option<Vec<crate::minecraft::version::RuleEntry>>,
}

#[derive(Debug, Deserialize)]
struct PrismComponent {
    #[serde(rename = "mainClass")]
    main_class: Option<String>,
    #[serde(rename = "minecraftArguments")]
    minecraft_arguments: Option<String>,
    #[serde(rename = "+tweakers")]
    tweakers: Option<Vec<String>>,
    libraries: Option<Vec<PrismLibrary>>,
    #[serde(rename = "mavenFiles")]
    maven_files: Option<Vec<PrismLibrary>>,
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

fn maven_artifact_url(base_url: &str, rel_path: &str) -> String {
    if base_url.ends_with('/') {
        format!("{base_url}{rel_path}")
    } else {
        format!("{base_url}/{rel_path}")
    }
}

fn convert_prism_library(lib: PrismLibrary, default_base_url: &str) -> LibraryEntry {
    let name = lib.name;
    let rel_path = maven_to_path(&name, None);
    let downloads = if let Some(mut downloads) = lib.downloads {
        if let Some(ref mut artifact) = downloads.artifact {
            if artifact.path.is_none() {
                artifact.path = rel_path;
            }
        }
        Some(downloads)
    } else {
        let base_url = lib.url.as_deref().unwrap_or(default_base_url);
        let full_url = rel_path
            .as_deref()
            .map(|rel| maven_artifact_url(base_url, rel))
            .unwrap_or_else(|| base_url.to_string());
        Some(LibraryDownloads {
            artifact: Some(DownloadArtifact {
                path: rel_path,
                sha1: None,
                size: None,
                url: full_url,
            }),
            classifiers: None,
        })
    };

    LibraryEntry {
        name,
        downloads,
        rules: lib.rules,
        natives: None,
        url: None,
    }
}

fn forge_log4j_library(name: &str, sha1: &str, size: u64) -> LibraryEntry {
    let rel_path = maven_to_path(name, None);
    let url = format!(
        "https://libraries.minecraft.net/{}",
        rel_path.as_deref().unwrap_or_default()
    );
    LibraryEntry {
        name: name.to_string(),
        downloads: Some(LibraryDownloads {
            artifact: Some(DownloadArtifact {
                path: rel_path,
                sha1: Some(sha1.to_string()),
                size: Some(size),
                url,
            }),
            classifiers: None,
        }),
        rules: None,
        natives: None,
        url: None,
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
    let entries = resp.json::<Vec<MetaVersionEntry>>().await?;
    Ok(entries.into_iter().map(|e| e.loader.version).collect())
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
    let index = resp.json::<PrismIndex>().await?;
    let matching = index
        .versions
        .into_iter()
        .filter(|entry| {
            entry.requires.as_ref().is_some_and(|reqs| {
                reqs.iter()
                    .any(|r| r.uid == "net.minecraft" && r.equals.as_deref() == Some(game_version))
            })
        })
        .map(|entry| entry.version)
        .collect();
    Ok(matching)
}

/// Fetches available loader versions for a specific Minecraft version
pub async fn fetch_loader_versions(
    client: &reqwest::Client,
    loader: &ModLoaderType,
    game_version: &str,
) -> Result<Vec<String>, LoaderError> {
    match loader {
        ModLoaderType::Vanilla | ModLoaderType::Horizon => Ok(Vec::new()),
        ModLoaderType::Fabric => {
            let url = format!("https://meta.fabricmc.net/v2/versions/loader/{game_version}");
            fetch_meta_loader_versions(client, &url).await
        }
        ModLoaderType::Quilt => {
            let url = format!("https://meta.quiltmc.org/v3/versions/loader/{game_version}");
            fetch_meta_loader_versions(client, &url).await
        }
        ModLoaderType::NeoForge => {
            let url = "https://meta.prismlauncher.org/v1/net.neoforged/index.json";
            fetch_prism_loader_versions(client, url, game_version).await
        }
        ModLoaderType::Forge => {
            let url = "https://meta.prismlauncher.org/v1/net.minecraftforge/index.json";
            fetch_prism_loader_versions(client, url, game_version).await
        }
    }
}

/// Normalizes a loader version string by trimming whitespace, stripping case-insensitive
/// prefixes ("forge-", "neoforge-"), and stripping redundant leading/trailing game versions.
pub fn normalize_loader_version(game_version: &str, loader_version: &str) -> String {
    let mut ver = loader_version.trim();

    let lower = ver.to_ascii_lowercase();
    if lower.starts_with("forge-") {
        ver = &ver[6..];
    } else if lower.starts_with("neoforge-") {
        ver = &ver[9..];
    }

    if !game_version.is_empty() {
        let mut changed = true;
        while changed {
            changed = false;
            if let Some(rest) = ver.strip_prefix(game_version) {
                if rest.starts_with('-') || rest.starts_with('_') {
                    ver = rest.trim_start_matches(['-', '_']);
                    changed = true;
                }
            }
            if let Some(rest) = ver.strip_suffix(game_version) {
                if rest.ends_with('-') || rest.ends_with('_') {
                    ver = rest.trim_end_matches(['-', '_']);
                    changed = true;
                }
            }
        }
    }

    ver.trim_matches('-').to_string()
}

async fn resolve_meta_profile(
    client: &reqwest::Client,
    url: &str,
    profile_id: String,
    default_main_class: &str,
    default_maven_url: &str,
) -> Result<LoaderProfile, LoaderError> {
    let resp = client.get(url).send().await?;
    let val = resp.json::<serde_json::Value>().await?;

    let main_class = val["mainClass"]
        .as_str()
        .unwrap_or(default_main_class)
        .to_string();

    let raw_libs = val["libraries"].as_array().cloned().unwrap_or_default();
    let mut libraries = Vec::new();

    for lib in raw_libs {
        let name = lib["name"].as_str().unwrap_or_default().to_string();
        let base_url = lib["url"].as_str().unwrap_or(default_maven_url);
        if let Some(rel_path) = maven_to_path(&name, None) {
            let full_url = maven_artifact_url(base_url, &rel_path);
            libraries.push(LibraryEntry {
                name,
                downloads: Some(LibraryDownloads {
                    artifact: Some(DownloadArtifact {
                        path: Some(rel_path),
                        sha1: None,
                        size: None,
                        url: full_url,
                    }),
                    classifiers: None,
                }),
                rules: None,
                natives: None,
                url: None,
            });
        }
    }

    Ok(LoaderProfile {
        id: profile_id,
        main_class,
        libraries,
        maven_files: Vec::new(),
        jvm_args: Vec::new(),
        game_args: Vec::new(),
        minecraft_arguments: None,
    })
}

async fn resolve_prism_profile(
    client: &reqwest::Client,
    url: &str,
    profile_id: String,
    default_maven_url: &str,
    include_forgewrapper_log4j: bool,
) -> Result<LoaderProfile, LoaderError> {
    let resp = client.get(url).send().await?;
    let component = resp.json::<PrismComponent>().await?;

    let main_class = component
        .main_class
        .unwrap_or_else(|| "cpw.mods.bootstraplauncher.BootstrapLauncher".into());

    let mut game_args = Vec::new();
    if let Some(tweakers) = component.tweakers {
        for tweaker in tweakers {
            game_args.push("--tweakClass".to_string());
            game_args.push(tweaker);
        }
    }

    let mut libraries: Vec<LibraryEntry> = component
        .libraries
        .unwrap_or_default()
        .into_iter()
        .map(|lib| convert_prism_library(lib, default_maven_url))
        .collect();

    let maven_files: Vec<LibraryEntry> = component
        .maven_files
        .unwrap_or_default()
        .into_iter()
        .map(|lib| convert_prism_library(lib, default_maven_url))
        .collect();

    if include_forgewrapper_log4j
        && main_class.contains("forgewrapper")
        && !libraries
            .iter()
            .any(|l| l.name.starts_with("org.apache.logging.log4j:log4j-core:"))
    {
        libraries.push(forge_log4j_library(
            "org.apache.logging.log4j:log4j-api:2.15.0",
            "4a5aa7e55a29391c6f66e0b259d5189aa11e45d0",
            301804,
        ));
        libraries.push(forge_log4j_library(
            "org.apache.logging.log4j:log4j-core:2.15.0",
            "ba55c13d7ac2fd44df9cc8074455719a33f375b9",
            1789769,
        ));
        libraries.push(forge_log4j_library(
            "org.apache.logging.log4j:log4j-slf4j18-impl:2.15.0",
            "88f72ad364bfc3a7cf43186fc17212f2b4bb8d97",
            21223,
        ));
    }

    Ok(LoaderProfile {
        id: profile_id,
        main_class,
        libraries,
        maven_files,
        jvm_args: Vec::new(),
        game_args,
        minecraft_arguments: component.minecraft_arguments,
    })
}

/// Resolves a full loader profile, consulting the local cache first.
pub async fn resolve_loader_profile(
    client: &reqwest::Client,
    cache_dir: &Path,
    loader: &ModLoaderType,
    game_version: &str,
    loader_version: &str,
) -> Result<LoaderProfile, LoaderError> {
    let normalized = normalize_loader_version(game_version, loader_version);
    let effective_loader_version = if normalized.is_empty() {
        loader_version.trim()
    } else {
        &normalized
    };

    let profile_cache_dir = cache_dir.join("loader_profiles");
    let _ = fs::create_dir_all(&profile_cache_dir);

    let profile_file = profile_cache_dir.join(format!(
        "v5_{:?}_{}_{}.json",
        loader, game_version, effective_loader_version
    ));

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
                "https://meta.fabricmc.net/v2/versions/loader/{game_version}/{effective_loader_version}/profile/json"
            );
            resolve_meta_profile(
                client,
                &url,
                format!("fabric-{game_version}-{effective_loader_version}"),
                "net.fabricmc.loader.impl.launch.knot.KnotClient",
                "https://maven.fabricmc.net/",
            )
            .await?
        }
        ModLoaderType::Quilt => {
            let url = format!(
                "https://meta.quiltmc.org/v3/versions/loader/{game_version}/{effective_loader_version}/profile/json"
            );
            resolve_meta_profile(
                client,
                &url,
                format!("quilt-{game_version}-{effective_loader_version}"),
                "org.quiltmc.loader.impl.launch.knot.KnotClient",
                "https://maven.quiltmc.org/repository/release/",
            )
            .await?
        }
        ModLoaderType::NeoForge => {
            let url = format!(
                "https://meta.prismlauncher.org/v1/net.neoforged/{effective_loader_version}.json"
            );
            resolve_prism_profile(
                client,
                &url,
                format!("neoforge-{game_version}-{effective_loader_version}"),
                "https://maven.neoforged.net/releases/",
                false,
            )
            .await?
        }
        ModLoaderType::Forge => {
            let url = format!(
                "https://meta.prismlauncher.org/v1/net.minecraftforge/{effective_loader_version}.json"
            );
            resolve_prism_profile(
                client,
                &url,
                format!("forge-{game_version}-{effective_loader_version}"),
                "https://maven.minecraftforge.net/",
                true,
            )
            .await?
        }
    };

    if let Ok(data) = serde_json::to_string(&profile) {
        let _ = crate::core::write_file_atomic(&profile_file, data.as_bytes());
    }

    Ok(profile)
}

#[cfg(test)]
mod tests;
