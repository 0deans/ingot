use crate::app::accounts as account;
use crate::minecraft::downloader::{
    download_file_chunked, download_files_concurrent, is_download_task_cached, DownloadTask,
};
use crate::minecraft::instance::{InstanceConfig, ModLoaderType};
use crate::minecraft::java::ensure_java_runtime;
use crate::minecraft::loader::{resolve_loader_profile, LoaderProfile};
use crate::minecraft::version::{
    fetch_asset_index, fetch_version_package, maven_to_path, should_include_library, LibraryEntry,
    VersionPackage,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Runtime};

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

pub async fn prepare_launch_environment<R: Runtime>(
    app: &AppHandle<R>,
    client: &reqwest::Client,
    instance: &InstanceConfig,
    cache_dir: &Path,
    assets_dir: &Path,
    libraries_dir: &Path,
    report_prog: ProgressCallback,
) -> Result<LaunchPreparation, String> {
    fs::create_dir_all(cache_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(assets_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(libraries_dir).map_err(|e| e.to_string())?;

    // Step 1: Version Package
    report_prog("Resolving Version", 1, 10, "Fetching version metadata...");
    let version_pkg = fetch_version_package(client, cache_dir, &instance.game_version).await?;

    // Step 2: Loader Profile (if modded)
    let loader_profile: Option<LoaderProfile> = match instance.loader {
        ModLoaderType::Vanilla => None,
        ref loader => {
            let loader_ver = instance
                .loader_version
                .as_deref()
                .ok_or_else(|| "Loader version not selected".to_string())?;
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
            .await?;
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

    let java_bin = ensure_java_runtime(
        app,
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
                let rel_path = artifact
                    .path
                    .clone()
                    .or_else(|| maven_to_path(&lib.name, None));

                if let Some(path) = rel_path {
                    let dest = libraries_dir.join(&path);
                    classpath_entries.push(dest.clone());
                    library_tasks.push(DownloadTask {
                        url: artifact.url.clone(),
                        dest,
                        sha1: artifact.sha1.clone(),
                        size: artifact.size,
                    });
                }
            }
        } else if let Some(path) = maven_to_path(&lib.name, None) {
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
        download_files_concurrent(client, missing_libs, 16, Some(lib_cb)).await?;
    }

    // Step 6: Assets
    if let Some(asset_ref) = &version_pkg.asset_index {
        let asset_index = fetch_asset_index(client, assets_dir, asset_ref).await?;

        let mut asset_tasks = Vec::new();
        for (_, obj) in asset_index.objects {
            let prefix = &obj.hash[..2];
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
            download_files_concurrent(client, missing_assets, 24, Some(asset_cb)).await?;
        }
    }

    report_prog("Verifying Files", 7, 10, "All game files verified");

    // Step 7: Resolve active account and auth
    let accounts = account::load_accounts_file(app)?;
    let active_account = accounts.into_iter().find(|a| a.is_active);

    let auth = match active_account {
        Some(acc) => {
            let is_ely = acc.account_type == "ely";
            let is_microsoft = acc.account_type == "microsoft";
            let token = if is_microsoft {
                account::get_active_account_token(app.clone())
                    .await
                    .map_err(|e| e.to_string())?
            } else if is_ely {
                account::get_active_account_token(app.clone())
                    .await
                    .map_err(|e| e.to_string())
                    .unwrap_or_else(|_| "0".to_string())
            } else {
                "0".to_string()
            };
            let user_type = if is_microsoft { "msa" } else { "mojang" };
            let xuid = if is_microsoft {
                crate::auth::microsoft::xuid_from_token(&token)
            } else {
                None
            };
            ResolvedAuth {
                player_name: acc.username,
                uuid_str: acc.uuid,
                access_token: token,
                user_type,
                xuid,
                is_ely,
            }
        }
        None => ResolvedAuth {
            player_name: "Player".to_string(),
            uuid_str: uuid::Uuid::new_v4().to_string(),
            access_token: "0".to_string(),
            user_type: "mojang",
            xuid: None,
            is_ely: false,
        },
    };

    Ok(LaunchPreparation {
        version_pkg,
        loader_profile,
        java_bin,
        client_jar_dest,
        classpath_entries,
        auth,
    })
}

pub async fn setup_ely_authlib_agent(
    client: &reqwest::Client,
    cache_dir: &Path,
    report_prog: &ProgressCallback,
) -> PathBuf {
    let authlib_path = cache_dir.join("authlib-injector.jar");
    if !authlib_path.exists() {
        report_prog(
            "Auth Setup",
            7,
            10,
            "Setting up Ely.by skin & authentication agent...",
        );
        let mut dl_url =
            "https://authlib-injector.yushi.moe/artifact/56/authlib-injector-1.2.8.jar".to_string();
        if let Ok(resp) = client
            .get("https://authlib-injector.yushi.moe/artifact/latest.json")
            .send()
            .await
        {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(u) = json.get("download_url").and_then(|v| v.as_str()) {
                    dl_url = u.to_string();
                }
            }
        }

        if let Err(e) = download_file_chunked(client, &dl_url, &authlib_path, None, None).await {
            eprintln!(
                "[Launcher] Primary authlib-injector download failed ({e}), trying GitHub releases mirror..."
            );
            let github_url = "https://github.com/yushijinhun/authlib-injector/releases/download/v1.2.8/authlib-injector-1.2.8.jar";
            let _ = download_file_chunked(client, github_url, &authlib_path, None, None).await;
        }
    }
    authlib_path
}
