use super::arguments::{build_game_arguments, build_jvm_arguments, GameArgsConfig, JvmArgsConfig};
use super::models::{LaunchError, QuickPlayOptions};
use super::prepare::{setup_ely_authlib_agent, LaunchPreparation};
use super::progress::ProgressCallback;
use super::quick_play::apply_quick_play_arguments;
use crate::instance::InstanceConfig;
use crate::version::LibraryEntry;
use ingot_settings::LauncherSettings;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
};

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
    extract_native_jars(&prep.native_jars, &natives_dir)?;

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

    let mut custom_jvm_args: Vec<String> = Vec::new();
    if let Some(custom) = instance.jvm_args.as_deref() {
        custom_jvm_args.extend(custom.iter().cloned());
    }
    if let Some(ref prof) = prep.loader_profile {
        custom_jvm_args.extend(prof.jvm_args.iter().cloned());
        if prof.main_class.contains("forgewrapper") {
            custom_jvm_args.push(format!(
                "-Dforgewrapper.librariesDir={}",
                libraries_dir.to_string_lossy()
            ));
            custom_jvm_args.push(format!(
                "-Dforgewrapper.minecraft={}",
                prep.client_jar_dest.to_string_lossy()
            ));
            if let Some(installer) = find_forge_installer_jar(
                prof.maven_files.iter().chain(prof.libraries.iter()),
                libraries_dir,
            ) {
                custom_jvm_args.push(format!(
                    "-Dforgewrapper.installer={}",
                    installer.to_string_lossy()
                ));
            }
        }
    }

    let jvm_cfg = JvmArgsConfig {
        min_ram_mb: min_ram,
        max_ram_mb: max_ram,
        natives_dir: &natives_dir,
        libraries_dir,
        classpath_str: &classpath_str,
        cp_separator,
        custom_jvm_args: Some(&custom_jvm_args),
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

    let (version_arguments, legacy_arguments) = if let Some(ref prof) = prep.loader_profile {
        if let Some(ref loader_args) = prof.minecraft_arguments {
            (None, Some(loader_args.as_str()))
        } else {
            (
                prep.version_pkg.arguments.as_ref(),
                prep.version_pkg.minecraft_arguments.as_deref(),
            )
        }
    } else {
        (
            prep.version_pkg.arguments.as_ref(),
            prep.version_pkg.minecraft_arguments.as_deref(),
        )
    };

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
        version_arguments,
        legacy_arguments,
        is_fullscreen,
        window_width: width,
        window_height: height,
    };
    let game_args = build_game_arguments(&game_cfg);
    cmd_args.extend(game_args);

    if let Some(ref prof) = prep.loader_profile {
        cmd_args.extend(prof.game_args.clone());
    }

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
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    command.creation_flags(0x08000000);
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

fn find_forge_installer_jar<'a>(
    libraries: impl IntoIterator<Item = &'a LibraryEntry>,
    libraries_dir: &Path,
) -> Option<PathBuf> {
    for lib in libraries {
        let is_installer = lib.name.ends_with(":installer")
            || lib
                .downloads
                .as_ref()
                .and_then(|d| d.artifact.as_ref())
                .and_then(|a| a.path.as_deref())
                .is_some_and(|p| p.ends_with("-installer.jar"));
        if is_installer {
            let rel_path = lib
                .downloads
                .as_ref()
                .and_then(|d| d.artifact.as_ref())
                .and_then(|a| a.path.clone())
                .or_else(|| super::classpath::maven_coordinate_to_path(&lib.name, None));
            if let Some(rel) = rel_path {
                let candidate = libraries_dir.join(rel);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

pub fn extract_native_jars(
    native_jars: &[PathBuf],
    natives_dir: &Path,
) -> Result<(), std::io::Error> {
    for jar_path in native_jars {
        if !jar_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Native jar not found: {}", jar_path.display()),
            ));
        }
        let file = fs::File::open(jar_path)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string()))?;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string())
            })?;
            if entry.is_dir() {
                continue;
            }
            let name = entry.name().replace('\\', "/");
            if name.starts_with("META-INF/") {
                continue;
            }
            let file_name = match Path::new(&name).file_name() {
                Some(n) => n,
                None => continue,
            };
            let dest_path = natives_dir.join(file_name);
            let mut out = match fs::File::create(&dest_path) {
                Ok(f) => f,
                Err(err)
                    if dest_path.exists() && err.kind() == std::io::ErrorKind::PermissionDenied =>
                {
                    continue;
                }
                Err(err) => return Err(err),
            };
            std::io::copy(&mut entry, &mut out)?;
        }
    }
    Ok(())
}
