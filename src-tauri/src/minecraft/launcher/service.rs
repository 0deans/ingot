use super::arguments::{build_game_arguments, build_jvm_arguments, GameArgsConfig, JvmArgsConfig};
use super::models::{LaunchError, QuickPlayOptions};
use super::prepare::{setup_ely_authlib_agent, LaunchPreparation};
use super::progress::ProgressCallback;
use super::quick_play::apply_quick_play_arguments;
use crate::minecraft::instance::InstanceConfig;
use crate::settings::LauncherSettings;
use std::{fs, path::Path, process::Stdio};

pub struct CommandContext<'a> {
    pub instance: &'a InstanceConfig,
    pub settings: &'a LauncherSettings,
    pub instance_dir: &'a Path,
    pub cache_dir: &'a Path,
    pub assets_dir: &'a Path,
    pub libraries_dir: &'a Path,
    pub quick_play: Option<&'a QuickPlayOptions>,
}

pub async fn prepare_game_command(
    client: &reqwest::Client,
    prep: &LaunchPreparation,
    context: CommandContext<'_>,
    report_prog: &ProgressCallback,
) -> Result<tokio::process::Command, LaunchError> {
    let CommandContext {
        instance,
        settings: global_settings,
        instance_dir,
        cache_dir,
        assets_dir,
        libraries_dir,
        quick_play,
    } = context;
    let cp_separator = super::classpath::default_classpath_separator();
    let classpath_str = super::classpath::assemble_classpath(
        &prep.classpath_entries,
        Some(&prep.client_jar_dest),
        cp_separator,
    );

    let mem_settings = &global_settings.memory;
    let min_ram = instance.memory_min_mb.unwrap_or(mem_settings.min_ram_mb);
    let max_ram = instance.memory_max_mb.unwrap_or(mem_settings.max_ram_mb);

    let natives_dir = instance_dir.join("natives");
    fs::create_dir_all(&natives_dir)?;

    let authlib_agent = if prep.auth.is_ely {
        let path = setup_ely_authlib_agent(client, cache_dir, report_prog).await;
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
        libraries_dir,
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
        instance_dir,
        assets_dir,
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
        instance_dir,
        quick_play,
    );

    let mut command = tokio::process::Command::new(&prep.java_bin);
    command
        .args(&cmd_args)
        .current_dir(instance_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    Ok(command)
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
