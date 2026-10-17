use crate::minecraft::downloader::{
    download_file_chunked, download_files_concurrent, is_download_task_cached, DownloadTask,
};
use crate::minecraft::instance::{InstanceConfig, ModLoaderType};
use crate::minecraft::loader::{resolve_loader_profile, LoaderProfile};
use crate::minecraft::version::{
    fetch_asset_index, fetch_version_package, should_include_library, LibraryEntry, VersionPackage,
};
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
        ModLoaderType::Vanilla => None,
        ref loader => {
            let loader_ver = instance
                .loader_version
                .as_deref()
                .ok_or(super::models::LaunchError::LoaderVersionNotSelected)?;
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
                loader_ver,
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
                .await
                .map_err(super::models::LaunchError::Download)?;
            }
        }
    }

    // Step 5: Libraries
    let mut all_libraries: Vec<LibraryEntry> = version_pkg
        .libraries
        .iter()
        .filter(|l| should_include_library(l))
        .cloned()
        .collect();

    if let Some(ref prof) = loader_profile {
        all_libraries.extend(prof.libraries.clone());
    }

    let mut library_tasks = Vec::new();
    let mut classpath_entries = Vec::new();

    for lib in &all_libraries {
        if let Some(downloads) = &lib.downloads {
            if let Some(artifact) = &downloads.artifact {
                if let Some(dest) = super::classpath::resolve_library_path(
                    &lib.name,
                    artifact.path.as_deref(),
                    None,
                    libraries_dir,
                ) {
                    classpath_entries.push(dest.clone());
                    library_tasks.push(DownloadTask {
                        url: artifact.url.clone(),
                        dest,
                        sha1: artifact.sha1.clone(),
                        size: artifact.size,
                    });
                }
            }
        } else if let Some(path) = super::classpath::maven_coordinate_to_path(&lib.name, None) {
            let base_url = lib
                .url
                .as_deref()
                .unwrap_or("https://libraries.minecraft.net/");
            let full_url = format!("{}{}", base_url, path);
            let dest = libraries_dir.join(&path);
            classpath_entries.push(dest.clone());
            library_tasks.push(DownloadTask {
                url: full_url,
                dest,
                sha1: None,
                size: None,
            });
        }
    }

    let missing_libs: Vec<DownloadTask> = library_tasks
        .iter()
        .filter(|t| !is_download_task_cached(t))
        .cloned()
        .collect();

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
        download_files_concurrent(client, missing_libs, 16, Some(lib_cb))
            .await
            .map_err(super::models::LaunchError::Download)?;
    }

    // Step 6: Assets
    if let Some(asset_ref) = &version_pkg.asset_index {
        let asset_index = fetch_asset_index(client, assets_dir, asset_ref)
            .await
            .map_err(super::models::LaunchError::Download)?;

        let mut asset_tasks = Vec::new();
        for (_, obj) in asset_index.objects {
            let prefix = obj
                .hash
                .get(..2)
                .filter(|_| {
                    obj.hash.len() == 40 && obj.hash.bytes().all(|byte| byte.is_ascii_hexdigit())
                })
                .ok_or_else(|| super::models::LaunchError::Download("Invalid asset hash".into()))?;
            let dest = assets_dir.join("objects").join(prefix).join(&obj.hash);
            let url = format!(
                "https://resources.download.minecraft.net/{}/{}",
                prefix, obj.hash
            );
            asset_tasks.push(DownloadTask {
                url,
                dest,
                sha1: Some(obj.hash),
                size: Some(obj.size),
            });
        }

        let missing_assets: Vec<DownloadTask> = asset_tasks
            .into_iter()
            .filter(|t| !is_download_task_cached(t))
            .collect();

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
            download_files_concurrent(client, missing_assets, 24, Some(asset_cb))
                .await
                .map_err(super::models::LaunchError::Download)?;
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
        auth,
    })
}

#[cfg(test)]
mod tests;
