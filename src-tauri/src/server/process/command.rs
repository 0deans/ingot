use super::console::{find_java_console_bin, sandbox_progress};
use super::models::ProcessError;
use crate::app::java::ensure_java_runtime;
use crate::minecraft::java::get_required_java_version;
use crate::server::config::{ServerConfig, ServerCoreType};
use std::path::{Path, PathBuf};
use tauri::Runtime;

/// Builds standard Aikar's high-performance server G1GC flags
pub fn build_aikar_gc_flags(memory_min_mb: u32, memory_max_mb: u32) -> Vec<String> {
    vec![
        format!("-Xms{memory_min_mb}M"),
        format!("-Xmx{memory_max_mb}M"),
        "-XX:+UseG1GC".into(),
        "-XX:+ParallelRefProcEnabled".into(),
        "-XX:MaxGCPauseMillis=200".into(),
        "-XX:+UnlockExperimentalVMOptions".into(),
        "-XX:+DisableExplicitGC".into(),
        "-XX:+AlwaysPreTouch".into(),
        "-XX:G1NewSizePercent=30".into(),
        "-XX:G1MaxNewSizePercent=40".into(),
        "-XX:G1ReservePercent=20".into(),
        "-XX:G1HeapWastePercent=5".into(),
        "-XX:G1MixedGCCountTarget=4".into(),
        "-XX:InitiatingHeapOccupancyPercent=15".into(),
        "-XX:G1MixedGCLiveThresholdPercent=90".into(),
        "-XX:G1RSetUpdatingPauseTimePercent=5".into(),
        "-XX:SurvivorRatio=32".into(),
        "-XX:+PerfDisableSharedMem".into(),
        "-XX:MaxTenuringThreshold=1".into(),
        "-Dusing.aikars.flags=https://mcflags.emc.gs".into(),
        "-Daikars.new.flags=true".into(),
    ]
}

/// Builds Java flags required for newer Java runtimes (modules / unsafe memory)
pub fn build_java_version_flags(required_java: u32) -> Vec<String> {
    let mut flags = Vec::new();
    if required_java >= 24 {
        flags.push("--sun-misc-unsafe-memory-access=allow".into());
    }
    if required_java >= 17 {
        flags.push("--enable-native-access=ALL-UNNAMED".into());
    }
    flags
}

/// Builds execution command for the server process (Pumpkin, Bedrock, or Java)
pub async fn build_server_command<R: Runtime, FStep>(
    app: &tauri::AppHandle<R>,
    client: &reqwest::Client,
    config: &ServerConfig,
    server_dir: &Path,
    step: FStep,
) -> Result<tokio::process::Command, ProcessError>
where
    FStep: Fn(&str) + Clone + Send + Sync + 'static,
{
    let game_ver = &config.game_version;

    if config.core == ServerCoreType::Pumpkin {
        step("Preparing Pumpkin server...");
        step("Downloading Pumpkin (first start only)...");
        let bin = crate::server::pumpkin::ensure_pumpkin_binary(client, server_dir).await?;
        let sandbox = if crate::server::sandbox::is_android_sandbox() {
            step("Preparing Linux sandbox...");
            Some(
                crate::server::sandbox::ensure_base_rootfs(app, client, sandbox_progress(&step))
                    .await?,
            )
        } else {
            None
        };
        let public_port = if config.sleep_enabled.unwrap_or(true) {
            if config.port >= 11024 {
                config.port - 10000
            } else {
                config.port + 10000
            }
        } else {
            config.port
        };
        crate::server::pumpkin::build_pumpkin_command(
            &bin,
            server_dir,
            config.port,
            public_port,
            sandbox
                .as_ref()
                .map(|(dir, rootfs)| (dir.as_path(), rootfs.as_path())),
        )
        .map_err(ProcessError::Pumpkin)
    } else if config.core == ServerCoreType::Bedrock {
        step("Preparing Bedrock Dedicated Server...");
        step("Downloading Bedrock files (first start only)...");
        let bin =
            crate::server::bedrock::ensure_bedrock_server_binary(client, server_dir, game_ver)
                .await?;
        crate::server::bedrock::configure_bedrock_properties(server_dir, config)?;
        let mut b_cmd = tokio::process::Command::new(bin);
        b_cmd.current_dir(server_dir);
        Ok(b_cmd)
    } else {
        let mut args: Vec<String> =
            build_aikar_gc_flags(config.memory_min_mb, config.memory_max_mb);

        let required_java = get_required_java_version(game_ver, None);
        args.extend(build_java_version_flags(required_java));

        if let Some(ref custom_args) = config.jvm_args {
            args.extend(custom_args.clone());
        }

        // Paper/Spigot: plain console without JLine prompts or ANSI colors
        args.push("-Dterminal.jline=false".into());
        args.push("-Dterminal.ansi=false".into());

        let (sandbox, java_bin) = if crate::server::sandbox::is_android_sandbox() {
            step("Preparing Linux sandbox...");
            let (sandbox_dir, rootfs, guest_java) = crate::server::sandbox::ensure_sandbox_rootfs(
                app,
                client,
                required_java,
                sandbox_progress(&step),
            )
            .await?;
            (Some((sandbox_dir, rootfs)), PathBuf::from(guest_java))
        } else if let Some(ref path) = config.java_path {
            let p = PathBuf::from(path);
            (None, find_java_console_bin(&p).unwrap_or(p))
        } else {
            let resolved = ensure_java_runtime(app, client, required_java, None, None)
                .await
                .map_err(ProcessError::Java)?;
            let bin = find_java_console_bin(&resolved).ok_or_else(|| {
                ProcessError::Other(format!(
                    "Could not find java console binary in {}",
                    resolved.display()
                ))
            })?;
            (None, bin)
        };

        let java = |java_args: &[String]| {
            crate::server::sandbox::build_server_command(
                sandbox
                    .as_ref()
                    .map(|(dir, rootfs)| (dir.as_path(), rootfs.as_path())),
                server_dir,
                &java_bin,
                java_args,
            )
        };

        if crate::server::installer::uses_installer(&config.core) {
            if crate::server::installer::launch_args(server_dir, &config.core).is_none() {
                step(&format!(
                    "Installing {} (first start only, this can take a few minutes)...",
                    config.core
                ));
            }
            let launch = crate::server::installer::ensure_installed(
                client,
                server_dir,
                &config.core,
                game_ver,
                config.build_number.as_deref(),
                &java,
            )
            .await?;
            args.extend(launch);
        } else {
            args.push("-jar".into());
            args.push("server.jar".into());
        }
        args.push("nogui".into());
        java(&args).map_err(ProcessError::from)
    }
}
