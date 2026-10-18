use crate::server::process::ServerLifecycleHooks;
use tauri::{AppHandle, Runtime};

/// Adapter forwarding server lifecycle and crash events to Tauri application runtime.
pub struct TauriServerLifecycleHooks<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriServerLifecycleHooks<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> ServerLifecycleHooks for TauriServerLifecycleHooks<R> {
    fn on_process_started(&self, server_id: &str, pid: u32, started_at: u64) {
        crate::running::add(
            &self.app,
            crate::running::Kind::Server,
            server_id,
            pid,
            started_at,
        );
    }

    fn on_process_exited(&self, server_id: &str, pid: u32) {
        crate::running::remove(&self.app, crate::running::Kind::Server, server_id, pid);
    }

    fn on_crash(&self, crash: crate::version_change::CrashReport) {
        crate::app::events::version_change_crash(&self.app, crash);
    }
}

use crate::app::java::ensure_java_runtime_in_dir;
use crate::minecraft::java::get_required_java_version;
use crate::server::config::{ServerConfig, ServerCoreType};
use crate::server::downloader::ensure_server_jar;
use crate::server::process::command::{build_aikar_gc_flags, build_java_version_flags};
use crate::server::process::console::{find_java_console_bin, sandbox_progress};
use crate::server::process::{ProcessError, ServerCommandBuilder};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Default implementation building the native command (Pumpkin, Bedrock, or Java).
pub struct DefaultServerCommandBuilder;

impl ServerCommandBuilder for DefaultServerCommandBuilder {
    fn build_command(
        &self,
        data_dir: &Path,
        client: &reqwest::Client,
        config: &ServerConfig,
        server_dir: &Path,
        step: Arc<dyn Fn(&str) + Send + Sync>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<tokio::process::Command, ProcessError>> + Send>,
    > {
        let data_dir = data_dir.to_path_buf();
        let client = client.clone();
        let config = config.clone();
        let server_dir = server_dir.to_path_buf();
        Box::pin(async move {
            step("Checking server files...");
            ensure_server_jar(
                &client,
                &server_dir,
                &config.core,
                &config.game_version,
                config.build_number.as_deref(),
            )
            .await?;
            build_server_command(&data_dir, &client, &config, &server_dir, move |msg| {
                step(msg)
            })
            .await
        })
    }
}

/// Builds execution command for the server process (Pumpkin, Bedrock, or Java)
async fn build_server_command<FStep>(
    data_dir: &Path,
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
                crate::server::sandbox::ensure_base_rootfs_in_dir(
                    data_dir,
                    client,
                    sandbox_progress(&step),
                )
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
            let (sandbox_dir, rootfs, guest_java) =
                crate::server::sandbox::ensure_sandbox_rootfs_in_dir(
                    data_dir,
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
            let resolved = ensure_java_runtime_in_dir(
                data_dir.to_path_buf(),
                client,
                required_java,
                None,
                None,
            )
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
            if crate::server::installer::launch_args(server_dir, &config.core)?.is_none() {
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
