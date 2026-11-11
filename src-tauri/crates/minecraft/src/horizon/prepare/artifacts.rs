use super::super::models::HorizonError;
use crate::artifacts::downloader::{
    download_file_chunked, download_files_concurrent, DownloadTask,
};
use crate::minecraft::launcher::classpath::{maven_coordinate_to_path, resolve_library_path};
use crate::minecraft::version::{
    fetch_asset_index, should_include_library, AssetObject, VersionPackage,
};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn link_or_copy_if_missing(source: &Path, destination: &Path) -> bool {
    if destination.is_file() || !source.is_file() {
        return false;
    }
    if fs::hard_link(source, destination).is_err() {
        let _ = fs::copy(source, destination);
    }
    true
}

pub(super) async fn ensure_pinned_manifest(
    client: &reqwest::Client,
    preparer_root: &Path,
    version: &str,
    manifest_sha1: &str,
) -> Result<PathBuf, HorizonError> {
    let manifest_path = preparer_root
        .join(".local")
        .join("minecraft")
        .join(version)
        .join("manifest.json");
    if manifest_path.is_file() {
        return Ok(manifest_path);
    }

    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let url = format!("https://piston-meta.mojang.com/v1/packages/{manifest_sha1}/{version}.json");
    let bytes = client
        .get(&url)
        .header("User-Agent", "Ingot-Launcher")
        .send()
        .await?
        .error_for_status()
        .map_err(HorizonError::Transport)?
        .bytes()
        .await?;

    crate::core::write_file_atomic(&manifest_path, &bytes)?;
    Ok(manifest_path)
}

pub(super) async fn ensure_server_jar<FProg>(
    client: &reqwest::Client,
    data_dir: &Path,
    version: &str,
    server_sha1: &str,
    report_prog: Arc<FProg>,
) -> Result<PathBuf, HorizonError>
where
    FProg: Fn(&str, usize, usize, &str) + Send + Sync + 'static,
{
    let server_jars_dir = data_dir.join("cache").join("server-jars");
    let server_jar_path = server_jars_dir.join(format!("{version}.jar"));
    if server_jar_path.is_file() {
        return Ok(server_jar_path);
    }

    fs::create_dir_all(&server_jars_dir)?;

    let url = format!("https://piston-data.mojang.com/v1/objects/{server_sha1}/server.jar");

    let dl_cb = {
        let rp = report_prog.clone();
        Arc::new(move |cur: u64, total: u64| {
            let detail = format!(
                "server.jar ({:.1}MB / {:.1}MB)",
                cur as f64 / 1_048_576.0,
                total as f64 / 1_048_576.0
            );
            let percent = (cur * 100).checked_div(total).unwrap_or(0) as usize;
            rp("Downloading Server", percent, 100, &detail);
        })
    };

    download_file_chunked(
        client,
        &url,
        &server_jar_path,
        Some(server_sha1),
        Some(dl_cb),
    )
    .await
    .map_err(|e| HorizonError::Preparation(format!("Failed to download server jar: {e}")))?;

    Ok(server_jar_path)
}

pub(super) async fn ensure_minecraft_files<FProg>(
    client: &reqwest::Client,
    data_dir: &Path,
    version: &str,
    client_sha1: &str,
    pinned_manifest_path: &Path,
    report_prog: Arc<FProg>,
) -> Result<(), HorizonError>
where
    FProg: Fn(&str, usize, usize, &str) + Send + Sync + 'static,
{
    let cache_version_dir = data_dir.join("cache").join("versions").join(version);
    let cached_jar = cache_version_dir.join(format!("{version}.jar"));

    if !cached_jar.is_file() {
        fs::create_dir_all(&cache_version_dir)?;
        let url = format!("https://piston-data.mojang.com/v1/objects/{client_sha1}/client.jar");

        let dl_cb = {
            let rp = report_prog.clone();
            Arc::new(move |cur: u64, total: u64| {
                let detail = format!(
                    "client.jar ({:.1}MB / {:.1}MB)",
                    cur as f64 / 1_048_576.0,
                    total as f64 / 1_048_576.0
                );
                let percent = (cur * 100).checked_div(total).unwrap_or(0) as usize;
                rp("Downloading Client", percent, 100, &detail);
            })
        };

        download_file_chunked(client, &url, &cached_jar, Some(client_sha1), Some(dl_cb))
            .await
            .map_err(|e| {
                HorizonError::Preparation(format!("Failed to download client jar: {e}"))
            })?;
    }

    let mc_version_dir = data_dir.join("versions").join(version);
    fs::create_dir_all(&mc_version_dir)?;

    let target_jar = mc_version_dir.join(format!("{version}.jar"));
    link_or_copy_if_missing(&cached_jar, &target_jar);

    let target_json = mc_version_dir.join(format!("{version}.json"));
    if !target_json.is_file() {
        let cached_json = cache_version_dir.join(format!("{version}.json"));
        if !link_or_copy_if_missing(&cached_json, &target_json) {
            link_or_copy_if_missing(pinned_manifest_path, &target_json);
        }
    }

    ensure_minecraft_libraries(client, data_dir, pinned_manifest_path, report_prog.clone()).await?;
    ensure_minecraft_assets(client, data_dir, pinned_manifest_path, report_prog).await?;

    Ok(())
}

pub(crate) fn collect_asset_tasks(
    assets_dir: &Path,
    objects: &HashMap<String, AssetObject>,
) -> Vec<DownloadTask> {
    let mut tasks = Vec::with_capacity(objects.len());
    let mut seen_hashes = HashSet::new();
    let objects_dir = assets_dir.join("objects");

    for obj in objects.values() {
        if obj.hash.len() != 40 || !obj.hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            continue;
        }
        if !seen_hashes.insert(&obj.hash) {
            continue;
        }
        let prefix = &obj.hash[..2];
        let dest = objects_dir.join(prefix).join(&obj.hash);
        let url = format!(
            "https://resources.download.minecraft.net/{}/{}",
            prefix, obj.hash
        );
        tasks.push(DownloadTask {
            url,
            dest,
            sha1: Some(obj.hash.clone()),
            size: Some(obj.size),
        });
    }

    tasks
}

async fn ensure_minecraft_assets<FProg>(
    client: &reqwest::Client,
    data_dir: &Path,
    pinned_manifest_path: &Path,
    report_prog: Arc<FProg>,
) -> Result<(), HorizonError>
where
    FProg: Fn(&str, usize, usize, &str) + Send + Sync + 'static,
{
    let manifest_bytes = fs::read(pinned_manifest_path)?;
    let version_pkg: VersionPackage = serde_json::from_slice(&manifest_bytes)?;

    let Some(asset_ref) = version_pkg.asset_index else {
        return Ok(());
    };

    let assets_dir = data_dir.join("assets");
    fs::create_dir_all(&assets_dir)?;

    report_prog(
        "Downloading Assets",
        0,
        100,
        "Fetching Minecraft asset index...",
    );
    let asset_index = fetch_asset_index(client, &assets_dir, &asset_ref)
        .await
        .map_err(|e| HorizonError::Preparation(format!("Failed to fetch asset index: {e}")))?;

    let tasks = collect_asset_tasks(&assets_dir, &asset_index.objects);
    if tasks.is_empty() {
        return Ok(());
    }

    let dl_cb = {
        let rp = report_prog.clone();
        Arc::new(move |cur: usize, tot: usize, file_name: &str| {
            let detail = format!("Asset {cur}/{tot} ({file_name})");
            let percent = (cur * 100).checked_div(tot).unwrap_or(0);
            rp("Downloading Assets", percent, 100, &detail);
        })
    };

    download_files_concurrent(client, tasks, 24, Some(dl_cb))
        .await
        .map_err(|e| HorizonError::Preparation(format!("Failed to download assets: {e}")))?;

    Ok(())
}

pub(crate) fn collect_library_tasks(
    pkg: &VersionPackage,
    libraries_dir: &Path,
) -> Vec<DownloadTask> {
    let mut tasks = Vec::new();
    let mut seen_dests = HashSet::new();

    for lib in &pkg.libraries {
        if !should_include_library(lib) {
            continue;
        }

        if let Some(downloads) = &lib.downloads {
            if let Some(artifact) = &downloads.artifact {
                if let Some(dest) =
                    resolve_library_path(&lib.name, artifact.path.as_deref(), None, libraries_dir)
                {
                    if seen_dests.insert(dest.clone()) {
                        tasks.push(DownloadTask {
                            url: artifact.url.clone(),
                            dest,
                            sha1: artifact.sha1.clone(),
                            size: artifact.size,
                        });
                    }
                }
            }
        } else if let Some(path) = maven_coordinate_to_path(&lib.name, None) {
            let base_url = lib
                .url
                .as_deref()
                .unwrap_or("https://libraries.minecraft.net/");
            let full_url = format!("{base_url}{path}");
            let dest = libraries_dir.join(&path);
            if seen_dests.insert(dest.clone()) {
                tasks.push(DownloadTask {
                    url: full_url,
                    dest,
                    sha1: None,
                    size: None,
                });
            }
        }
    }

    tasks
}

async fn ensure_minecraft_libraries<FProg>(
    client: &reqwest::Client,
    data_dir: &Path,
    pinned_manifest_path: &Path,
    report_prog: Arc<FProg>,
) -> Result<(), HorizonError>
where
    FProg: Fn(&str, usize, usize, &str) + Send + Sync + 'static,
{
    let manifest_bytes = fs::read(pinned_manifest_path)?;
    let version_pkg: VersionPackage = serde_json::from_slice(&manifest_bytes)?;

    let libraries_dir = data_dir.join("libraries");
    fs::create_dir_all(&libraries_dir)?;

    let tasks = collect_library_tasks(&version_pkg, &libraries_dir);
    if tasks.is_empty() {
        return Ok(());
    }

    let dl_cb = {
        let rp = report_prog.clone();
        Arc::new(move |cur: usize, tot: usize, file_name: &str| {
            let detail = format!("Library {cur}/{tot} ({file_name})");
            let percent = (cur * 100).checked_div(tot).unwrap_or(0);
            rp("Downloading Libraries", percent, 100, &detail);
        })
    };

    download_files_concurrent(client, tasks, 16, Some(dl_cb))
        .await
        .map_err(|e| HorizonError::Preparation(format!("Failed to download libraries: {e}")))?;

    Ok(())
}

pub(super) async fn resolve_java_home(
    client: &reqwest::Client,
    data_dir: &Path,
    major_version: u32,
) -> Result<PathBuf, HorizonError> {
    let java_bin = crate::artifacts::java::ensure_java_runtime_in_dir(
        data_dir.to_path_buf(),
        client,
        major_version,
        None,
        None,
    )
    .await
    .map_err(|e| {
        HorizonError::Preparation(format!("Failed to ensure Java {major_version}: {e}"))
    })?;

    let bin_dir = java_bin.parent().ok_or_else(|| {
        HorizonError::Preparation("Java executable parent directory not found".into())
    })?;

    let java_home = bin_dir
        .parent()
        .ok_or_else(|| HorizonError::Preparation("Java home directory not found".into()))?;

    Ok(java_home.to_path_buf())
}
