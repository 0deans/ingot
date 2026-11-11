use crate::loader::LoaderProfile;
use crate::version::{DownloadArtifact, LibraryEntry, VersionPackage};
use ingot_artifacts::downloader::DownloadTask;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const DEFAULT_LIBRARIES_BASE_URL: &str = "https://libraries.minecraft.net/";

/// Target OS and architecture values used to filter library rules and native classifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LibraryTargetPlatform<'a> {
    pub(super) os: &'a str,
    pub(super) rule_arch: &'a str,
    pub(super) native_arch_bits: &'a str,
}

impl<'a> LibraryTargetPlatform<'a> {
    /// Captures the target platform values for the current host environment.
    pub(super) fn host() -> Self {
        let native_arch_bits = if std::mem::size_of::<usize>() == 8 {
            "64"
        } else {
            "32"
        };
        Self {
            os: super::rules::current_os_name(),
            rule_arch: super::rules::current_arch_name(),
            native_arch_bits,
        }
    }
}

/// Pure plan of library downloads, runtime classpath JARs, and native JAR paths.
pub(super) struct LibraryPlan {
    pub(super) downloads: Vec<DownloadTask>,
    pub(super) classpath_entries: Vec<PathBuf>,
    pub(super) native_jars: Vec<PathBuf>,
}

/// Computes which libraries to download and load without mutating or inspecting disk state.
pub(super) fn build_library_plan(
    version_pkg: &VersionPackage,
    loader_profile: Option<&LoaderProfile>,
    libraries_dir: &Path,
    target: LibraryTargetPlatform<'_>,
) -> LibraryPlan {
    let mut builder = PlanBuilder::new(libraries_dir, target);

    // Loader libraries precede vanilla entries so modded overrides win coordinate precedence.
    let active_libraries = loader_profile
        .into_iter()
        .flat_map(|profile| profile.libraries.iter())
        .chain(version_pkg.libraries.iter())
        .filter(|lib| should_include_for_target(lib, target));

    for lib in active_libraries {
        builder.process_native_library(lib);
        builder.process_classpath_library(lib);
    }

    // Loader maven_files must still be fetched without polluting the classpath or shadowing vanilla.
    if let Some(profile) = loader_profile {
        for lib in profile
            .maven_files
            .iter()
            .filter(|lib| should_include_for_target(lib, target))
        {
            if let Some(task) = builder.resolve_standard_library_task(lib) {
                builder.queue_download(task);
            }
        }
    }

    builder.finish()
}

struct PlanBuilder<'a> {
    libraries_dir: &'a Path,
    target: LibraryTargetPlatform<'a>,
    downloads: Vec<DownloadTask>,
    classpath_entries: Vec<PathBuf>,
    native_jars: Vec<PathBuf>,
    seen_classpath_paths: HashSet<PathBuf>,
    seen_download_dests: HashSet<PathBuf>,
    seen_native_dests: HashSet<PathBuf>,
    seen_maven_keys: HashSet<String>,
}

impl<'a> PlanBuilder<'a> {
    fn new(libraries_dir: &'a Path, target: LibraryTargetPlatform<'a>) -> Self {
        Self {
            libraries_dir,
            target,
            downloads: Vec::new(),
            classpath_entries: Vec::new(),
            native_jars: Vec::new(),
            seen_classpath_paths: HashSet::new(),
            seen_download_dests: HashSet::new(),
            seen_native_dests: HashSet::new(),
            seen_maven_keys: HashSet::new(),
        }
    }

    fn finish(self) -> LibraryPlan {
        LibraryPlan {
            downloads: self.downloads,
            classpath_entries: self.classpath_entries,
            native_jars: self.native_jars,
        }
    }

    fn queue_download(&mut self, task: DownloadTask) {
        if self.seen_download_dests.insert(task.dest.clone()) {
            self.downloads.push(task);
        }
    }

    fn process_native_library(&mut self, lib: &LibraryEntry) {
        let Some(natives_map) = lib.natives.as_ref() else {
            return;
        };
        let Some(classifier_tmpl) = get_native_classifier_template(natives_map, self.target.os)
        else {
            return;
        };

        let classifier = classifier_tmpl.replace("${arch}", self.target.native_arch_bits);
        let classifier_artifact = lib
            .downloads
            .as_ref()
            .and_then(|dl| dl.classifiers.as_ref())
            .and_then(|classifiers| classifiers.get(&classifier));

        let Some(rel_path) = classifier_artifact
            .and_then(|artifact| artifact.path.clone())
            .or_else(|| super::classpath::maven_coordinate_to_path(&lib.name, Some(&classifier)))
        else {
            return;
        };

        let task = build_download_task(
            classifier_artifact,
            &rel_path,
            lib.url.as_deref(),
            self.libraries_dir,
        );
        if self.seen_native_dests.insert(task.dest.clone()) {
            self.native_jars.push(task.dest.clone());
        }
        self.queue_download(task);
    }

    fn process_classpath_library(&mut self, lib: &LibraryEntry) {
        let Some(task) = self.resolve_standard_library_task(lib) else {
            return;
        };

        // Check coordinate identity first so a shadowed vanilla library does not claim a dest slot.
        let should_add = match parse_maven_artifact_key(&lib.name) {
            Some(key) => {
                self.seen_maven_keys.insert(key)
                    && self.seen_classpath_paths.insert(task.dest.clone())
            }
            None => self.seen_classpath_paths.insert(task.dest.clone()),
        };

        if should_add {
            self.classpath_entries.push(task.dest.clone());
            self.queue_download(task);
        }
    }

    fn resolve_standard_library_task(&self, lib: &LibraryEntry) -> Option<DownloadTask> {
        if let Some(downloads) = &lib.downloads {
            let artifact = downloads.artifact.as_ref()?;
            let dest = super::classpath::resolve_library_path(
                &lib.name,
                artifact.path.as_deref(),
                None,
                self.libraries_dir,
            )?;
            return Some(DownloadTask {
                url: artifact.url.clone(),
                dest,
                sha1: artifact.sha1.clone(),
                size: artifact.size,
            });
        }

        let rel_path = super::classpath::maven_coordinate_to_path(&lib.name, None)?;
        Some(build_download_task(
            None,
            &rel_path,
            lib.url.as_deref(),
            self.libraries_dir,
        ))
    }
}

fn should_include_for_target(lib: &LibraryEntry, target: LibraryTargetPlatform<'_>) -> bool {
    match lib.rules.as_deref() {
        Some(rules) => super::rules::check_rules_for_target(rules, target.os, target.rule_arch),
        None => true,
    }
}

fn build_download_task(
    artifact: Option<&DownloadArtifact>,
    rel_path: &str,
    base_url: Option<&str>,
    libraries_dir: &Path,
) -> DownloadTask {
    let dest = libraries_dir.join(rel_path);
    match artifact {
        Some(art) => DownloadTask {
            url: art.url.clone(),
            dest,
            sha1: art.sha1.clone(),
            size: art.size,
        },
        None => DownloadTask {
            url: maven_download_url(base_url, rel_path),
            dest,
            sha1: None,
            size: None,
        },
    }
}

fn maven_download_url(base_url: Option<&str>, rel_path: &str) -> String {
    let base = base_url.unwrap_or(DEFAULT_LIBRARIES_BASE_URL);
    if base.ends_with('/') {
        format!("{base}{rel_path}")
    } else {
        format!("{base}/{rel_path}")
    }
}

fn get_native_classifier_template<'a>(
    natives_map: &'a HashMap<String, String>,
    os_key: &str,
) -> Option<&'a String> {
    natives_map.get(os_key).or_else(|| {
        if os_key == "osx" {
            natives_map.get("macos")
        } else {
            None
        }
    })
}

fn parse_maven_artifact_key(coord: &str) -> Option<String> {
    let (name, _) = coord.split_once('@').unwrap_or((coord, ""));
    let mut parts = name.split(':');
    let group = parts.next()?;
    let artifact = parts.next()?;
    let _version = parts.next();
    let classifier = parts.next().unwrap_or("");
    Some(format!("{group}:{artifact}:{classifier}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launcher::rules::{OsRule, RuleEntry};
    use crate::version::LibraryDownloads;

    const WINDOWS_X64: LibraryTargetPlatform<'static> = LibraryTargetPlatform {
        os: "windows",
        rule_arch: "x64",
        native_arch_bits: "64",
    };

    fn artifact_lib(name: &str, path: &str, url: &str) -> LibraryEntry {
        LibraryEntry {
            name: name.to_string(),
            downloads: Some(LibraryDownloads {
                artifact: Some(DownloadArtifact {
                    path: Some(path.to_string()),
                    sha1: Some("deadbeef".to_string()),
                    size: Some(42),
                    url: url.to_string(),
                }),
                classifiers: None,
            }),
            rules: None,
            natives: None,
            url: None,
        }
    }

    fn minimal_package(libraries: Vec<LibraryEntry>) -> VersionPackage {
        VersionPackage {
            id: "test-mc".to_string(),
            downloads: None,
            libraries,
            asset_index: None,
            main_class: "net.minecraft.client.main.Main".to_string(),
            arguments: None,
            minecraft_arguments: None,
            java_version: None,
        }
    }

    fn minimal_loader(
        libraries: Vec<LibraryEntry>,
        maven_files: Vec<LibraryEntry>,
    ) -> LoaderProfile {
        LoaderProfile {
            id: "loader-test".to_string(),
            main_class: "loader.Main".to_string(),
            libraries,
            maven_files,
            jvm_args: Vec::new(),
            game_args: Vec::new(),
            minecraft_arguments: None,
        }
    }

    #[test]
    fn loader_precedes_and_shadows_vanilla_by_maven_key() {
        let libs_dir = Path::new("/libs");
        let loader = minimal_loader(
            vec![
                artifact_lib(
                    "org.apache.logging.log4j:log4j-api:2.15.0",
                    "org/apache/logging/log4j/log4j-api/2.15.0/log4j-api-2.15.0.jar",
                    "https://example.invalid/log4j-api-2.15.0.jar",
                ),
                artifact_lib(
                    "com.example:shared:1.0:api",
                    "com/example/shared/1.0/shared-1.0-api.jar",
                    "https://example.invalid/shared-api.jar",
                ),
            ],
            Vec::new(),
        );
        let package = minimal_package(vec![
            artifact_lib(
                "org.apache.logging.log4j:log4j-api:2.8.1",
                "org/apache/logging/log4j/log4j-api/2.8.1/log4j-api-2.8.1.jar",
                "https://example.invalid/log4j-api-2.8.1.jar",
            ),
            artifact_lib(
                "com.example:shared:1.0:impl",
                "com/example/shared/1.0/shared-1.0-impl.jar",
                "https://example.invalid/shared-impl.jar",
            ),
        ]);

        let plan = build_library_plan(&package, Some(&loader), libs_dir, WINDOWS_X64);

        assert_eq!(
            plan.classpath_entries,
            vec![
                libs_dir.join("org/apache/logging/log4j/log4j-api/2.15.0/log4j-api-2.15.0.jar"),
                libs_dir.join("com/example/shared/1.0/shared-1.0-api.jar"),
                libs_dir.join("com/example/shared/1.0/shared-1.0-impl.jar"),
            ]
        );
        let urls: Vec<&str> = plan.downloads.iter().map(|t| t.url.as_str()).collect();
        assert_eq!(
            urls,
            vec![
                "https://example.invalid/log4j-api-2.15.0.jar",
                "https://example.invalid/shared-api.jar",
                "https://example.invalid/shared-impl.jar",
            ]
        );
    }

    #[test]
    fn deduplicates_identical_destinations_and_unparseable_names() {
        let libs_dir = Path::new("/libs");
        let package = minimal_package(vec![
            artifact_lib(
                "com.example:first:1.0",
                "shared/same.jar",
                "https://example.invalid/first.jar",
            ),
            artifact_lib(
                "com.example:second:1.0",
                "shared/same.jar",
                "https://example.invalid/second.jar",
            ),
            artifact_lib(
                "unparseable-a",
                "shared/custom.jar",
                "https://example.invalid/custom-a.jar",
            ),
            artifact_lib(
                "unparseable-b",
                "shared/custom.jar",
                "https://example.invalid/custom-b.jar",
            ),
        ]);

        let plan = build_library_plan(&package, None, libs_dir, WINDOWS_X64);

        assert_eq!(
            plan.classpath_entries,
            vec![
                libs_dir.join("shared/same.jar"),
                libs_dir.join("shared/custom.jar"),
            ]
        );
        assert_eq!(plan.downloads.len(), 2);
        assert_eq!(plan.downloads[0].url, "https://example.invalid/first.jar");
        assert_eq!(
            plan.downloads[1].url,
            "https://example.invalid/custom-a.jar"
        );
    }

    #[test]
    fn evaluates_os_and_arch_rules_for_explicit_target_platform() {
        let libs_dir = Path::new("/libs");
        let mac_arm_only = LibraryEntry {
            rules: Some(vec![RuleEntry {
                action: "allow".to_string(),
                os: Some(OsRule {
                    name: Some("osx".to_string()),
                    version: None,
                    arch: Some("arm64".to_string()),
                }),
            }]),
            ..artifact_lib(
                "org.lwjgl:lwjgl:3.3.3",
                "org/lwjgl/lwjgl/3.3.3/lwjgl-arm64.jar",
                "https://example.invalid/lwjgl-arm64.jar",
            )
        };
        let non_windows = LibraryEntry {
            rules: Some(vec![
                RuleEntry {
                    action: "allow".to_string(),
                    os: None,
                },
                RuleEntry {
                    action: "disallow".to_string(),
                    os: Some(OsRule {
                        name: Some("windows".to_string()),
                        version: None,
                        arch: None,
                    }),
                },
            ]),
            ..artifact_lib(
                "net.java.dev.jna:jna:5.13.0",
                "net/java/dev/jna/jna/5.13.0/jna-5.13.0.jar",
                "https://example.invalid/jna.jar",
            )
        };
        let package = minimal_package(vec![mac_arm_only, non_windows]);

        let osx_arm64 = LibraryTargetPlatform {
            os: "osx",
            rule_arch: "arm64",
            native_arch_bits: "64",
        };
        let osx_plan = build_library_plan(&package, None, libs_dir, osx_arm64);
        assert_eq!(
            osx_plan.classpath_entries,
            vec![
                libs_dir.join("org/lwjgl/lwjgl/3.3.3/lwjgl-arm64.jar"),
                libs_dir.join("net/java/dev/jna/jna/5.13.0/jna-5.13.0.jar"),
            ]
        );

        let win_plan = build_library_plan(&package, None, libs_dir, WINDOWS_X64);
        assert!(win_plan.classpath_entries.is_empty());
        assert!(win_plan.downloads.is_empty());
    }

    #[test]
    fn native_classifiers_support_osx_macos_fallback_and_arch_bits() {
        let libs_dir = Path::new("/libs");
        let mut classifiers = HashMap::new();
        classifiers.insert(
            "natives-macos-32".to_string(),
            DownloadArtifact {
                path: Some(
                    "org/lwjgl/lwjgl-platform/2.9.4/lwjgl-platform-2.9.4-natives-macos-32.jar"
                        .to_string(),
                ),
                sha1: Some("abc".to_string()),
                size: Some(99),
                url: "https://example.invalid/natives-macos-32.jar".to_string(),
            },
        );
        let macos_fallback_native = LibraryEntry {
            name: "org.lwjgl.lwjgl:lwjgl-platform:2.9.4".to_string(),
            downloads: Some(LibraryDownloads {
                artifact: None,
                classifiers: Some(classifiers),
            }),
            rules: None,
            natives: Some(HashMap::from([(
                "macos".to_string(),
                "natives-macos-${arch}".to_string(),
            )])),
            url: None,
        };

        let maven_fallback_native = LibraryEntry {
            name: "com.example:native-maven:1.2.3".to_string(),
            downloads: None,
            rules: None,
            natives: Some(HashMap::from([
                ("osx".to_string(), "natives-osx-${arch}".to_string()),
                ("macos".to_string(), "natives-ignored".to_string()),
            ])),
            url: Some("https://repo.example.invalid".to_string()),
        };

        let package = minimal_package(vec![macos_fallback_native, maven_fallback_native]);
        let osx_32 = LibraryTargetPlatform {
            os: "osx",
            rule_arch: "x86",
            native_arch_bits: "32",
        };

        let plan = build_library_plan(&package, None, libs_dir, osx_32);

        assert_eq!(
            plan.native_jars,
            vec![
                libs_dir.join(
                    "org/lwjgl/lwjgl-platform/2.9.4/lwjgl-platform-2.9.4-natives-macos-32.jar"
                ),
                libs_dir
                    .join("com/example/native-maven/1.2.3/native-maven-1.2.3-natives-osx-32.jar"),
            ]
        );
        // The first native library has `downloads.artifact: None`, so only the second appears as a Maven JAR.
        assert_eq!(
            plan.classpath_entries,
            vec![libs_dir.join("com/example/native-maven/1.2.3/native-maven-1.2.3.jar")]
        );
        assert_eq!(plan.downloads.len(), 3);
        assert_eq!(
            plan.downloads[0].url,
            "https://example.invalid/natives-macos-32.jar"
        );
        assert_eq!(
            plan.downloads[1].url,
            "https://repo.example.invalid/com/example/native-maven/1.2.3/native-maven-1.2.3-natives-osx-32.jar"
        );
        assert_eq!(
            plan.downloads[2].url,
            "https://repo.example.invalid/com/example/native-maven/1.2.3/native-maven-1.2.3.jar"
        );
    }

    #[test]
    fn maven_files_skip_classpath_and_do_not_shadow_vanilla() {
        let libs_dir = Path::new("/libs");
        let loader = minimal_loader(
            Vec::new(),
            vec![
                artifact_lib(
                    "com.google.guava:guava:20.0",
                    "com/google/guava/guava/20.0/guava-20.0.jar",
                    "https://example.invalid/guava-20.0.jar",
                ),
                artifact_lib(
                    "com.google.guava:guava-dup:20.0",
                    "com/google/guava/guava/20.0/guava-20.0.jar",
                    "https://example.invalid/guava-20.0-dup.jar",
                ),
            ],
        );
        let package = minimal_package(vec![artifact_lib(
            "com.google.guava:guava:21.0",
            "com/google/guava/guava/21.0/guava-21.0.jar",
            "https://example.invalid/guava-21.0.jar",
        )]);

        let plan = build_library_plan(&package, Some(&loader), libs_dir, WINDOWS_X64);

        assert_eq!(
            plan.classpath_entries,
            vec![libs_dir.join("com/google/guava/guava/21.0/guava-21.0.jar")]
        );
        assert!(plan.native_jars.is_empty());
        let urls: Vec<&str> = plan.downloads.iter().map(|t| t.url.as_str()).collect();
        assert_eq!(
            urls,
            vec![
                "https://example.invalid/guava-21.0.jar",
                "https://example.invalid/guava-20.0.jar",
            ]
        );
    }

    #[test]
    fn non_artifact_maven_libraries_build_urls_with_and_without_trailing_slash() {
        let libs_dir = Path::new("/libs");
        let default_url_lib = LibraryEntry {
            name: "net.fabricmc:tiny-mappings-parser:0.3.0".to_string(),
            downloads: None,
            rules: None,
            natives: None,
            url: None,
        };
        let trailing_slash_lib = LibraryEntry {
            name: "net.fabricmc:sponge-mixin:0.12.5".to_string(),
            downloads: None,
            rules: None,
            natives: None,
            url: Some("https://maven.fabricmc.net/".to_string()),
        };
        let no_trailing_slash_lib = LibraryEntry {
            name: "net.minecraftforge:unsafe:0.9.2".to_string(),
            downloads: None,
            rules: None,
            natives: None,
            url: Some("https://maven.minecraftforge.net".to_string()),
        };
        let empty_downloads_lib = LibraryEntry {
            name: "com.example:artifactless:1.0".to_string(),
            downloads: Some(LibraryDownloads {
                artifact: None,
                classifiers: None,
            }),
            rules: None,
            natives: None,
            url: Some("https://ignored.example.invalid".to_string()),
        };

        let package = minimal_package(vec![
            default_url_lib,
            trailing_slash_lib,
            no_trailing_slash_lib,
            empty_downloads_lib,
        ]);

        let plan = build_library_plan(&package, None, libs_dir, WINDOWS_X64);

        assert_eq!(
            plan.classpath_entries,
            vec![
                libs_dir
                    .join("net/fabricmc/tiny-mappings-parser/0.3.0/tiny-mappings-parser-0.3.0.jar"),
                libs_dir.join("net/fabricmc/sponge-mixin/0.12.5/sponge-mixin-0.12.5.jar"),
                libs_dir.join("net/minecraftforge/unsafe/0.9.2/unsafe-0.9.2.jar"),
            ]
        );
        let urls: Vec<&str> = plan.downloads.iter().map(|t| t.url.as_str()).collect();
        assert_eq!(
            urls,
            vec![
                "https://libraries.minecraft.net/net/fabricmc/tiny-mappings-parser/0.3.0/tiny-mappings-parser-0.3.0.jar",
                "https://maven.fabricmc.net/net/fabricmc/sponge-mixin/0.12.5/sponge-mixin-0.12.5.jar",
                "https://maven.minecraftforge.net/net/minecraftforge/unsafe/0.9.2/unsafe-0.9.2.jar",
            ]
        );
        assert!(plan
            .downloads
            .iter()
            .all(|t| t.sha1.is_none() && t.size.is_none()));
    }
}
