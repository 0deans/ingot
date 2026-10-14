use super::arguments::{build_game_arguments, build_jvm_arguments, GameArgsConfig, JvmArgsConfig};
use super::exit::{finish_game, handle_launcher_behavior_on_launch};
use super::models::{
    InstanceStatusEvent, LaunchError, LaunchProgressEvent, QuickPlayOptions, StatusFn,
};
use super::prepare::{prepare_launch_environment, setup_ely_authlib_agent};
use super::process::ProcessManager;
use super::progress::make_progress_reporter;
use super::quick_play::apply_quick_play_arguments;
use crate::minecraft::instance::{get_instance_dir, InstanceConfig};
use crate::system;
use std::fs;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Runtime};

pub async fn launch_minecraft<R: Runtime, FProg, FStatus>(
    app: AppHandle<R>,
    process_manager: ProcessManager,
    client: reqwest::Client,
    instance: InstanceConfig,
    quick_play: Option<QuickPlayOptions>,
    on_progress: FProg,
    on_status: FStatus,
) -> Result<u32, LaunchError>
where
    FProg: Fn(LaunchProgressEvent) + Send + Sync + 'static,
    FStatus: Fn(InstanceStatusEvent) + Send + Sync + 'static,
{
    let instance_id = instance.id.clone();

    let starting_guard = process_manager.reserve_starting(&instance_id).await?;

    let report_prog = make_progress_reporter(instance_id.clone(), on_progress);

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| LaunchError::AppDataDir(format!("Failed to get app data directory: {e}")))?;

    let cache_dir = data_dir.join("cache");
    let assets_dir = data_dir.join("assets");
    let libraries_dir = data_dir.join("libraries");
    let instance_dir = get_instance_dir(&app, &instance_id).map_err(LaunchError::InstanceDir)?;
    match crate::version_change::detect_incomplete_transaction(&instance_dir) {
        Ok(Some(incomplete)) => {
            return Err(LaunchError::IncompleteTransaction(format!(
                "{:?}",
                incomplete.phase
            )));
        }
        Err(err) => {
            return Err(LaunchError::IncompleteTransaction(format!(
                "{err}. Please recover or inspect the instance backup first."
            )));
        }
        Ok(None) => {}
    }

    let prep = prepare_launch_environment(
        &app,
        &client,
        &instance,
        &cache_dir,
        &assets_dir,
        &libraries_dir,
        report_prog.clone(),
    )
    .await
    .map_err(LaunchError::Other)?;

    let mut classpath_entries = prep.classpath_entries;
    classpath_entries.push(prep.client_jar_dest);

    #[cfg(target_os = "windows")]
    let cp_separator = ";";
    #[cfg(not(target_os = "windows"))]
    let cp_separator = ":";

    let classpath_str = classpath_entries
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<String>>()
        .join(cp_separator);

    let global_settings =
        system::load_settings(&app).map_err(|error| LaunchError::Settings(error.to_string()))?;
    let mem_settings = &global_settings.memory;
    let min_ram = instance.memory_min_mb.unwrap_or(mem_settings.min_ram_mb);
    let max_ram = instance.memory_max_mb.unwrap_or(mem_settings.max_ram_mb);

    let natives_dir = instance_dir.join("natives");
    let _ = fs::create_dir_all(&natives_dir);

    let authlib_agent = if prep.auth.is_ely {
        let path = setup_ely_authlib_agent(&client, &cache_dir, &report_prog).await;
        if path.exists() {
            Some(path)
        } else {
            eprintln!("[Launcher] Warning: Could not download authlib-injector.jar, Ely.by skin may not display.");
            None
        }
    } else {
        None
    };

    let main_class = if let Some(ref prof) = prep.loader_profile {
        &prof.main_class
    } else {
        &prep.version_pkg.main_class
    };

    let jvm_cfg = JvmArgsConfig {
        min_ram_mb: min_ram,
        max_ram_mb: max_ram,
        natives_dir: &natives_dir,
        libraries_dir: &libraries_dir,
        classpath_str: &classpath_str,
        cp_separator,
        custom_jvm_args: instance.jvm_args.as_deref(),
        authlib_agent_path: authlib_agent.as_deref(),
        version_arguments: prep.version_pkg.arguments.as_ref(),
        main_class,
    };
    let mut cmd_args = build_jvm_arguments(&jvm_cfg);

    let asset_index_id = prep
        .version_pkg
        .asset_index
        .as_ref()
        .map(|a| a.id.as_str())
        .unwrap_or("legacy");

    let is_fullscreen = instance
        .fullscreen
        .unwrap_or(global_settings.window.fullscreen);
    let width = instance
        .window_width
        .unwrap_or(global_settings.window.width);
    let height = instance
        .window_height
        .unwrap_or(global_settings.window.height);

    let game_cfg = GameArgsConfig {
        player_name: &prep.auth.player_name,
        game_version: &instance.game_version,
        instance_dir: &instance_dir,
        assets_dir: &assets_dir,
        asset_index_id,
        uuid_str: &prep.auth.uuid_str,
        access_token: &prep.auth.access_token,
        user_type: prep.auth.user_type,
        xuid: prep.auth.xuid.as_deref(),
        version_arguments: prep.version_pkg.arguments.as_ref(),
        legacy_arguments: prep.version_pkg.minecraft_arguments.as_deref(),
        is_fullscreen,
        window_width: width,
        window_height: height,
    };
    let game_args = build_game_arguments(&game_cfg);
    cmd_args.extend(game_args);

    apply_quick_play_arguments(
        &mut cmd_args,
        &instance.game_version,
        &instance_dir,
        quick_play.as_ref(),
    );

    // Run pre-launch synchronization
    if let Err(e) = crate::minecraft::sync::sync_before_launch(&app, &instance, &instance_dir) {
        eprintln!("[Launcher] Warning: Pre-launch sync failed: {e}");
    }

    report_prog("Starting Game", 10, 10, "Launching Minecraft process...");

    let mut command = tokio::process::Command::new(&prep.java_bin);
    command
        .args(&cmd_args)
        .current_dir(&instance_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let (mut child, pid) = spawn_and_commit_process(command, starting_guard, now).await?;
    crate::running::add(&app, crate::running::Kind::Game, &instance_id, pid, now);

    on_status(InstanceStatusEvent {
        instance_id: instance_id.clone(),
        is_running: true,
        pid,
        started_at: now,
    });

    handle_launcher_behavior_on_launch(
        &app,
        global_settings.launcher_behavior.as_str(),
        &instance_id,
    );

    // Background task to monitor child exit
    let pm_clone = process_manager.clone();
    let app_clone = app.clone();
    let on_status_clone: StatusFn = Arc::new(on_status);
    let inst_clone = instance.clone();

    tokio::spawn(async move {
        let clean = match child.wait().await {
            Ok(status) => {
                println!(
                    "[Launcher] Minecraft process exited with status: {}",
                    status
                );
                status.success()
            }
            Err(e) => {
                eprintln!("[Launcher] Error waiting for Minecraft process: {e}");
                false
            }
        };
        let ended = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(now);
        finish_game(
            &app_clone,
            &pm_clone,
            &inst_clone,
            pid,
            now,
            ended,
            clean,
            &on_status_clone,
        )
        .await;
    });

    Ok(pid)
}

pub async fn spawn_and_commit_process(
    command: tokio::process::Command,
    starting_guard: super::process::StartingGuard,
    now: u64,
) -> Result<(tokio::process::Child, u32), LaunchError> {
    spawn_and_commit_process_with_hook(command, starting_guard, now, |_| async {}).await
}

pub async fn spawn_and_commit_process_with_hook<F, Fut>(
    mut command: tokio::process::Command,
    starting_guard: super::process::StartingGuard,
    now: u64,
    after_spawn_before_commit: F,
) -> Result<(tokio::process::Child, u32), LaunchError>
where
    F: FnOnce(u32) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    starting_guard.check_active()?;
    command.kill_on_drop(true);
    let mut child = command.spawn().map_err(LaunchError::ProcessSpawn)?;
    let pid = child.id().unwrap_or(0);

    after_spawn_before_commit(pid).await;

    if let Err(commit_err) = starting_guard.commit(pid, now) {
        let _ = child.start_kill();
        let _ = child.wait().await;
        return Err(commit_err);
    }

    Ok((child, pid))
}
