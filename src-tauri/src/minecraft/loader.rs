use crate::minecraft::instance::ModLoaderType;
use crate::minecraft::version::{
    maven_to_path, DownloadArtifact, LibraryDownloads, LibraryEntry,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoaderProfile {
    pub id: String,
    pub main_class: String,
    pub libraries: Vec<LibraryEntry>,
    pub jvm_args: Vec<String>,
    pub game_args: Vec<String>,
}

// Fabric meta types
#[derive(Debug, Deserialize)]
struct FabricLoaderVersion {
    version: String,
}

#[derive(Debug, Deserialize)]
struct FabricVersionEntry {
    loader: FabricLoaderVersion,
}

// Quilt meta types
#[derive(Debug, Deserialize)]
struct QuiltLoaderVersion {
    version: String,
}

#[derive(Debug, Deserialize)]
struct QuiltVersionEntry {
    loader: QuiltLoaderVersion,
}

// Prism meta types for Forge / NeoForge
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
}

#[derive(Debug, Deserialize)]
struct PrismComponent {
    #[serde(rename = "mainClass")]
    main_class: Option<String>,
    libraries: Option<Vec<PrismLibrary>>,
}

/// Fetches available loader versions for a specific Minecraft version
pub async fn fetch_loader_versions(
    client: &reqwest::Client,
    loader: &ModLoaderType,
    game_version: &str,
) -> Result<Vec<String>, String> {
    match loader {
        ModLoaderType::Vanilla => Ok(Vec::new()),
        ModLoaderType::Fabric => {
            let url = format!(
                "https://meta.fabricmc.net/v2/versions/loader/{}",
                game_version
            );
            let resp = client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch Fabric versions: {e}"))?;

            if !resp.status().is_success() {
                return Ok(Vec::new());
            }

            let entries = resp
                .json::<Vec<FabricVersionEntry>>()
                .await
                .map_err(|e| format!("Failed to parse Fabric versions: {e}"))?;

            Ok(entries.into_iter().map(|e| e.loader.version).collect())
        }
        ModLoaderType::Quilt => {
            let url = format!(
                "https://meta.quiltmc.org/v3/versions/loader/{}",
                game_version
            );
            let resp = client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch Quilt versions: {e}"))?;

            if !resp.status().is_success() {
                return Ok(Vec::new());
            }

            let entries = resp
                .json::<Vec<QuiltVersionEntry>>()
                .await
                .map_err(|e| format!("Failed to parse Quilt versions: {e}"))?;

            Ok(entries.into_iter().map(|e| e.loader.version).collect())
        }
        ModLoaderType::NeoForge => {
            let url = "https://meta.prismlauncher.org/v1/net.neoforged/index.json";
            let resp = client
                .get(url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch NeoForge versions: {e}"))?;

            if !resp.status().is_success() {
                return Ok(Vec::new());
            }

            let index = resp
                .json::<PrismIndex>()
                .await
                .map_err(|e| format!("Failed to parse NeoForge index: {e}"))?;

            let matching: Vec<String> = index
                .versions
                .into_iter()
                .filter(|v| {
                    if let Some(ref reqs) = v.requires {
                        reqs.iter().any(|r| {
                            r.uid == "net.minecraft" && r.equals.as_deref() == Some(game_version)
                        })
                    } else {
                        false
                    }
                })
                .map(|v| v.version)
                .collect();

            Ok(matching)
        }
        ModLoaderType::Forge => {
            let url = "https://meta.prismlauncher.org/v1/net.minecraftforge/index.json";
            let resp = client
                .get(url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch Forge versions: {e}"))?;

            if !resp.status().is_success() {
                return Ok(Vec::new());
            }

            let index = resp
                .json::<PrismIndex>()
                .await
                .map_err(|e| format!("Failed to parse Forge index: {e}"))?;

            let matching: Vec<String> = index
                .versions
                .into_iter()
                .filter(|v| {
                    if let Some(ref reqs) = v.requires {
                        reqs.iter().any(|r| {
                            r.uid == "net.minecraft" && r.equals.as_deref() == Some(game_version)
                        })
                    } else {
                        false
                    }
                })
                .map(|v| v.version)
                .collect();

            Ok(matching)
        }
    }
}

/// Resolves a full profile for Fabric or Quilt
pub async fn resolve_loader_profile(
    client: &reqwest::Client,
    cache_dir: &Path,
    loader: &ModLoaderType,
    game_version: &str,
    loader_version: &str,
) -> Result<LoaderProfile, String> {
    let profile_cache_dir = cache_dir.join("loader_profiles");
    let _ = fs::create_dir_all(&profile_cache_dir);

    let profile_file = profile_cache_dir.join(format!(
        "{:?}_{}_{}.json",
        loader, game_version, loader_version
    ));

    if profile_file.exists() {
        if let Ok(data) = fs::read_to_string(&profile_file) {
            if let Ok(prof) = serde_json::from_str::<LoaderProfile>(&data) {
                return Ok(prof);
            }
        }
    }

    match loader {
        ModLoaderType::Vanilla => Err("Vanilla has no loader profile".into()),
        ModLoaderType::Fabric => {
            let url = format!(
                "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
                game_version, loader_version
            );
            let resp = client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch Fabric profile: {e}"))?;

            let val = resp
                .json::<serde_json::Value>()
                .await
                .map_err(|e| format!("Failed to parse Fabric profile: {e}"))?;

            let main_class = val["mainClass"]
                .as_str()
                .unwrap_or("net.fabricmc.loader.impl.launch.knot.KnotClient")
                .to_string();

            let raw_libs = val["libraries"].as_array().cloned().unwrap_or_default();
            let mut libraries = Vec::new();

            for lib in raw_libs {
                let name = lib["name"].as_str().unwrap_or_default().to_string();
                let base_url = lib["url"].as_str().unwrap_or("https://maven.fabricmc.net/");
                if let Some(rel_path) = maven_to_path(&name, None) {
                    let full_url = format!("{}{}", base_url, rel_path);
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

            let profile = LoaderProfile {
                id: format!("fabric-{}-{}", game_version, loader_version),
                main_class,
                libraries,
                jvm_args: Vec::new(),
                game_args: Vec::new(),
            };

            if let Ok(data) = serde_json::to_string(&profile) {
                let _ = fs::write(&profile_file, data);
            }

            Ok(profile)
        }
        ModLoaderType::Quilt => {
            let url = format!(
                "https://meta.quiltmc.org/v3/versions/loader/{}/{}/profile/json",
                game_version, loader_version
            );
            let resp = client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch Quilt profile: {e}"))?;

            let val = resp
                .json::<serde_json::Value>()
                .await
                .map_err(|e| format!("Failed to parse Quilt profile: {e}"))?;

            let main_class = val["mainClass"]
                .as_str()
                .unwrap_or("org.quiltmc.loader.impl.launch.knot.KnotClient")
                .to_string();

            let raw_libs = val["libraries"].as_array().cloned().unwrap_or_default();
            let mut libraries = Vec::new();

            for lib in raw_libs {
                let name = lib["name"].as_str().unwrap_or_default().to_string();
                let base_url = lib["url"].as_str().unwrap_or("https://maven.quiltmc.org/repository/release/");
                if let Some(rel_path) = maven_to_path(&name, None) {
                    let full_url = format!("{}{}", base_url, rel_path);
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

            let profile = LoaderProfile {
                id: format!("quilt-{}-{}", game_version, loader_version),
                main_class,
                libraries,
                jvm_args: Vec::new(),
                game_args: Vec::new(),
            };

            if let Ok(data) = serde_json::to_string(&profile) {
                let _ = fs::write(&profile_file, data);
            }

            Ok(profile)
        }
        ModLoaderType::NeoForge => {
            let url = format!(
                "https://meta.prismlauncher.org/v1/net.neoforged/{}.json",
                loader_version
            );
            let resp = client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch NeoForge profile: {e}"))?;

            let component = resp
                .json::<PrismComponent>()
                .await
                .map_err(|e| format!("Failed to parse NeoForge profile: {e}"))?;

            let main_class = component
                .main_class
                .unwrap_or("cpw.mods.bootstraplauncher.BootstrapLauncher".into());

            let mut libraries = Vec::new();
            if let Some(libs) = component.libraries {
                for lib in libs {
                    let name = lib.name;
                    let rel_path = maven_to_path(&name, None);
                    let base_url = lib
                        .url
                        .as_deref()
                        .unwrap_or("https://maven.neoforged.net/releases/");
                    let full_url = if let Some(ref rel) = rel_path {
                        format!("{}{}", base_url, rel)
                    } else {
                        base_url.to_string()
                    };

                    libraries.push(LibraryEntry {
                        name,
                        downloads: Some(LibraryDownloads {
                            artifact: Some(DownloadArtifact {
                                path: rel_path,
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

            let profile = LoaderProfile {
                id: format!("neoforge-{}-{}", game_version, loader_version),
                main_class,
                libraries,
                jvm_args: Vec::new(),
                game_args: Vec::new(),
            };

            if let Ok(data) = serde_json::to_string(&profile) {
                let _ = fs::write(&profile_file, data);
            }

            Ok(profile)
        }
        ModLoaderType::Forge => {
            let url = format!(
                "https://meta.prismlauncher.org/v1/net.minecraftforge/{}.json",
                loader_version
            );
            let resp = client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch Forge profile: {e}"))?;

            let component = resp
                .json::<PrismComponent>()
                .await
                .map_err(|e| format!("Failed to parse Forge profile: {e}"))?;

            let main_class = component
                .main_class
                .unwrap_or("cpw.mods.bootstraplauncher.BootstrapLauncher".into());

            let mut libraries = Vec::new();
            if let Some(libs) = component.libraries {
                for lib in libs {
                    let name = lib.name;
                    let rel_path = maven_to_path(&name, None);
                    let base_url = lib
                        .url
                        .as_deref()
                        .unwrap_or("https://maven.minecraftforge.net/");
                    let full_url = if let Some(ref rel) = rel_path {
                        format!("{}{}", base_url, rel)
                    } else {
                        base_url.to_string()
                    };

                    libraries.push(LibraryEntry {
                        name,
                        downloads: Some(LibraryDownloads {
                            artifact: Some(DownloadArtifact {
                                path: rel_path,
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

            let profile = LoaderProfile {
                id: format!("forge-{}-{}", game_version, loader_version),
                main_class,
                libraries,
                jvm_args: Vec::new(),
                game_args: Vec::new(),
            };

            if let Ok(data) = serde_json::to_string(&profile) {
                let _ = fs::write(&profile_file, data);
            }

            Ok(profile)
        }
    }
}
