use super::command::build_server_command;
use super::manager::ServerProcessManager;
use super::models::{ActiveServer, ProcessError, StartingGuard};
use super::monitoring::{
    spawn_exit_monitor, spawn_stderr_reader, spawn_stdout_reader, ExitMonitorContext,
    StdoutReaderContext,
};
use crate::server::config::{
    get_server_dir, read_server_properties_from_dir, write_server_properties_to_dir, ServerConfig,
    ServerCoreType, ServerLogEvent, ServerStatus, ServerStatusEvent,
};
use crate::server::downloader::ensure_server_jar;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Runtime;
use tokio::sync::Mutex;

/// Launches the Minecraft server process
pub async fn launch_server<R: Runtime, FLog, FStatus>(
    app: tauri::AppHandle<R>,
    pm: ServerProcessManager,
    client: reqwest::Client,
    server_id: String,
    on_log: FLog,
    on_status: FStatus,
) -> Result<u32, ProcessError>
where
    FLog: Fn(ServerLogEvent) + Send + Sync + 'static,
    FStatus: Fn(ServerStatusEvent) + Send + Sync + 'static,
{
    let mut claimed = claim_server_launch(&app, &pm, &server_id).await?;
    if claimed.config.core != ServerCoreType::Bedrock
        && claimed.config.sleep_enabled.unwrap_or(true)
    {
        claimed.config.port = crate::server::config::internal_port(&claimed.config);
    }
    launch_claimed_server(app, pm, client, claimed, on_log, on_status).await
}

pub(crate) struct ClaimedServerLaunch {
    pub(crate) config: ServerConfig,
    ownership: StartingGuard,
}

pub(crate) async fn claim_server_launch<R: Runtime>(
    app: &tauri::AppHandle<R>,
    processes: &ServerProcessManager,
    server_id: &str,
) -> Result<ClaimedServerLaunch, ProcessError> {
    claim_server_from_repository(&crate::app::state(app).server_configs, processes, server_id).await
}

pub(super) async fn claim_server_from_repository(
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

pub(crate) async fn launch_claimed_server<R: Runtime, FLog, FStatus>(
    app: tauri::AppHandle<R>,
    pm: ServerProcessManager,
    client: reqwest::Client,
    claimed: ClaimedServerLaunch,
    on_log: FLog,
    on_status: FStatus,
) -> Result<u32, ProcessError>
where
    FLog: Fn(ServerLogEvent) + Send + Sync + 'static,
    FStatus: Fn(ServerStatusEvent) + Send + Sync + 'static,
{
    let ClaimedServerLaunch {
        config,
        ownership: _starting,
    } = claimed;
    let server_id = config.id.clone();

    let on_status = Arc::new(on_status);
    let status_cb = on_status.clone();
    let on_log = Arc::new(on_log);
    let log_cb = on_log.clone();
    let result = launch_server_inner(
        app,
        pm.clone(),
        client,
        config,
        move |e| log_cb(e),
        move |e| status_cb(e),
    )
    .await;

    if let Err(ref e) = result {
        on_log(ServerLogEvent {
            server_id: server_id.clone(),
            line: format!("[Ingot] Failed to start: {e}"),
            level: "error".to_string(),
        });
    }
    if result.is_err() && pm.get_server_status(&server_id).await != ServerStatus::Running {
        on_status(ServerStatusEvent {
            server_id: server_id.clone(),
            status: ServerStatus::Stopped,
            pid: None,
        });
    }
    result
}

async fn launch_server_inner<R: Runtime, FLog, FStatus>(
    app: tauri::AppHandle<R>,
    pm: ServerProcessManager,
    client: reqwest::Client,
    config: ServerConfig,
    on_log: FLog,
    on_status: FStatus,
) -> Result<u32, ProcessError>
where
    FLog: Fn(ServerLogEvent) + Send + Sync + 'static,
    FStatus: Fn(ServerStatusEvent) + Send + Sync + 'static,
{
    let on_status: Arc<dyn Fn(ServerStatusEvent) + Send + Sync + 'static> = Arc::new(on_status);
    let server_id = config.id.clone();
    let server_dir = get_server_dir(&app, &server_id)?;

    pm.set_server_status(&server_id, ServerStatus::Starting)
        .await;
    on_status(ServerStatusEvent {
        server_id: server_id.clone(),
        status: ServerStatus::Starting,
        pid: None,
    });

    let on_log = Arc::new(on_log);
    let setup_log: Arc<std::sync::Mutex<Vec<String>>> = Arc::default();
    let step = {
        let on_log = on_log.clone();
        let setup_log = setup_log.clone();
        let server_id = server_id.clone();
        move |msg: &str| {
            let line = format!("[Ingot] {msg}");
            if let Ok(mut log) = setup_log.lock() {
                log.push(line.clone());
            }
            on_log(ServerLogEvent {
                server_id: server_id.clone(),
                line,
                level: "info".to_string(),
            });
        }
    };

    // 1. Ensure server.jar is downloaded
    step("Checking server files...");
    let core_type = config.core.clone();
    let game_ver = config.game_version.clone();
    let build_num = config.build_number.as_deref();

    if let Err(e) = ensure_server_jar(&client, &server_dir, &core_type, &game_ver, build_num).await
    {
        pm.set_server_status(&server_id, ServerStatus::Stopped)
            .await;
        on_status(ServerStatusEvent {
            server_id: server_id.clone(),
            status: ServerStatus::Stopped,
            pid: None,
        });
        return Err(ProcessError::Core(e));
    }

    crate::app::state(&app)
        .server_configs
        .with_serialized(|| crate::server::config::repair_offline_uuids(&server_dir))??;

    // 2. Ensure eula.txt is accepted
    let eula_path = server_dir.join("eula.txt");
    if !eula_path.exists() {
        let _ = std::fs::write(eula_path, "# Agreed by Ingot\neula=true\n");
    }

    // 3. Ensure server.properties exists and has configured port & loopback bind when behind proxy
    let mut props = read_server_properties_from_dir(&server_dir)?;
    if props.server_port != config.port {
        props.server_port = config.port;
        write_server_properties_to_dir(&server_dir, &props)?;
    }
    let bind_ip = if config.sleep_enabled.unwrap_or(true) {
        "127.0.0.1"
    } else {
        ""
    };
    crate::server::files::write_properties(
        &server_dir,
        &[crate::server::files::PropertyEntry {
            key: "server-ip".to_string(),
            value: bind_ip.to_string(),
        }],
    )?;

    // 4. Build execution command
    let mut cmd = build_server_command(&app, &client, &config, &server_dir, step.clone()).await?;

    step("Launching server process...");
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW flag so command prompt window does not pop up
        cmd.creation_flags(0x08000000);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| ProcessError::SpawnFailed(e.to_string()))?;

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

    pm.insert_server(ActiveServer {
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
    })
    .await;

    crate::running::add(&app, crate::running::Kind::Server, &server_id, pid, now);

    on_status(ServerStatusEvent {
        server_id: server_id.clone(),
        status: ServerStatus::Running,
        pid: Some(pid),
    });

    spawn_stdout_reader(StdoutReaderContext {
        stdout,
        server_id: server_id.clone(),
        server_dir: server_dir.clone(),
        ready,
        line_tx,
        pending_queries,
        log_history: log_history.clone(),
        online_players,
        player_xuids,
        on_log: on_log.clone(),
    });

    spawn_stderr_reader(stderr, server_id.clone(), log_history, on_log);

    spawn_exit_monitor(ExitMonitorContext {
        app,
        pm,
        child_arc,
        server_id,
        server_dir,
        server_name: config.name,
        core: config.core,
        pid,
        ready_flag,
        on_status: on_status.clone(),
    });

    Ok(pid)
}
