use super::manager::ServerProcessManager;
use super::models::{ActiveServer, ProcessError, ServerLaunchGeneration, StartingGuard};
use super::monitoring::{
    spawn_exit_monitor, spawn_stderr_reader, spawn_stdout_reader, ExitMonitorContext,
    StdoutReaderContext,
};
use crate::minecraft::metadata::MetadataRepository;
use crate::server::config::{
    read_server_properties_from_dir, write_server_properties_to_dir, ServerConfig, ServerLogEvent,
    ServerStatus, ServerStatusEvent,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

/// Trait for receiving server lifecycle events and updating external tracking state.
pub trait ServerLifecycleHooks: Send + Sync + 'static {
    fn on_process_started(&self, server_id: &str, pid: u32, started_at: u64);
    fn on_process_exited(&self, server_id: &str, pid: u32);
    fn on_crash(&self, crash: crate::version_change::CrashReport);
}

/// Strategy for constructing the server execution command.
pub trait ServerCommandBuilder: Send + Sync + 'static {
    fn build_command(
        &self,
        data_dir: &Path,
        client: &reqwest::Client,
        config: &ServerConfig,
        server_dir: &Path,
        step: Arc<dyn Fn(&str) + Send + Sync>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<tokio::process::Command, ProcessError>> + Send>,
    >;
}

pub struct ClaimedServerLaunch {
    pub config: ServerConfig,
    ownership: StartingGuard,
}

pub(crate) async fn claim_server_launch(
    repository: &crate::minecraft::metadata::MetadataRepository<ServerConfig>,
    processes: &ServerProcessManager,
    server_id: &str,
) -> Result<ClaimedServerLaunch, ProcessError> {
    let ownership = processes.claim_launch(server_id).await?;
    let config = repository
        .load()?
        .into_iter()
        .find(|config| config.id == server_id)
        .ok_or_else(|| ProcessError::ServerNotFound(format!("Server not found: {server_id}")))?;
    Ok(ClaimedServerLaunch { config, ownership })
}

/// Ensures required configuration files (EULA and server.properties) are written before launch.
pub fn setup_server_environment(
    server_dir: &Path,
    config: &ServerConfig,
) -> Result<(), ProcessError> {
    let eula_path = server_dir.join("eula.txt");
    if !eula_path.exists() {
        std::fs::write(&eula_path, "# Agreed by Ingot\neula=true\n").map_err(ProcessError::Io)?;
    }

    let mut props = read_server_properties_from_dir(server_dir)?;
    if props.server_port != config.port {
        props.server_port = config.port;
        write_server_properties_to_dir(server_dir, &props)?;
    }
    let bind_ip = if config.sleep_enabled.unwrap_or(true) {
        "127.0.0.1"
    } else {
        ""
    };
    crate::server::files::write_properties(
        server_dir,
        &[crate::server::files::PropertyEntry {
            key: "server-ip".to_string(),
            value: bind_ip.to_string(),
        }],
    )?;
    Ok(())
}

/// Dependencies owned by one server launch and retained for supervised wakes.
#[derive(Clone)]
pub(crate) struct ServerLaunchContext {
    pub(crate) repository: Arc<MetadataRepository<ServerConfig>>,
    pub(crate) server_dir: PathBuf,
    pub(crate) data_dir: PathBuf,
    pub(crate) processes: ServerProcessManager,
    pub(crate) client: reqwest::Client,
    pub(crate) hooks: Arc<dyn ServerLifecycleHooks>,
    pub(crate) command_builder: Arc<dyn ServerCommandBuilder>,
    pub(crate) on_log: Arc<dyn Fn(ServerLogEvent) + Send + Sync>,
    pub(crate) on_status: Arc<dyn Fn(ServerStatusEvent) + Send + Sync>,
    pub(crate) publication: ServerLaunchGeneration,
}

pub(crate) async fn launch_claimed_server(
    context: &ServerLaunchContext,
    claimed: &ClaimedServerLaunch,
) -> Result<u32, ProcessError> {
    let config = claimed.config.clone();
    let server_id = config.id.clone();

    let pm = &context.processes;
    let on_log = &context.on_log;
    let on_status = &context.on_status;
    let result = launch_server_inner(context, config, &claimed.ownership.generation).await;

    if let Err(ref e) = result {
        on_log(ServerLogEvent {
            server_id: server_id.clone(),
            line: format!("[Ingot] Failed to start: {e}"),
            level: "error".to_string(),
        });
        if pm.get_server_status(&server_id).await != ServerStatus::Running {
            pm.set_server_status(&server_id, ServerStatus::Stopped)
                .await;
            on_status(ServerStatusEvent {
                server_id: server_id.clone(),
                status: ServerStatus::Stopped,
                pid: None,
            });
        }
    }
    result
}

async fn launch_server_inner(
    context: &ServerLaunchContext,
    config: ServerConfig,
    generation: &ServerLaunchGeneration,
) -> Result<u32, ProcessError> {
    let ServerLaunchContext {
        repository,
        server_dir,
        data_dir,
        processes: pm,
        client,
        hooks,
        command_builder: cmd_builder,
        on_log,
        on_status,
        publication,
    } = context;
    let server_id = config.id.clone();

    pm.set_server_status(&server_id, ServerStatus::Starting)
        .await;
    on_status(ServerStatusEvent {
        server_id: server_id.clone(),
        status: ServerStatus::Starting,
        pid: None,
    });

    let on_log = on_log.clone();
    let setup_log: Arc<std::sync::Mutex<Vec<String>>> = Arc::default();
    let step = {
        let on_log = on_log.clone();
        let setup_log = setup_log.clone();
        let server_id = server_id.clone();
        Arc::new(move |msg: &str| {
            let line = format!("[Ingot] {msg}");
            if let Ok(mut log) = setup_log.lock() {
                log.push(line.clone());
            }
            on_log(ServerLogEvent {
                server_id: server_id.clone(),
                line,
                level: "info".to_string(),
            });
        })
    };

    repository.with_serialized(|| {
        crate::server::config::repair_offline_uuids(server_dir)?;
        setup_server_environment(server_dir, &config)
    })??;

    // 3. Build execution command
    let mut cmd = cmd_builder
        .build_command(data_dir, client, &config, server_dir, step.clone())
        .await?;

    step("Launching server process...");
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW flag so command prompt window does not pop up
        cmd.creation_flags(0x08000000);
    }

    cmd.kill_on_drop(true);
    // Acquire before spawning: cancellation cannot split spawn from registry publication.
    let mut registry = pm.servers.lock().await;
    // Lock order: async registry, launch generation, supervisor generation.
    // Cancellation releases its synchronous gate before acquiring the registry.
    let _launch_publication = generation.enter()?;
    let _supervisor_publication = publication.enter()?;
    let mut child = cmd.spawn().map_err(ProcessError::SpawnFailed)?;

    let pid = child.id().ok_or(ProcessError::MissingPid)?;

    let stdin = child.stdin.take().ok_or(ProcessError::MissingStdin)?;
    let stdout = child.stdout.take().ok_or(ProcessError::MissingStdout)?;
    let stderr = child.stderr.take().ok_or(ProcessError::MissingStderr)?;

    let stdin_arc = Arc::new(Mutex::new(stdin));
    let child_arc = Arc::new(Mutex::new(child));
    let setup_lines = setup_log.lock().map(|l| l.clone()).unwrap_or_default();
    let log_history = Arc::new(Mutex::new(setup_lines));
    let (line_tx, _) = tokio::sync::broadcast::channel::<String>(512);
    let pending_queries = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let ready = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let ready_flag = ready.clone();
    let online_players: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let player_xuids: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let stdout_task = spawn_stdout_reader(StdoutReaderContext {
        stdout,
        server_id: server_id.clone(),
        server_dir: server_dir.to_path_buf(),
        ready: ready.clone(),
        line_tx: line_tx.clone(),
        pending_queries: pending_queries.clone(),
        log_history: log_history.clone(),
        online_players: online_players.clone(),
        player_xuids: player_xuids.clone(),
        on_log: on_log.clone(),
    });

    let stderr_task = spawn_stderr_reader(
        stderr,
        server_id.clone(),
        log_history.clone(),
        on_log.clone(),
    );

    let abort_handles = vec![stdout_task.abort_handle(), stderr_task.abort_handle()];

    registry.insert(
        server_id.clone(),
        ActiveServer {
            server_id: server_id.clone(),
            core: config.core.clone(),
            pid,
            port: config.port,
            started_at: now,
            status: ServerStatus::Running,
            stdin: stdin_arc.clone(),
            log_history: log_history.clone(),
            online_players: online_players.clone(),
            player_xuids: player_xuids.clone(),
            line_tx: line_tx.clone(),
            query_lock: Arc::new(Mutex::new(())),
            pending_queries: pending_queries.clone(),
            ready: ready.clone(),
            abort_handles,
        },
    );

    hooks.on_process_started(&server_id, pid, now);

    on_status(ServerStatusEvent {
        server_id: server_id.clone(),
        status: ServerStatus::Running,
        pid: Some(pid),
    });

    drop(registry);
    // The exit monitor is the reaping owner and must never be cancelled with readers.
    spawn_exit_monitor(ExitMonitorContext {
        pm: pm.clone(),
        child_arc,
        readers: vec![stdout_task, stderr_task],
        server_id,
        server_dir: server_dir.clone(),
        server_name: config.name,
        core: config.core,
        pid,
        ready_flag,
        hooks: hooks.clone(),
        on_status: on_status.clone(),
    });
    Ok(pid)
}
