use crate::minecraft::downloader::{
    download_file_chunked, download_files_concurrent, is_download_task_cached, DownloadError,
    DownloadTask,
};
use crate::minecraft::instance::{InstanceConfig, ModLoaderType};
use crate::minecraft::loader::{resolve_loader_profile, LoaderProfile};
use crate::minecraft::version::{
    fetch_asset_index, fetch_version_package, should_include_library, LibraryEntry, VersionPackage,
};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub type PreparationFuture<'a, T> = std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<T, super::models::LaunchError>> + Send + 'a>,
>;

pub trait PreparationEffects: Sync {
    fn ensure_java<'a>(
        &'a self,
        client: &'a reqwest::Client,
        major_version: u32,
        custom_path: Option<&'a str>,
        progress: Option<crate::minecraft::java::JavaProgressFn>,
    ) -> PreparationFuture<'a, PathBuf>;
    fn resolve_auth(&self) -> PreparationFuture<'_, ResolvedAuth>;
}

pub use super::authlib::setup_ely_authlib_agent;

pub struct ResolvedAuth {
    pub player_name: String,
    pub uuid_str: String,
    pub access_token: String,
    pub user_type: &'static str,
    pub xuid: Option<String>,
    pub is_ely: bool,
}

pub struct LaunchPreparation {
    pub version_pkg: VersionPackage,
    pub loader_profile: Option<LoaderProfile>,
    pub java_bin: PathBuf,
    pub client_jar_dest: PathBuf,
    pub classpath_entries: Vec<PathBuf>,
    pub native_jars: Vec<PathBuf>,
    pub auth: ResolvedAuth,
}

use super::progress::ProgressCallback;

pub async fn prepare_launch_environment(
    effects: &impl PreparationEffects,
    client: &reqwest::Client,
    instance: &InstanceConfig,
    cache_dir: &Path,
    assets_dir: &Path,
    libraries_dir: &Path,
    report_prog: ProgressCallback,
) -> Result<LaunchPreparation, super::models::LaunchError> {
    fs::create_dir_all(cache_dir)?;
    fs::create_dir_all(assets_dir)?;
    fs::create_dir_all(libraries_dir)?;

    // Step 1: Version Package
    report_prog("Resolving Version", 1, 10, "Fetching version metadata...");
    let version_pkg = fetch_version_package(client, cache_dir, &instance.game_version)
        .await
        .map_err(super::models::LaunchError::VersionPackage)?;

    // Step 2: Loader Profile (if modded)
    let loader_profile: Option<LoaderProfile> = match instance.loader {
        ModLoaderType::Vanilla | ModLoaderType::Horizon => None,
        ref loader => {
            let resolved_version = match instance.loader_version.as_deref().map(str::trim) {
                Some(v) if !v.is_empty() => v.to_string(),
                _ => {
                    let versions = crate::minecraft::loader::fetch_loader_versions(
                        client,
                        loader,
                        &instance.game_version,
                    )
                    .await
                    .map_err(|_| super::models::LaunchError::LoaderVersionNotSelected)?;
                    versions
                        .into_iter()
                        .next()
                        .ok_or(super::models::LaunchError::LoaderVersionNotSelected)?
                }
            };
            report_prog(
                "Resolving Mod Loader",
                2,
                10,
                &format!("Fetching {:?} profile...", loader),
            );
            let profile = resolve_loader_profile(
                client,
                cache_dir,
                loader,
                &instance.game_version,
                &resolved_version,
            )
            .await
            .map_err(super::models::LaunchError::LoaderProfile)?;
            Some(profile)
        }
    };

    // Step 3: Java Runtime
    report_prog(
        "Checking Java Runtime",
        3,
        10,
        "Detecting Java environment...",
    );
    let required_java = version_pkg
        .java_version
        .as_ref()
        .filter(|j| j.major_version > 0)
        .map(|j| j.major_version)
        .unwrap_or_else(|| {
            crate::minecraft::java::get_required_java_version(&instance.game_version, None)
        });

    let java_prog = {
        let rp = report_prog.clone();
        Arc::new(move |cur: u64, tot: u64, det: &str| {
            rp("Java Setup", cur, tot, det);
        })
    };

    let java_bin = effects
        .ensure_java(
            client,
            required_java,
            instance.java_path.as_deref(),
            Some(java_prog),
        )
        .await?;

    // Step 4: Client Jar
    let client_jar_dest = cache_dir
        .join("versions")
        .join(&instance.game_version)
        .join(format!("{}.jar", instance.game_version));

    if let Some(downloads) = &version_pkg.downloads {
        if let Some(client_artifact) = &downloads.client {
            let client_task = DownloadTask {
                url: client_artifact.url.clone(),
                dest: client_jar_dest.clone(),
                sha1: client_artifact.sha1.clone(),
                size: client_artifact.size,
            };

            if !is_download_task_cached(&client_task) {
                report_prog(
                    "Downloading Client",
                    4,
                    10,
                    "Downloading Minecraft client jar...",
                );
                let dl_cb = {
                    let rp = report_prog.clone();
                    Arc::new(move |cur: u64, tot: u64| {
                        rp(
                            "Downloading Client",
                            cur,
                            tot,
                            &format!(
                                "client.jar ({:.1}MB / {:.1}MB)",
                                cur as f64 / 1_048_576.0,
                                tot as f64 / 1_048_576.0
                            ),
                        );
                    })
                };
                download_file_chunked(
                    client,
                    &client_artifact.url,
                    &client_jar_dest,
                    client_artifact.sha1.as_deref(),
                    Some(dl_cb),
                )
                .await?;
            }
        }
    }

    // Step 5: Libraries
    let mut all_libraries: Vec<LibraryEntry> = Vec::new();

    if let Some(ref prof) = loader_profile {
        all_libraries.extend(
            prof.libraries
                .iter()
                .filter(|l| should_include_library(l))
                .cloned(),
        );
    }

    all_libraries.extend(
        version_pkg
            .libraries
            .iter()
            .filter(|l| should_include_library(l))
            .cloned(),
    );

    let mut library_tasks = Vec::new();
    let mut classpath_entries = Vec::new();
    let mut native_jars = Vec::new();
    let mut seen_classpath_paths = HashSet::new();
    let mut seen_download_dests = HashSet::new();
    let mut seen_native_dests = HashSet::new();
    let mut seen_maven_keys = HashSet::new();

    let os_key = current_os_key();
    let arch_str = if std::mem::size_of::<usize>() == 8 {
        "64"
    } else {
        "32"
    };

    for lib in &all_libraries {
        if let Some(ref natives_map) = lib.natives {
            if let Some(classifier_tmpl) = get_native_classifier_template(natives_map, os_key) {
                let classifier = classifier_tmpl.replace("${arch}", arch_str);
                let (classifier_artifact, rel_path) = if let Some(ref dl) = lib.downloads {
                    let art = dl.classifiers.as_ref().and_then(|c| c.get(&classifier));
                    let path = art.and_then(|a| a.path.clone()).or_else(|| {
                        super::classpath::maven_coordinate_to_path(&lib.name, Some(&classifier))
                    });
                    (art, path)
                } else {
                    let path =
                        super::classpath::maven_coordinate_to_path(&lib.name, Some(&classifier));
                    (None, path)
                };

                if let Some(path) = rel_path {
                    let dest = libraries_dir.join(&path);
                    if seen_native_dests.insert(dest.clone()) {
                        native_jars.push(dest.clone());
                    }
                    if seen_download_dests.insert(dest.clone()) {
                        let (url, sha1, size) = if let Some(art) = classifier_artifact {
                            (art.url.clone(), art.sha1.clone(), art.size)
                        } else {
                            let base_url = lib
                                .url
                                .as_deref()
                                .unwrap_or("https://libraries.minecraft.net/");
                            let full_url = if base_url.ends_with('/') {
                                format!("{}{}", base_url, path)
                            } else {
                                format!("{}/{}", base_url, path)
                            };
                            (full_url, None, None)
                        };
                        library_tasks.push(DownloadTask {
                            url,
                            dest,
                            sha1,
                            size,
                        });
                    }
                }
            }
        }

        if let Some(downloads) = &lib.downloads {
            if let Some(artifact) = &downloads.artifact {
                if let Some(dest) = super::classpath::resolve_library_path(
                    &lib.name,
                    artifact.path.as_deref(),
                    None,
                    libraries_dir,
                ) {
                    let should_add = match parse_maven_artifact_key(&lib.name) {
                        Some(key) => {
                            seen_maven_keys.insert(key) && seen_classpath_paths.insert(dest.clone())
                        }
                        None => seen_classpath_paths.insert(dest.clone()),
                    };
                    if should_add {
                        classpath_entries.push(dest.clone());
                    }
                    if should_add && seen_download_dests.insert(dest.clone()) {
                        library_tasks.push(DownloadTask {
                            url: artifact.url.clone(),
                            dest,
                            sha1: artifact.sha1.clone(),
                            size: artifact.size,
                        });
                    }
                }
            }
        } else if let Some(path) = super::classpath::maven_coordinate_to_path(&lib.name, None) {
            let base_url = lib
                .url
                .as_deref()
                .unwrap_or("https://libraries.minecraft.net/");
            let full_url = if base_url.ends_with('/') {
                format!("{}{}", base_url, path)
            } else {
                format!("{}/{}", base_url, path)
            };
            let dest = libraries_dir.join(&path);
            let should_add = match parse_maven_artifact_key(&lib.name) {
                Some(key) => {
                    seen_maven_keys.insert(key) && seen_classpath_paths.insert(dest.clone())
                }
                None => seen_classpath_paths.insert(dest.clone()),
            };
            if should_add {
                classpath_entries.push(dest.clone());
            }
            if should_add && seen_download_dests.insert(dest.clone()) {
                library_tasks.push(DownloadTask {
                    url: full_url,
                    dest,
                    sha1: None,
                    size: None,
                });
            }
        }
    }

    if let Some(ref prof) = loader_profile {
        for lib in prof
            .maven_files
            .iter()
            .filter(|l| should_include_library(l))
        {
            if let Some(downloads) = &lib.downloads {
                if let Some(artifact) = &downloads.artifact {
                    if let Some(dest) = super::classpath::resolve_library_path(
                        &lib.name,
                        artifact.path.as_deref(),
                        None,
                        libraries_dir,
                    ) {
                        if seen_download_dests.insert(dest.clone()) {
                            library_tasks.push(DownloadTask {
                                url: artifact.url.clone(),
                                dest,
                                sha1: artifact.sha1.clone(),
                                size: artifact.size,
                            });
                        }
                    }
                }
            } else if let Some(path) = super::classpath::maven_coordinate_to_path(&lib.name, None) {
                let dest = libraries_dir.join(&path);
                if seen_download_dests.insert(dest.clone()) {
                    let base_url = lib
                        .url
                        .as_deref()
                        .unwrap_or("https://libraries.minecraft.net/");
                    let full_url = if base_url.ends_with('/') {
                        format!("{}{}", base_url, path)
                    } else {
                        format!("{}/{}", base_url, path)
                    };
                    library_tasks.push(DownloadTask {
                        url: full_url,
                        dest,
                        sha1: None,
                        size: None,
                    });
                }
            }
        }
    }

    let missing_libs: Vec<DownloadTask> = filter_uncached_tasks(library_tasks).await;

    if !missing_libs.is_empty() {
        let count = missing_libs.len();
        report_prog(
            "Downloading Libraries",
            5,
            10,
            &format!("Downloading {} libraries...", count),
        );
        let lib_cb = {
            let rp = report_prog.clone();
            Arc::new(move |cur: usize, tot: usize, file_name: &str| {
                rp(
                    "Downloading Libraries",
                    cur as u64,
                    tot as u64,
                    &format!("Library {}/{} ({})", cur, tot, file_name),
                );
            })
        };
        download_files_concurrent(client, missing_libs, 16, Some(lib_cb)).await?;
    }

    // Step 6: Assets
    if let Some(asset_ref) = &version_pkg.asset_index {
        let asset_index = fetch_asset_index(client, assets_dir, asset_ref)
            .await
            .map_err(super::models::LaunchError::VersionPackage)?;

        let mut asset_tasks = Vec::new();
        let mut seen_asset_dests = HashSet::new();
        for (_, obj) in asset_index.objects {
            let task = prepare_asset_task(assets_dir, obj)?;
            if seen_asset_dests.insert(task.dest.clone()) {
                asset_tasks.push(task);
            }
        }

        let missing_assets: Vec<DownloadTask> = filter_uncached_tasks(asset_tasks).await;

        if !missing_assets.is_empty() {
            let count = missing_assets.len();
            report_prog(
                "Downloading Assets",
                6,
                10,
                &format!("Downloading {} assets...", count),
            );
            let asset_cb = {
                let rp = report_prog.clone();
                Arc::new(move |cur: usize, tot: usize, _: &str| {
                    rp(
                        "Downloading Assets",
                        cur as u64,
                        tot as u64,
                        &format!("Assets {}/{}", cur, tot),
                    );
                })
            };
            download_files_concurrent(client, missing_assets, 24, Some(asset_cb)).await?;
        }
    }

    report_prog("Verifying Files", 7, 10, "All game files verified");

    // Step 7: Resolve active account and auth
    let auth = effects.resolve_auth().await?;

    Ok(LaunchPreparation {
        version_pkg,
        loader_profile,
        java_bin,
        client_jar_dest,
        classpath_entries,
        native_jars,
        auth,
    })
}

fn prepare_asset_task(
    assets_dir: &Path,
    object: crate::minecraft::version::AssetObject,
) -> Result<DownloadTask, DownloadError> {
    if object.hash.len() != 40 {
        return Err(DownloadError::InvalidAssetHash);
    }
    if !object.hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(DownloadError::InvalidAssetHash);
    }
    let prefix = object
        .hash
        .get(..2)
        .ok_or(DownloadError::InvalidAssetHash)?;
    let destination = assets_dir.join("objects").join(prefix).join(&object.hash);
    let url = format!(
        "https://resources.download.minecraft.net/{}/{}",
        prefix, object.hash
    );
    Ok(DownloadTask {
        url,
        dest: destination,
        sha1: Some(object.hash),
        size: Some(object.size),
    })
}

async fn filter_uncached_tasks(tasks: Vec<DownloadTask>) -> Vec<DownloadTask> {
    if tasks.is_empty() {
        return Vec::new();
    }
    if tasks.len() <= 32 {
        return tasks
            .into_iter()
            .filter(|t| !is_download_task_cached(t))
            .collect();
    }
    let worker_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(2, 16);
    let chunk_size = (tasks.len() / worker_count).max(32);
    let chunks: Vec<Vec<DownloadTask>> = tasks
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect();

    let mut handles = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        handles.push(tokio::task::spawn_blocking(move || {
            chunk
                .into_iter()
                .filter(|t| !is_download_task_cached(t))
                .collect::<Vec<_>>()
        }));
    }

    let mut missing = Vec::new();
    for handle in handles {
        if let Ok(chunk_missing) = handle.await {
            missing.extend(chunk_missing);
        }
    }
    missing
}

fn current_os_key() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(target_os = "macos")]
    {
        "osx"
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        "linux"
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
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 2 {
        return None;
    }
    let group = parts[0];
    let artifact = parts[1];
    let classifier = parts.get(3).copied().unwrap_or("");
    Some(format!("{}:{}:{}", group, artifact, classifier))
}

#[cfg(test)]
mod tests;
