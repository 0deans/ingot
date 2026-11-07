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
    downloads: Option<LibraryDownloads>,
    rules: Option<Vec<crate::minecraft::version::RuleEntry>>,
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
        let full_url = if let Some(ref rel) = rel_path {
            if base_url.ends_with('/') {
                format!("{}{}", base_url, rel)
            } else {
                format!("{}/{}", base_url, rel)
            }
        } else {
            base_url.to_string()
        };
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

/// Fetches available loader versions for a specific Minecraft version
pub async fn fetch_loader_versions(
    client: &reqwest::Client,
    loader: &ModLoaderType,
    game_version: &str,
) -> Result<Vec<String>, LoaderError> {
    match loader {
        ModLoaderType::Vanilla => Ok(Vec::new()),
        ModLoaderType::Fabric => {
            let url = format!(
                "https://meta.fabricmc.net/v2/versions/loader/{}",
                game_version
            );
            let resp = client.get(&url).send().await?;

            if !resp.status().is_success() {
                return Ok(Vec::new());
            }

            let entries = resp.json::<Vec<FabricVersionEntry>>().await?;
            Ok(entries.into_iter().map(|e| e.loader.version).collect())
        }
        ModLoaderType::Quilt => {
            let url = format!(
                "https://meta.quiltmc.org/v3/versions/loader/{}",
                game_version
            );
            let resp = client.get(&url).send().await?;

            if !resp.status().is_success() {
                return Ok(Vec::new());
            }

            let entries = resp.json::<Vec<QuiltVersionEntry>>().await?;
            Ok(entries.into_iter().map(|e| e.loader.version).collect())
        }
        ModLoaderType::NeoForge => {
            let url = "https://meta.prismlauncher.org/v1/net.neoforged/index.json";
            let resp = client.get(url).send().await?;

            if !resp.status().is_success() {
                return Ok(Vec::new());
            }

            let index = resp.json::<PrismIndex>().await?;
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
            let resp = client.get(url).send().await?;

            if !resp.status().is_success() {
                return Ok(Vec::new());
            }

            let index = resp.json::<PrismIndex>().await?;
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
        ModLoaderType::Horizon => Ok(Vec::new()),
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

/// Resolves a full profile for Fabric or Quilt
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

    match loader {
        ModLoaderType::Vanilla | ModLoaderType::Horizon => Err(LoaderError::VanillaNotSupported),
        ModLoaderType::Fabric => {
            let url = format!(
                "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
                game_version, effective_loader_version
            );
            let resp = client.get(&url).send().await?;
            let val = resp.json::<serde_json::Value>().await?;

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
                    let full_url = if base_url.ends_with('/') {
                        format!("{}{}", base_url, rel_path)
                    } else {
                        format!("{}/{}", base_url, rel_path)
                    };
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
                id: format!("fabric-{}-{}", game_version, effective_loader_version),
                main_class,
                libraries,
                maven_files: Vec::new(),
                jvm_args: Vec::new(),
                game_args: Vec::new(),
                minecraft_arguments: None,
            };

            if let Ok(data) = serde_json::to_string(&profile) {
                let _ = fs::write(&profile_file, data);
            }

            Ok(profile)
        }
        ModLoaderType::Quilt => {
            let url = format!(
                "https://meta.quiltmc.org/v3/versions/loader/{}/{}/profile/json",
                game_version, effective_loader_version
            );
            let resp = client.get(&url).send().await?;
            let val = resp.json::<serde_json::Value>().await?;

            let main_class = val["mainClass"]
                .as_str()
                .unwrap_or("org.quiltmc.loader.impl.launch.knot.KnotClient")
                .to_string();

            let raw_libs = val["libraries"].as_array().cloned().unwrap_or_default();
            let mut libraries = Vec::new();

            for lib in raw_libs {
                let name = lib["name"].as_str().unwrap_or_default().to_string();
                let base_url = lib["url"]
                    .as_str()
                    .unwrap_or("https://maven.quiltmc.org/repository/release/");
                if let Some(rel_path) = maven_to_path(&name, None) {
                    let full_url = if base_url.ends_with('/') {
                        format!("{}{}", base_url, rel_path)
                    } else {
                        format!("{}/{}", base_url, rel_path)
                    };
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
                id: format!("quilt-{}-{}", game_version, effective_loader_version),
                main_class,
                libraries,
                maven_files: Vec::new(),
                jvm_args: Vec::new(),
                game_args: Vec::new(),
                minecraft_arguments: None,
            };

            if let Ok(data) = serde_json::to_string(&profile) {
                let _ = fs::write(&profile_file, data);
            }

            Ok(profile)
        }
        ModLoaderType::NeoForge => {
            let url = format!(
                "https://meta.prismlauncher.org/v1/net.neoforged/{}.json",
                effective_loader_version
            );
            let resp = client.get(&url).send().await?;
            let component = resp.json::<PrismComponent>().await?;

            let main_class = component
                .main_class
                .unwrap_or_else(|| "cpw.mods.bootstraplauncher.BootstrapLauncher".into());

            let mut game_args = Vec::new();
            if let Some(tweakers) = component.tweakers {
                for t in tweakers {
                    game_args.push("--tweakClass".to_string());
                    game_args.push(t);
                }
            }

            let mut libraries = Vec::new();
            if let Some(libs) = component.libraries {
                for lib in libs {
                    libraries.push(convert_prism_library(
                        lib,
                        "https://maven.neoforged.net/releases/",
                    ));
                }
            }
            let mut maven_files = Vec::new();
            if let Some(files) = component.maven_files {
                for lib in files {
                    maven_files.push(convert_prism_library(
                        lib,
                        "https://maven.neoforged.net/releases/",
                    ));
                }
            }

            let profile = LoaderProfile {
                id: format!("neoforge-{}-{}", game_version, effective_loader_version),
                main_class,
                libraries,
                maven_files,
                jvm_args: Vec::new(),
                game_args,
                minecraft_arguments: component.minecraft_arguments,
            };

            if let Ok(data) = serde_json::to_string(&profile) {
                let _ = fs::write(&profile_file, data);
            }

            Ok(profile)
        }
        ModLoaderType::Forge => {
            let url = format!(
                "https://meta.prismlauncher.org/v1/net.minecraftforge/{}.json",
                effective_loader_version
            );
            let resp = client.get(&url).send().await?;
            let component = resp.json::<PrismComponent>().await?;

            let main_class = component
                .main_class
                .unwrap_or_else(|| "cpw.mods.bootstraplauncher.BootstrapLauncher".into());

            let mut game_args = Vec::new();
            if let Some(tweakers) = component.tweakers {
                for t in tweakers {
                    game_args.push("--tweakClass".to_string());
                    game_args.push(t);
                }
            }

            let mut libraries = Vec::new();
            if let Some(libs) = component.libraries {
                for lib in libs {
                    libraries.push(convert_prism_library(
                        lib,
                        "https://maven.minecraftforge.net/",
                    ));
                }
            }
            let mut maven_files = Vec::new();
            if let Some(files) = component.maven_files {
                for lib in files {
                    maven_files.push(convert_prism_library(
                        lib,
                        "https://maven.minecraftforge.net/",
                    ));
                }
            }

            if main_class.contains("forgewrapper")
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

            let profile = LoaderProfile {
                id: format!("forge-{}-{}", game_version, effective_loader_version),
                main_class,
                libraries,
                maven_files,
                jvm_args: Vec::new(),
                game_args,
                minecraft_arguments: component.minecraft_arguments,
            };

            if let Ok(data) = serde_json::to_string(&profile) {
                let _ = fs::write(&profile_file, data);
            }

            Ok(profile)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_loader_version() {
        assert_eq!(
            normalize_loader_version("1.7.10", "10.13.4.1614-1.7.10"),
            "10.13.4.1614"
        );
        assert_eq!(
            normalize_loader_version("1.16.5", "forge-36.2.39"),
            "36.2.39"
        );
        assert_eq!(
            normalize_loader_version("1.16.5", "1.16.5-36.2.39"),
            "36.2.39"
        );
        assert_eq!(
            normalize_loader_version("1.20.1", "neoforge-47.1.106"),
            "47.1.106"
        );
        assert_eq!(normalize_loader_version("1.20.1", "47.1.106"), "47.1.106");
        assert_eq!(
            normalize_loader_version("1.12.2", "14.23.5.2860-1.12.2"),
            "14.23.5.2860"
        );
    }

    #[test]
    fn test_prism_library_deserialization_with_downloads_and_rules() {
        let json = r#"{
            "name": "net.minecraftforge:forge:1.16.5-36.2.39:launcher",
            "downloads": {
                "artifact": {
                    "path": "net/minecraftforge/forge/1.16.5-36.2.39/forge-1.16.5-36.2.39-launcher.jar",
                    "url": "https://files.prismlauncher.org/maven/net/minecraftforge/forge/1.16.5-36.2.39/forge-1.16.5-36.2.39-launcher.jar",
                    "sha1": "abcdef1234567890",
                    "size": 12345
                }
            },
            "rules": [
                {
                    "action": "allow",
                    "os": { "name": "windows" }
                }
            ]
        }"#;

        let prism_lib: PrismLibrary = serde_json::from_str(json).unwrap();
        assert_eq!(
            prism_lib.name,
            "net.minecraftforge:forge:1.16.5-36.2.39:launcher"
        );
        assert!(prism_lib.downloads.is_some());
        assert!(prism_lib.rules.is_some());

        let converted = convert_prism_library(prism_lib, "https://maven.minecraftforge.net/");
        assert_eq!(
            converted.name,
            "net.minecraftforge:forge:1.16.5-36.2.39:launcher"
        );
        let dl = converted.downloads.unwrap();
        let art = dl.artifact.unwrap();
        assert_eq!(
            art.path.as_deref(),
            Some("net/minecraftforge/forge/1.16.5-36.2.39/forge-1.16.5-36.2.39-launcher.jar")
        );
        assert_eq!(
            art.url,
            "https://files.prismlauncher.org/maven/net/minecraftforge/forge/1.16.5-36.2.39/forge-1.16.5-36.2.39-launcher.jar"
        );
        assert_eq!(art.sha1.as_deref(), Some("abcdef1234567890"));
        assert_eq!(art.size, Some(12345));
        assert!(converted.rules.is_some());
    }

    #[test]
    fn test_prism_library_conversion_fills_missing_path() {
        let json = r#"{
            "name": "net.minecraftforge:forge:1.16.5-36.2.39:launcher",
            "downloads": {
                "artifact": {
                    "url": "https://files.prismlauncher.org/maven/test.jar"
                }
            }
        }"#;

        let prism_lib: PrismLibrary = serde_json::from_str(json).unwrap();
        let converted = convert_prism_library(prism_lib, "https://maven.minecraftforge.net/");
        let dl = converted.downloads.unwrap();
        let art = dl.artifact.unwrap();
        assert_eq!(
            art.path.as_deref(),
            Some("net/minecraftforge/forge/1.16.5-36.2.39/forge-1.16.5-36.2.39-launcher.jar")
        );
        assert_eq!(art.url, "https://files.prismlauncher.org/maven/test.jar");
    }

    #[test]
    fn test_prism_library_conversion_fallback_without_downloads() {
        let prism_lib = PrismLibrary {
            name: "cpw.mods:modlauncher:8.0.9".to_string(),
            url: Some("https://maven.minecraftforge.net/".to_string()),
            downloads: None,
            rules: None,
        };

        let converted = convert_prism_library(prism_lib, "https://maven.minecraftforge.net/");
        let dl = converted.downloads.unwrap();
        let art = dl.artifact.unwrap();
        assert_eq!(
            art.path.as_deref(),
            Some("cpw/mods/modlauncher/8.0.9/modlauncher-8.0.9.jar")
        );
        assert_eq!(
            art.url,
            "https://maven.minecraftforge.net/cpw/mods/modlauncher/8.0.9/modlauncher-8.0.9.jar"
        );
    }

    #[test]
    fn test_prism_component_deserialization_with_tweakers_and_minecraft_arguments() {
        let json = r#"{
            "mainClass": "net.minecraft.launchwrapper.Launch",
            "+tweakers": ["net.minecraftforge.fml.common.launcher.FMLTweaker"],
            "minecraftArguments": "--username ${auth_player_name} --version ${version_name}"
        }"#;

        let comp: PrismComponent = serde_json::from_str(json).unwrap();
        assert_eq!(
            comp.main_class.as_deref(),
            Some("net.minecraft.launchwrapper.Launch")
        );
        assert_eq!(
            comp.tweakers.as_deref(),
            Some(&["net.minecraftforge.fml.common.launcher.FMLTweaker".to_string()][..])
        );
        assert_eq!(
            comp.minecraft_arguments.as_deref(),
            Some("--username ${auth_player_name} --version ${version_name}")
        );
    }

    #[test]
    fn test_forge_log4j_library_helper() {
        let lib = forge_log4j_library(
            "org.apache.logging.log4j:log4j-core:2.15.0",
            "ba55c13d7ac2fd44df9cc8074455719a33f375b9",
            1789769,
        );
        assert_eq!(lib.name, "org.apache.logging.log4j:log4j-core:2.15.0");
        let dl = lib.downloads.unwrap();
        let art = dl.artifact.unwrap();
        assert_eq!(
            art.path.as_deref(),
            Some("org/apache/logging/log4j/log4j-core/2.15.0/log4j-core-2.15.0.jar")
        );
        assert_eq!(
            art.url,
            "https://libraries.minecraft.net/org/apache/logging/log4j/log4j-core/2.15.0/log4j-core-2.15.0.jar"
        );
        assert_eq!(
            art.sha1.as_deref(),
            Some("ba55c13d7ac2fd44df9cc8074455719a33f375b9")
        );
        assert_eq!(art.size, Some(1789769));
    }

    #[tokio::test]
    async fn test_resolve_loader_profile_reads_v5_cache() {
        let temp_dir = tempfile::tempdir().unwrap();
        let profiles_dir = temp_dir.path().join("loader_profiles");
        fs::create_dir_all(&profiles_dir).unwrap();

        let cached_profile = LoaderProfile {
            id: "forge-1.16.5-36.2.39".into(),
            main_class: "io.github.zekerzhayard.forgewrapper.installer.Main".into(),
            libraries: vec![forge_log4j_library(
                "org.apache.logging.log4j:log4j-core:2.15.0",
                "ba55c13d7ac2fd44df9cc8074455719a33f375b9",
                1789769,
            )],
            maven_files: vec![LibraryEntry {
                name: "com.google.guava:guava:20.0".into(),
                downloads: None,
                rules: None,
                natives: None,
                url: None,
            }],
            jvm_args: Vec::new(),
            game_args: Vec::new(),
            minecraft_arguments: None,
        };
        fs::write(
            profiles_dir.join("v5_Forge_1.16.5_36.2.39.json"),
            serde_json::to_string(&cached_profile).unwrap(),
        )
        .unwrap();

        let client = reqwest::Client::new();
        let loaded = resolve_loader_profile(
            &client,
            temp_dir.path(),
            &ModLoaderType::Forge,
            "1.16.5",
            "36.2.39",
        )
        .await
        .unwrap();

        assert_eq!(loaded.id, "forge-1.16.5-36.2.39");
        assert_eq!(loaded.libraries.len(), 1);
        assert_eq!(
            loaded.libraries[0].name,
            "org.apache.logging.log4j:log4j-core:2.15.0"
        );
        assert_eq!(loaded.maven_files.len(), 1);
        assert_eq!(loaded.maven_files[0].name, "com.google.guava:guava:20.0");
    }

    #[tokio::test]
    #[ignore = "requires public network to fetch Prism meta"]
    async fn test_resolve_loader_profile_forge_1_16_5_live() {
        let temp_dir = tempfile::tempdir().unwrap();
        let client = reqwest::Client::new();
        let profile = resolve_loader_profile(
            &client,
            temp_dir.path(),
            &ModLoaderType::Forge,
            "1.16.5",
            "36.2.39",
        )
        .await
        .unwrap();

        assert_eq!(
            profile.main_class,
            "io.github.zekerzhayard.forgewrapper.installer.Main"
        );
        assert!(!profile
            .libraries
            .iter()
            .any(|l| l.name.starts_with("com.google.guava:guava:20.0")));
        assert!(profile
            .maven_files
            .iter()
            .any(|l| l.name.starts_with("com.google.guava:guava:20.0")));
        assert!(profile
            .libraries
            .iter()
            .any(|l| l.name == "org.apache.logging.log4j:log4j-core:2.15.0"));
        assert!(profile
            .libraries
            .iter()
            .any(|l| l.name == "org.apache.logging.log4j:log4j-api:2.15.0"));
        assert!(profile
            .libraries
            .iter()
            .any(|l| l.name == "org.apache.logging.log4j:log4j-slf4j18-impl:2.15.0"));
    }

    #[tokio::test]
    #[ignore = "requires public network to download Log4j jars"]
    async fn test_download_log4j_libraries_live() {
        use crate::minecraft::downloader::{download_files_concurrent, DownloadTask};

        let temp_dir = tempfile::tempdir().unwrap();
        let client = reqwest::Client::new();
        let libs = vec![
            forge_log4j_library(
                "org.apache.logging.log4j:log4j-api:2.15.0",
                "4a5aa7e55a29391c6f66e0b259d5189aa11e45d0",
                301804,
            ),
            forge_log4j_library(
                "org.apache.logging.log4j:log4j-core:2.15.0",
                "ba55c13d7ac2fd44df9cc8074455719a33f375b9",
                1789769,
            ),
            forge_log4j_library(
                "org.apache.logging.log4j:log4j-slf4j18-impl:2.15.0",
                "88f72ad364bfc3a7cf43186fc17212f2b4bb8d97",
                21223,
            ),
        ];

        let mut tasks = Vec::new();
        for lib in libs {
            let dl = lib.downloads.unwrap();
            let art = dl.artifact.unwrap();
            let dest = temp_dir.path().join(art.path.unwrap());
            tasks.push(DownloadTask {
                url: art.url,
                dest,
                sha1: art.sha1,
                size: art.size,
            });
        }

        download_files_concurrent(&client, tasks, 3, None)
            .await
            .expect("Log4j libraries download failed");
    }
}
