use crate::account;
use crate::minecraft::downloader::{
    download_file_chunked, download_files_concurrent, is_download_task_cached, DownloadTask,
};
use crate::minecraft::instance::{get_instance_dir, update_last_played, InstanceConfig, ModLoaderType};
use crate::minecraft::java::ensure_java_runtime;
use crate::minecraft::loader::{resolve_loader_profile, LoaderProfile};
use crate::minecraft::version::{
    fetch_asset_index, fetch_version_package, maven_to_path, should_include_library,
    ArgumentValue, LibraryEntry,
};
use crate::system;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstanceStatusEvent {
    pub instance_id: String,
    pub is_running: bool,
    pub pid: u32,
    pub started_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LaunchProgressEvent {
    pub instance_id: String,
    pub phase: String,
    pub current_step: u64,
    pub total_steps: u64,
    pub percentage: f32,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RunningInstanceSummary {
    pub instance_id: String,
    pub pid: u32,
    pub started_at: u64,
}

struct ActiveChild {
    pid: u32,
    started_at: u64,
}

#[derive(Clone, Default)]
pub struct ProcessManager {
    running: Arc<Mutex<HashMap<String, ActiveChild>>>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            running: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn is_running(&self, instance_id: &str) -> bool {
        let lock = self.running.lock().await;
        lock.contains_key(instance_id)
    }

    pub async fn get_running_instances(&self) -> Vec<RunningInstanceSummary> {
        let lock = self.running.lock().await;
        lock.iter()
            .map(|(id, child)| RunningInstanceSummary {
                instance_id: id.clone(),
                pid: child.pid,
                started_at: child.started_at,
            })
            .collect()
    }

    pub async fn kill_instance(&self, instance_id: &str) -> Result<(), String> {
        let mut lock = self.running.lock().await;
        if let Some(child) = lock.remove(instance_id) {
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("taskkill")
                    .args(["/F", "/T", "/PID", &child.pid.to_string()])
                    .output();
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = std::process::Command::new("kill")
                    .args(["-9", &child.pid.to_string()])
                    .output();
            }
            Ok(())
        } else {
            Err(format!("Instance {} is not running", instance_id))
        }
    }
}

pub async fn launch_minecraft<R: Runtime, FProg, FStatus>(
    app: AppHandle<R>,
    process_manager: ProcessManager,
    client: reqwest::Client,
    instance: InstanceConfig,
    on_progress: FProg,
    on_status: FStatus,
) -> Result<u32, String>
where
    FProg: Fn(LaunchProgressEvent) + Send + Sync + 'static,
    FStatus: Fn(InstanceStatusEvent) + Send + Sync + 'static,
{
    let instance_id = instance.id.clone();

    if process_manager.is_running(&instance_id).await {
        return Err("Instance is already running".to_string());
    }

    let report_prog = {
        let id = instance_id.clone();
        let cb = Arc::new(on_progress);
        let last_time = Arc::new(std::sync::Mutex::new(std::time::Instant::now()));
        let last_phase = Arc::new(std::sync::Mutex::new(String::new()));

        move |phase: &str, current: u64, total: u64, detail: &str| {
            let percentage = if total > 0 {
                (current as f32 / total as f32) * 100.0
            } else {
                0.0
            };

            let should_emit = {
                let mut phase_lock = last_phase.lock().unwrap();
                let mut time_lock = last_time.lock().unwrap();
                let phase_changed = *phase_lock != phase;
                let is_boundary = current == 0 || (total > 0 && current >= total);
                let elapsed_ok = time_lock.elapsed() >= std::time::Duration::from_millis(80);

                if phase_changed || is_boundary || elapsed_ok {
                    *phase_lock = phase.to_string();
                    *time_lock = std::time::Instant::now();
                    true
                } else {
                    false
                }
            };

            if should_emit {
                cb(LaunchProgressEvent {
                    instance_id: id.clone(),
                    phase: phase.to_string(),
                    current_step: current,
                    total_steps: total,
                    percentage,
                    detail: detail.to_string(),
                });
            }
        }
    };

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {e}"))?;

    let cache_dir = data_dir.join("cache");
    let assets_dir = data_dir.join("assets");
    let libraries_dir = data_dir.join("libraries");
    let instance_dir = get_instance_dir(&app, &instance_id)?;

    fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&libraries_dir).map_err(|e| e.to_string())?;

    // Step 1: Version Package
    report_prog("Resolving Version", 1, 10, "Fetching version metadata...");
    let version_pkg = fetch_version_package(&client, &cache_dir, &instance.game_version).await?;

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
            let profile =
                resolve_loader_profile(&client, &cache_dir, loader, &instance.game_version, loader_ver)
                    .await?;
            Some(profile)
        }
    };

    // Step 3: Java Runtime
    report_prog("Checking Java Runtime", 3, 10, "Detecting Java environment...");
    let required_java = version_pkg
        .java_version
        .as_ref()
        .filter(|j| j.major_version > 0)
        .map(|j| j.major_version)
        .unwrap_or_else(|| crate::minecraft::java::get_required_java_version(&instance.game_version, None));

    let java_prog = {
        let rp = report_prog.clone();
        Arc::new(move |cur: u64, tot: u64, det: &str| {
            rp("Java Setup", cur, tot, det);
        })
    };

    let java_bin = ensure_java_runtime(
        &app,
        &client,
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
                report_prog("Downloading Client", 4, 10, "Downloading Minecraft client jar...");
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
                    &client,
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
        .into_iter()
        .filter(should_include_library)
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
            let base_url = lib.url.as_deref().unwrap_or("https://libraries.minecraft.net/");
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
        download_files_concurrent(&client, missing_libs, 16, Some(lib_cb)).await?;
    }

    // Step 6: Assets
    if let Some(asset_ref) = &version_pkg.asset_index {
        let asset_index = fetch_asset_index(&client, &assets_dir, asset_ref).await?;

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
            download_files_concurrent(&client, missing_assets, 24, Some(asset_cb)).await?;
        }
    }

    report_prog("Verifying Files", 7, 10, "All game files verified");

    // Step 7: Resolve active account and auth
    let accounts = account::load_accounts_file(&app).unwrap_or_default();
    let active_account = accounts.into_iter().find(|a| a.is_active);

    let (player_name, uuid_str, access_token, user_type, is_ely) = match active_account {
        Some(acc) => {
            let is_ely = acc.account_type == "ely";
            let token = if is_ely {
                crate::account::get_active_account_token(app.clone())
                    .await
                    .unwrap_or_else(|_| "0".to_string())
            } else {
                "0".to_string()
            };
            (
                acc.username,
                acc.uuid,
                token,
                "mojang",
                is_ely,
            )
        }
        None => (
            "Player".to_string(),
            uuid::Uuid::new_v4().to_string(),
            "0".to_string(),
            "mojang",
            false,
        ),
    };

    // Step 8: Build classpath and arguments
    classpath_entries.push(client_jar_dest);

    #[cfg(target_os = "windows")]
    let cp_separator = ";";
    #[cfg(not(target_os = "windows"))]
    let cp_separator = ":";

    let classpath_str = classpath_entries
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<String>>()
        .join(cp_separator);

    let mem_settings = system::get_memory_settings(&app);
    let min_ram = instance.memory_min_mb.unwrap_or(mem_settings.min_ram_mb);
    let max_ram = instance.memory_max_mb.unwrap_or(mem_settings.max_ram_mb);

    let mut cmd_args: Vec<String> = Vec::new();

    // JVM Memory args
    cmd_args.push(format!("-Xms{}M", min_ram));
    cmd_args.push(format!("-Xmx{}M", max_ram));

    // Common performance JVM flags
    cmd_args.push("-XX:+UnlockExperimentalVMOptions".into());
    cmd_args.push("-XX:+UseG1GC".into());
    cmd_args.push("-XX:G1NewSizePercent=20".into());
    cmd_args.push("-XX:G1ReservePercent=20".into());
    cmd_args.push("-XX:MaxGCPauseMillis=50".into());
    cmd_args.push("-XX:G1HeapRegionSize=32M".into());

    let natives_dir = instance_dir.join("natives");
    let _ = fs::create_dir_all(&natives_dir);

    // Ely authlib injector if Ely account
    if is_ely {
        let authlib_path = cache_dir.join("authlib-injector.jar");
        if !authlib_path.exists() {
            report_prog("Auth Setup", 7, 10, "Setting up Ely.by skin & authentication agent...");
            let mut dl_url = "https://authlib-injector.yushi.moe/artifact/56/authlib-injector-1.2.8.jar".to_string();
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

            if let Err(e) = download_file_chunked(&client, &dl_url, &authlib_path, None, None).await {
                eprintln!("[Launcher] Primary authlib-injector download failed ({e}), trying GitHub releases mirror...");
                let github_url = "https://github.com/yushijinhun/authlib-injector/releases/download/v1.2.8/authlib-injector-1.2.8.jar";
                let _ = download_file_chunked(&client, github_url, &authlib_path, None, None).await;
            }
        }
        if authlib_path.exists() {
            cmd_args.push("-Dauthlibinjector.noLogFile".into());
            cmd_args.push(format!("-javaagent:{}={}", authlib_path.to_string_lossy(), "ely.by"));
        } else {
            eprintln!("[Launcher] Warning: Could not download authlib-injector.jar, Ely.by skin may not display.");
        }
    }

    // Custom JVM args from instance
    if let Some(ref jvm) = instance.jvm_args {
        cmd_args.extend(jvm.clone());
    }

    let replace_jvm_vars = |arg: &str| -> String {
        arg.replace("${natives_directory}", &natives_dir.to_string_lossy())
            .replace("${launcher_name}", "ingot")
            .replace("${launcher_version}", env!("CARGO_PKG_VERSION"))
            .replace("${classpath}", &classpath_str)
            .replace("${library_directory}", &libraries_dir.to_string_lossy())
            .replace("${classpath_separator}", cp_separator)
    };

    let mut has_manifest_jvm = false;
    let mut classpath_handled_by_manifest = false;

    if let Some(ref args_obj) = version_pkg.arguments {
        if let Some(ref jvm_args) = args_obj.jvm {
            has_manifest_jvm = true;
            for arg in jvm_args {
                match arg {
                    ArgumentValue::Simple(s) => {
                        let replaced = replace_jvm_vars(s);
                        if replaced == "-cp" || replaced == "${classpath}" || s == "${classpath}" {
                            classpath_handled_by_manifest = true;
                        }
                        cmd_args.push(replaced);
                    }
                    ArgumentValue::Structured { rules, value } => {
                        let allowed = rules
                            .as_ref()
                            .map(|r| crate::minecraft::version::check_rules(r))
                            .unwrap_or(true);
                        if allowed {
                            if let Some(s) = value.as_str() {
                                let replaced = replace_jvm_vars(s);
                                if replaced == "-cp" || replaced == "${classpath}" || s == "${classpath}" {
                                    classpath_handled_by_manifest = true;
                                }
                                cmd_args.push(replaced);
                            } else if let Some(arr) = value.as_array() {
                                for item in arr {
                                    if let Some(s) = item.as_str() {
                                        let replaced = replace_jvm_vars(s);
                                        if replaced == "-cp" || replaced == "${classpath}" || s == "${classpath}" {
                                            classpath_handled_by_manifest = true;
                                        }
                                        cmd_args.push(replaced);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if !has_manifest_jvm {
        cmd_args.push(format!("-Djava.library.path={}", natives_dir.to_string_lossy()));
        cmd_args.push("-Dminecraft.launcher.brand=ingot".into());
        cmd_args.push(format!("-Dminecraft.launcher.version={}", env!("CARGO_PKG_VERSION")));
        #[cfg(target_os = "windows")]
        cmd_args.push("-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump".into());
        #[cfg(target_os = "macos")]
        cmd_args.push("-XstartOnFirstThread".into());
    }

    if !classpath_handled_by_manifest {
        cmd_args.push("-cp".into());
        cmd_args.push(classpath_str);
    }

    // Main class
    let main_class = if let Some(ref prof) = loader_profile {
        prof.main_class.clone()
    } else {
        version_pkg.main_class
    };
    cmd_args.push(main_class);

    // Game arguments
    let asset_index_id = version_pkg
        .asset_index
        .as_ref()
        .map(|a| a.id.as_str())
        .unwrap_or("legacy");

    let replace_vars = |arg: &str| -> String {
        arg.replace("${auth_player_name}", &player_name)
            .replace("${version_name}", &instance.game_version)
            .replace("${game_directory}", &instance_dir.to_string_lossy())
            .replace("${assets_root}", &assets_dir.to_string_lossy())
            .replace("${assets_index_name}", asset_index_id)
            .replace("${auth_uuid}", &uuid_str)
            .replace("${auth_access_token}", &access_token)
            .replace("${user_type}", user_type)
            .replace("${version_type}", "release")
            .replace("${clientid}", &uuid_str)
            .replace("${auth_xuid}", "0")
    };

    if let Some(args_obj) = version_pkg.arguments {
        if let Some(game_args) = args_obj.game {
            for arg in game_args {
                match arg {
                    ArgumentValue::Simple(s) => {
                        let replaced = replace_vars(&s);
                        // Filter out unreplaced placeholder arguments
                        if !replaced.contains("${") {
                            cmd_args.push(replaced);
                        }
                    }
                    ArgumentValue::Structured { rules, value } => {
                        let allowed = rules.as_ref().map(|r| crate::minecraft::version::check_rules(r)).unwrap_or(true);
                        if allowed {
                            if let Some(s) = value.as_str() {
                                let replaced = replace_vars(s);
                                if !replaced.contains("${") && replaced != "--demo" {
                                    cmd_args.push(replaced);
                                }
                            } else if let Some(arr) = value.as_array() {
                                let mut replaced_group = Vec::new();
                                let mut has_unresolved = false;
                                for item in arr {
                                    if let Some(s) = item.as_str() {
                                        let replaced = replace_vars(s);
                                        if replaced.contains("${") {
                                            has_unresolved = true;
                                            break;
                                        }
                                        replaced_group.push(replaced);
                                    }
                                }
                                if !has_unresolved {
                                    cmd_args.extend(replaced_group);
                                }
                            }
                        }
                    }
                }
            }
        }
    } else if let Some(legacy_args) = version_pkg.minecraft_arguments {
        for part in legacy_args.split_whitespace() {
            let replaced = replace_vars(part);
            if !replaced.contains("${") {
                cmd_args.push(replaced);
            }
        }
    }

    // Apply Window / Display Settings
    let global_settings = crate::system::load_settings(&app);
    let is_fullscreen = instance.fullscreen.unwrap_or(global_settings.window.fullscreen);
    if is_fullscreen {
        if !cmd_args.iter().any(|a| a == "--fullscreen") {
            cmd_args.push("--fullscreen".into());
        }
    } else {
        let width = instance.window_width.unwrap_or(global_settings.window.width);
        let height = instance.window_height.unwrap_or(global_settings.window.height);
        if !cmd_args.iter().any(|a| a == "--width") {
            cmd_args.push("--width".into());
            cmd_args.push(width.to_string());
        }
        if !cmd_args.iter().any(|a| a == "--height") {
            cmd_args.push("--height".into());
            cmd_args.push(height.to_string());
        }
    }

    // Run pre-launch synchronization
    if let Err(e) = crate::minecraft::sync::sync_before_launch(&app, &instance, &instance_dir) {
        eprintln!("[Launcher] Warning: Pre-launch sync failed: {e}");
    }

    report_prog("Starting Game", 10, 10, "Launching Minecraft process...");

    let mut command = tokio::process::Command::new(&java_bin);
    command
        .args(&cmd_args)
        .current_dir(&instance_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let mut child = command
        .spawn()
        .map_err(|e| format!("Failed to spawn Minecraft process: {e}"))?;

    let pid = child.id().unwrap_or(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    {
        let mut lock = process_manager.running.lock().await;
        lock.insert(
            instance_id.clone(),
            ActiveChild {
                pid,
                started_at: now,
            },
        );
    }

    on_status(InstanceStatusEvent {
        instance_id: instance_id.clone(),
        is_running: true,
        pid,
        started_at: now,
    });

    // Handle launcher behavior (keepOpen, hideToTray, close)
    match global_settings.launcher_behavior.as_str() {
        crate::system::BEHAVIOR_HIDE_TO_TRAY => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
        }
        crate::system::BEHAVIOR_CLOSE => {
            let _ = update_last_played(&app, &instance_id, 0);
            let app_exit = app.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                app_exit.exit(0);
            });
        }
        _ => {
            // BEHAVIOR_KEEP_OPEN: keep launcher open as is
        }
    }

    // Background task to monitor child exit
    let pm_clone = process_manager.clone();
    let app_clone = app.clone();
    let inst_id_clone = instance_id.clone();
    let on_status_clone = Arc::new(on_status);
    let inst_clone = instance.clone();
    let inst_dir_clone = instance_dir.clone();

    tokio::spawn(async move {
        match child.wait().await {
            Ok(status) => {
                println!("[Launcher] Minecraft process exited with status: {}", status);
            }
            Err(e) => {
                eprintln!("[Launcher] Error waiting for Minecraft process: {e}");
            }
        }

        // Run post-exit synchronization
        if let Err(e) = crate::minecraft::sync::sync_after_exit(&app_clone, &inst_clone, &inst_dir_clone) {
            eprintln!("[Launcher] Warning: Post-exit sync failed: {e}");
        }

        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs().saturating_sub(now))
            .unwrap_or(0);

        let mut lock = pm_clone.running.lock().await;
        lock.remove(&inst_id_clone);
        let no_more_running = lock.is_empty();
        drop(lock);

        let _ = update_last_played(&app_clone, &inst_id_clone, elapsed);

        on_status_clone(InstanceStatusEvent {
            instance_id: inst_id_clone,
            is_running: false,
            pid: 0,
            started_at: 0,
        });

        // If launcher behavior is hide to tray (or window was hidden), restore window on game exit
        let current_settings = crate::system::load_settings(&app_clone);
        if no_more_running {
            let should_restore = current_settings.launcher_behavior == crate::system::BEHAVIOR_HIDE_TO_TRAY
                || app_clone
                    .get_webview_window("main")
                    .map(|w| w.is_visible().unwrap_or(true) == false)
                    .unwrap_or(false);
            if should_restore {
                crate::tray::restore_main_window(&app_clone);
            }
        }
    });

    Ok(pid)
}
