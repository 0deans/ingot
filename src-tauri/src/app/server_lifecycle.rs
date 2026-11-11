use crate::app::events;
use crate::app::server_config::{delete_server, get_server_dir, load_servers, update_server};
use ingot_artifacts::java::{ensure_java_runtime_in_dir, get_required_java_version};
use ingot_server::config::{ports_of, ConfigError, ServerConfig, ServerCoreType};
use ingot_server::downloader::ensure_server_jar;
use ingot_server::process::command::{build_aikar_gc_flags, build_java_version_flags};
use ingot_server::process::console::{find_java_console_bin, sandbox_progress};
use ingot_server::process::{
    claim_server_launch, ProcessError, ServerCommandBuilder, ServerLaunchContext,
    ServerLifecycleHooks,
};
use ingot_server::{ServerLogEvent, ServerProcessManager, ServerStatus, ServerStatusEvent};
use ingot_version_change::{detect_incomplete_transaction, TransactionPhase, VersionChangeError};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime};
use thiserror::Error;

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

    fn on_crash(&self, crash: ingot_artifacts::crash::CrashReport) {
        events::version_change_crash(&self.app, crash);
    }
}

/// Default implementation building the native command (Pumpkin, Bedrock, or Java).
pub struct DefaultServerCommandBuilder {
    sandbox: ingot_sandbox::SandboxSetup,
}

impl DefaultServerCommandBuilder {
    pub fn new(sandbox: ingot_sandbox::SandboxSetup) -> Self {
        Self { sandbox }
    }
}

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
        let sandbox = self.sandbox.clone();
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
            build_server_command(
                &sandbox,
                &data_dir,
                &client,
                &config,
                &server_dir,
                move |msg| step(msg),
            )
            .await
        })
    }
}

/// Builds execution command for the server process (Pumpkin, Bedrock, or Java)
async fn build_server_command<FStep>(
    sandbox_setup: &ingot_sandbox::SandboxSetup,
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
        let bin = ingot_server::pumpkin::ensure_pumpkin_binary(client, server_dir).await?;
        let sandbox = if ingot_sandbox::is_android_sandbox() {
            step("Preparing Linux sandbox...");
            Some(
                sandbox_setup
                    .ensure_base_rootfs(data_dir, client, sandbox_progress(&step))
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
        ingot_server::pumpkin::build_pumpkin_command(
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
        let bin = ingot_server::bedrock::ensure_bedrock_server_binary(client, server_dir, game_ver)
            .await?;
        ingot_server::bedrock::configure_bedrock_properties(server_dir, config)?;
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

        let (sandbox, java_bin) = if ingot_sandbox::is_android_sandbox() {
            step("Preparing Linux sandbox...");
            let (sandbox_dir, rootfs, guest_java) = sandbox_setup
                .ensure_java(data_dir, client, required_java, sandbox_progress(&step))
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
                ProcessError::MissingJavaConsoleBinary {
                    runtime_dir: resolved.clone(),
                }
            })?;
            (None, bin)
        };

        let java = |java_args: &[String]| {
            ingot_sandbox::build_server_command(
                sandbox
                    .as_ref()
                    .map(|(dir, rootfs)| (dir.as_path(), rootfs.as_path())),
                server_dir,
                &java_bin,
                java_args,
            )
        };

        if ingot_server::installer::uses_installer(&config.core) {
            if ingot_server::installer::launch_args(server_dir, &config.core)?.is_none() {
                step(&format!(
                    "Installing {} (first start only, this can take a few minutes)...",
                    config.core
                ));
            }
            let launch = ingot_server::installer::ensure_installed(
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

/// Errors returned while validating prerequisites, claiming ownership, or launching a server.
#[derive(Debug, Error)]
pub(crate) enum ServerLaunchError {
    #[error("Server not found: {0}")]
    NotFound(String),

    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    Process(#[from] ProcessError),

    #[error("Cannot start server: version change transaction is in incomplete state ({0:?}). Please recover or revert it first.")]
    IncompleteTransaction(TransactionPhase),

    #[error("Cannot start server: {0}. Please recover or inspect the server backup first.")]
    TransactionInspection(#[source] Box<VersionChangeError>),

    #[error("\"{server_name}\" is still running from before Ingot closed. Stop it first, then start it again.")]
    LeftoverRunning { server_name: String },

    #[error("Port {port} is used by \"{server_name}\", which is running. Stop it, or give this server another port in Settings.")]
    PortUsedByServer { port: u16, server_name: String },

    #[error("Port {0} is already in use by another app. Close it, or give this server another port in Settings.")]
    PortInUse(u16),

    #[error("{0}")]
    AppDataDir(String),
}

/// Errors returned while executing a maintenance-guarded server configuration mutation.
#[derive(Debug, Error)]
pub(crate) enum ServerMaintenanceError {
    #[error(transparent)]
    Process(#[from] ProcessError),

    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    TaskJoin(#[from] tokio::task::JoinError),
}

/// Returns whether TPS metrics sampling applies to the configured server core.
pub(crate) fn core_has_tps(config: &ServerConfig) -> bool {
    matches!(
        config.core,
        ServerCoreType::Paper | ServerCoreType::Purpur | ServerCoreType::Folia
    )
}

/// Best-effort installation of the Ingot companion plugin for newly created servers.
pub(crate) fn install_companion_if_supported<R: Runtime>(
    app: &AppHandle<R>,
    server: &ServerConfig,
) {
    if !ingot_server::companion::supported(&server.core, &server.game_version) {
        return;
    }
    match get_server_dir(app, &server.id) {
        Ok(dir) => {
            if let Err(error) =
                ingot_server::companion::install(&dir, &server.core, &server.game_version)
            {
                eprintln!("[Server] Couldn't install the Ingot plugin: {error}");
            }
        }
        Err(error) => {
            eprintln!("[Server] Couldn't resolve server directory for Ingot plugin: {error}");
        }
    }
}

/// Reserves exclusive maintenance ownership before deleting a server off the async runtime.
pub(crate) async fn delete_server_guarded<R: Runtime>(
    app: &AppHandle<R>,
    server_id: String,
    delete_files: bool,
) -> Result<(), ServerMaintenanceError> {
    let app_handle = app.clone();
    let target_id = server_id.clone();
    run_with_maintenance(app, &target_id, move || {
        delete_server(&app_handle, &server_id, delete_files)
    })
    .await
}

/// Reserves exclusive maintenance ownership before updating server configuration off the async runtime.
pub(crate) async fn update_server_guarded<R: Runtime>(
    app: &AppHandle<R>,
    server: ServerConfig,
) -> Result<(), ServerMaintenanceError> {
    let app_handle = app.clone();
    let target_id = server.id.clone();
    run_with_maintenance(app, &target_id, move || update_server(&app_handle, server)).await
}

async fn run_with_maintenance<R, F>(
    app: &AppHandle<R>,
    server_id: &str,
    work: F,
) -> Result<(), ServerMaintenanceError>
where
    R: Runtime,
    F: FnOnce() -> Result<(), ConfigError> + Send + 'static,
{
    let ownership = crate::app::state(app)
        .servers
        .reserve_maintenance(server_id)
        .await?;
    tokio::task::spawn_blocking(move || {
        let _ownership = ownership;
        work()
    })
    .await??;
    Ok(())
}

fn check_version_change_clear(dir: &Path) -> Result<(), ServerLaunchError> {
    match detect_incomplete_transaction(dir) {
        Ok(None) => Ok(()),
        Ok(Some(incomplete)) => Err(ServerLaunchError::IncompleteTransaction(incomplete.phase)),
        Err(error) => Err(ServerLaunchError::TransactionInspection(Box::new(error))),
    }
}

async fn ensure_no_port_conflict(
    pm: &ServerProcessManager,
    servers: &[ServerConfig],
    config: &ServerConfig,
) -> Result<(), ServerLaunchError> {
    let target_ports = ports_of(config);
    for other in servers.iter().filter(|server| server.id != config.id) {
        let other_ports = ports_of(other);
        let Some(port) = target_ports
            .iter()
            .copied()
            .find(|port| other_ports.contains(port))
        else {
            continue;
        };
        if pm.get_server_status(&other.id).await != ServerStatus::Stopped {
            return Err(ServerLaunchError::PortUsedByServer {
                port,
                server_name: other.name.clone(),
            });
        }
    }
    Ok(())
}

async fn wait_for_port_free(port: u16) -> Result<(), ServerLaunchError> {
    let port_free = || std::net::TcpListener::bind(("0.0.0.0", port)).is_ok();
    let mut free = port_free();
    for _ in 0..10 {
        if free {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        free = port_free();
    }
    if !free {
        return Err(ServerLaunchError::PortInUse(port));
    }
    Ok(())
}

async fn validate_pre_launch<R: Runtime>(
    app: &AppHandle<R>,
    servers: &[ServerConfig],
    config: &ServerConfig,
    dir: &Path,
) -> Result<(), ServerLaunchError> {
    check_version_change_clear(dir)?;

    let leftovers = crate::app::lifecycle::leftover_servers(app).await;
    if leftovers
        .iter()
        .any(|leftover| leftover.server_id == config.id)
    {
        events::leftover_servers(app, leftovers);
        return Err(ServerLaunchError::LeftoverRunning {
            server_name: config.name.clone(),
        });
    }

    let app_state = crate::app::state(app);
    ensure_no_port_conflict(&app_state.servers, servers, config).await?;

    if matches!(
        app_state.servers.get_server_status(&config.id).await,
        ServerStatus::Stopped | ServerStatus::Sleeping
    ) {
        app_state.supervisor.release(&config.id).await;
    }

    wait_for_port_free(config.port).await
}

/// Validates pre-launch state, claims ownership, starts supervision, and ensures TPS sampling.
pub(crate) async fn start_server<R: Runtime>(
    app: &AppHandle<R>,
    server_id: &str,
) -> Result<u32, ServerLaunchError> {
    let servers = load_servers(app)?;
    let config = servers
        .iter()
        .find(|server| server.id == server_id)
        .cloned()
        .ok_or_else(|| ServerLaunchError::NotFound(server_id.to_string()))?;

    let server_dir = get_server_dir(app, server_id)?;
    validate_pre_launch(app, &servers, &config, &server_dir).await?;

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| ServerLaunchError::AppDataDir(error.to_string()))?;

    let app_state = crate::app::state(app);
    let pm = app_state.servers.clone();
    let claimed = claim_server_launch(&app_state.server_configs, &pm, server_id).await?;

    let app_log = app.clone();
    let on_log = move |event: ServerLogEvent| {
        events::server_log(&app_log, event);
    };

    let app_status = app.clone();
    let on_status = move |event: ServerStatusEvent| {
        events::server_status(&app_status, event);
    };

    let has_tps = core_has_tps(&config);
    let pid = app_state
        .supervisor
        .supervise_and_start(
            ServerLaunchContext {
                repository: app_state.server_configs.clone(),
                server_dir,
                data_dir,
                processes: pm.clone(),
                client: app_state.http.clone(),
                hooks: Arc::new(TauriServerLifecycleHooks::new(app.clone())),
                command_builder: Arc::new(DefaultServerCommandBuilder::new(
                    app_state.sandbox_setup.clone(),
                )),
                on_log: Arc::new(on_log),
                on_status: Arc::new(on_status),
                publication: Default::default(),
            },
            claimed,
        )
        .await?;

    ingot_server::stats::ensure_sampler(pm, server_id, has_tps);
    Ok(pid)
}

/// Transitions a running supervised server into sleep mode and emits status updates.
pub(crate) async fn put_server_to_sleep<R: Runtime>(
    app: &AppHandle<R>,
    server_id: &str,
) -> Result<(), ProcessError> {
    let app_status = app.clone();
    let on_status = move |event: ServerStatusEvent| {
        events::server_status(&app_status, event);
    };
    let app_state = crate::app::state(app);
    app_state
        .supervisor
        .put_to_sleep(&app_state.servers, server_id, on_status)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ingot_version_change::models::{backup_dir, Journal, JOURNAL};
    use ingot_version_change::write_journal_typed;

    fn sample_server(core: ServerCoreType) -> ServerConfig {
        ServerConfig {
            id: "srv-1".to_string(),
            name: "Test Server".to_string(),
            core,
            game_version: "1.21.4".to_string(),
            build_number: None,
            port: 25565,
            memory_min_mb: 1024,
            memory_max_mb: 2048,
            icon: None,
            java_path: None,
            jvm_args: None,
            auto_start: None,
            sleep_enabled: None,
            idle_timeout_seconds: None,
            internal_port: None,
            playit_enabled: None,
            playit_secret_key: None,
            created_at: 0,
            last_run_at: None,
        }
    }

    fn sample_journal(phase: TransactionPhase) -> Journal {
        Journal {
            created_at: 0,
            from_game_version: "1.21.3".to_string(),
            from_loader: "paper".to_string(),
            from_loader_version: None,
            to_game_version: "1.21.4".to_string(),
            to_loader: "paper".to_string(),
            phase,
            moved: Vec::new(),
            disabled: Vec::new(),
            added: Vec::new(),
            configs: Vec::new(),
            absent_configs: Vec::new(),
            worlds: Vec::new(),
            first_start_checked: false,
        }
    }

    #[test]
    fn core_has_tps_matches_supported_cores() {
        assert!(core_has_tps(&sample_server(ServerCoreType::Paper)));
        assert!(core_has_tps(&sample_server(ServerCoreType::Purpur)));
        assert!(core_has_tps(&sample_server(ServerCoreType::Folia)));
        assert!(!core_has_tps(&sample_server(ServerCoreType::Vanilla)));
        assert!(!core_has_tps(&sample_server(ServerCoreType::Fabric)));
        assert!(!core_has_tps(&sample_server(ServerCoreType::Bedrock)));
    }

    #[test]
    fn check_version_change_clear_blocks_incomplete_and_corrupt_journals() {
        let dir = tempfile::tempdir().unwrap();
        assert!(check_version_change_clear(dir.path()).is_ok());

        std::fs::create_dir_all(backup_dir(dir.path())).unwrap();
        write_journal_typed(dir.path(), &sample_journal(TransactionPhase::Committed)).unwrap();
        assert!(check_version_change_clear(dir.path()).is_ok());

        write_journal_typed(dir.path(), &sample_journal(TransactionPhase::Swapping)).unwrap();
        assert!(matches!(
            check_version_change_clear(dir.path()),
            Err(ServerLaunchError::IncompleteTransaction(
                TransactionPhase::Swapping
            ))
        ));

        std::fs::write(backup_dir(dir.path()).join(JOURNAL), "{invalid json").unwrap();
        assert!(matches!(
            check_version_change_clear(dir.path()),
            Err(ServerLaunchError::TransactionInspection(_))
        ));
    }

    #[test]
    fn launch_error_messages_match_user_facing_text() {
        assert_eq!(
            ServerLaunchError::NotFound("srv-1".into()).to_string(),
            "Server not found: srv-1"
        );
        assert_eq!(
            ServerLaunchError::LeftoverRunning {
                server_name: "Survival".into()
            }
            .to_string(),
            "\"Survival\" is still running from before Ingot closed. Stop it first, then start it again."
        );
        assert_eq!(
            ServerLaunchError::PortUsedByServer {
                port: 25565,
                server_name: "Lobby".into()
            }
            .to_string(),
            "Port 25565 is used by \"Lobby\", which is running. Stop it, or give this server another port in Settings."
        );
        assert_eq!(
            ServerLaunchError::PortInUse(25565).to_string(),
            "Port 25565 is already in use by another app. Close it, or give this server another port in Settings."
        );
    }
}
