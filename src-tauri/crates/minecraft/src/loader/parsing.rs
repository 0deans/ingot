//! Pure mod-loader version normalization, profile parsing, and Maven library conversion.

use crate::instance::ModLoaderType;
use crate::launcher::classpath::maven_coordinate_to_path as maven_to_path;
use crate::version::{DownloadArtifact, LibraryDownloads, LibraryEntry};
use serde::Deserialize;

use super::LoaderProfile;

const LOG4J_API_SHA1: &str = "4a5aa7e55a29391c6f66e0b259d5189aa11e45d0";
const LOG4J_CORE_SHA1: &str = "ba55c13d7ac2fd44df9cc8074455719a33f375b9";
const LOG4J_SLF4J18_SHA1: &str = "88f72ad364bfc3a7cf43186fc17212f2b4bb8d97";

#[derive(Debug, Deserialize)]
struct MetaLoaderVersion {
    version: String,
}

#[derive(Debug, Deserialize)]
struct MetaVersionEntry {
    loader: MetaLoaderVersion,
}

#[derive(Debug, Deserialize)]
struct MetaProfileLibrary {
    name: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MetaProfilePayload {
    #[serde(rename = "mainClass")]
    main_class: Option<String>,
    libraries: Option<Vec<MetaProfileLibrary>>,
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
pub(super) struct PrismLibrary {
    pub(super) name: String,
    pub(super) url: Option<String>,
    pub(super) downloads: Option<LibraryDownloads>,
    pub(super) rules: Option<Vec<crate::launcher::rules::RuleEntry>>,
}

#[derive(Debug, Deserialize)]
pub(super) struct PrismComponent {
    #[serde(rename = "mainClass")]
    pub(super) main_class: Option<String>,
    #[serde(rename = "minecraftArguments")]
    pub(super) minecraft_arguments: Option<String>,
    #[serde(rename = "+tweakers")]
    pub(super) tweakers: Option<Vec<String>>,
    pub(super) libraries: Option<Vec<PrismLibrary>>,
    #[serde(rename = "mavenFiles")]
    pub(super) maven_files: Option<Vec<PrismLibrary>>,
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

pub(super) fn effective_loader_version<'a>(
    raw_loader_version: &'a str,
    normalized: &'a str,
) -> &'a str {
    if normalized.is_empty() {
        raw_loader_version.trim()
    } else {
        normalized
    }
}

pub(super) fn profile_cache_file_name(
    loader: &ModLoaderType,
    game_version: &str,
    effective_loader_version: &str,
) -> String {
    format!("v5_{loader:?}_{game_version}_{effective_loader_version}.json")
}

pub(super) fn maven_artifact_url(base_url: &str, rel_path: &str) -> String {
    if base_url.ends_with('/') {
        format!("{base_url}{rel_path}")
    } else {
        format!("{base_url}/{rel_path}")
    }
}

pub(super) fn convert_prism_library(lib: PrismLibrary, default_base_url: &str) -> LibraryEntry {
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

pub(super) fn forge_log4j_library(name: &str, sha1: &str, size: u64) -> LibraryEntry {
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

pub(super) fn parse_meta_loader_versions(bytes: &[u8]) -> Result<Vec<String>, serde_json::Error> {
    let entries: Vec<MetaVersionEntry> = serde_json::from_slice(bytes)?;
    Ok(entries.into_iter().map(|e| e.loader.version).collect())
}

pub(super) fn parse_prism_loader_versions(
    bytes: &[u8],
    game_version: &str,
) -> Result<Vec<String>, serde_json::Error> {
    let index: PrismIndex = serde_json::from_slice(bytes)?;
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

pub(super) fn parse_meta_profile(
    bytes: &[u8],
    profile_id: String,
    default_main_class: &str,
    default_maven_url: &str,
) -> Result<LoaderProfile, serde_json::Error> {
    let payload: MetaProfilePayload = serde_json::from_slice(bytes)?;

    let main_class = payload
        .main_class
        .unwrap_or_else(|| default_main_class.to_string());

    let mut libraries = Vec::new();
    for lib in payload.libraries.unwrap_or_default() {
        let name = lib.name.unwrap_or_default();
        let base_url = lib.url.as_deref().unwrap_or(default_maven_url);
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

pub(super) fn parse_prism_profile(
    bytes: &[u8],
    profile_id: String,
    default_maven_url: &str,
    include_forgewrapper_log4j: bool,
) -> Result<LoaderProfile, serde_json::Error> {
    let component: PrismComponent = serde_json::from_slice(bytes)?;

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
            LOG4J_API_SHA1,
            301804,
        ));
        libraries.push(forge_log4j_library(
            "org.apache.logging.log4j:log4j-core:2.15.0",
            LOG4J_CORE_SHA1,
            1789769,
        ));
        libraries.push(forge_log4j_library(
            "org.apache.logging.log4j:log4j-slf4j18-impl:2.15.0",
            LOG4J_SLF4J18_SHA1,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_meta_and_prism_loader_versions() {
        let meta_json = br#"[{"loader":{"version":"0.16.9"}},{"loader":{"version":"0.16.8"}}]"#;
        assert_eq!(
            parse_meta_loader_versions(meta_json).unwrap(),
            vec!["0.16.9", "0.16.8"]
        );

        let prism_json = br#"{
            "versions": [
                {"version": "47.3.0", "requires": [{"uid": "net.minecraft", "equals": "1.20.1"}]},
                {"version": "49.0.3", "requires": [{"uid": "net.minecraft", "equals": "1.20.4"}]},
                {"version": "0.0.1"}
            ]
        }"#;
        assert_eq!(
            parse_prism_loader_versions(prism_json, "1.20.1").unwrap(),
            vec!["47.3.0"]
        );
    }

    #[test]
    fn parses_meta_profile_with_default_maven_and_skips_invalid_coords() {
        let json = br#"{
            "libraries": [
                {"name": "net.fabricmc:tiny-mappings-parser:0.3.0"},
                {"name": "org.ow2.asm:asm:9.7", "url": "https://custom.example/maven"},
                {"name": "invalid-coord"}
            ]
        }"#;

        let profile = parse_meta_profile(
            json,
            "fabric-1.21.4-0.16.9".into(),
            "net.fabricmc.loader.impl.launch.knot.KnotClient",
            "https://maven.fabricmc.net/",
        )
        .unwrap();

        assert_eq!(
            profile.main_class,
            "net.fabricmc.loader.impl.launch.knot.KnotClient"
        );
        assert_eq!(profile.libraries.len(), 2);
        assert_eq!(
            profile.libraries[0]
                .downloads
                .as_ref()
                .unwrap()
                .artifact
                .as_ref()
                .unwrap()
                .url,
            "https://maven.fabricmc.net/net/fabricmc/tiny-mappings-parser/0.3.0/tiny-mappings-parser-0.3.0.jar"
        );
        assert_eq!(
            profile.libraries[1]
                .downloads
                .as_ref()
                .unwrap()
                .artifact
                .as_ref()
                .unwrap()
                .url,
            "https://custom.example/maven/org/ow2/asm/asm/9.7/asm-9.7.jar"
        );
    }

    #[test]
    fn parses_prism_profile_tweakers_and_forgewrapper_log4j_injection() {
        let forgewrapper_json = br#"{
            "mainClass": "io.github.zekerzhayard.forgewrapper.installer.Main",
            "+tweakers": ["fml.Tweaker"],
            "minecraftArguments": "--version ${version_name}",
            "libraries": [
                {"name": "cpw.mods:modlauncher:8.0.9"}
            ],
            "mavenFiles": [
                {"name": "com.google.guava:guava:20.0"}
            ]
        }"#;

        let injected = parse_prism_profile(
            forgewrapper_json,
            "forge-1.16.5-36.2.39".into(),
            "https://maven.minecraftforge.net/",
            true,
        )
        .unwrap();
        assert_eq!(
            injected.game_args,
            vec!["--tweakClass".to_string(), "fml.Tweaker".to_string()]
        );
        assert_eq!(
            injected.minecraft_arguments.as_deref(),
            Some("--version ${version_name}")
        );
        assert_eq!(injected.maven_files.len(), 1);
        assert!(injected
            .libraries
            .iter()
            .any(|l| l.name == "org.apache.logging.log4j:log4j-core:2.15.0"));
        assert_eq!(injected.libraries.len(), 4);

        let already_has_log4j = br#"{
            "mainClass": "io.github.zekerzhayard.forgewrapper.installer.Main",
            "libraries": [
                {"name": "org.apache.logging.log4j:log4j-core:2.17.1"}
            ]
        }"#;
        let preserved = parse_prism_profile(
            already_has_log4j,
            "forge-1.18.2-40.2.0".into(),
            "https://maven.minecraftforge.net/",
            true,
        )
        .unwrap();
        assert_eq!(preserved.libraries.len(), 1);
    }

    #[test]
    fn computes_effective_loader_version_and_cache_file_name() {
        assert_eq!(
            effective_loader_version(" forge-1.20.1 ", ""),
            "forge-1.20.1"
        );
        assert_eq!(effective_loader_version("47.1.0", "47.1.0"), "47.1.0");
        assert_eq!(
            profile_cache_file_name(&ModLoaderType::Forge, "1.16.5", "36.2.39"),
            "v5_Forge_1.16.5_36.2.39.json"
        );
    }
}
