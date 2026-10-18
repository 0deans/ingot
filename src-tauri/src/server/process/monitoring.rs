use super::console::{
    clean_console_line, extract_player_keyword, parse_bedrock_connected, parse_bedrock_disconnected,
};
use super::lifecycle::ServerLifecycleHooks;
use super::manager::ServerProcessManager;
use crate::server::config::{ServerCoreType, ServerLogEvent, ServerStatus, ServerStatusEvent};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdout};
use tokio::sync::Mutex;

pub(crate) struct StdoutReaderContext {
    pub(crate) stdout: ChildStdout,
    pub(crate) server_id: String,
    pub(crate) server_dir: PathBuf,
    pub(crate) ready: Arc<std::sync::atomic::AtomicBool>,
    pub(crate) line_tx: tokio::sync::broadcast::Sender<String>,
    pub(crate) pending_queries: Arc<std::sync::atomic::AtomicUsize>,
    pub(crate) log_history: Arc<Mutex<Vec<String>>>,
    pub(crate) online_players: Arc<Mutex<Vec<String>>>,
    pub(crate) player_xuids: Arc<Mutex<HashMap<String, String>>>,
    pub(crate) on_log: Arc<dyn Fn(ServerLogEvent) + Send + Sync + 'static>,
}

pub(crate) fn spawn_stdout_reader(ctx: StdoutReaderContext) -> tokio::task::JoinHandle<()> {
    let StdoutReaderContext {
        stdout,
        server_id,
        server_dir,
        ready,
        line_tx,
        pending_queries,
        log_history,
        online_players,
        player_xuids,
        on_log,
    } = ctx;
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(raw)) = lines.next_line().await {
            let line = clean_console_line(&raw);
            if (line.contains("Done (") && line.contains("For help"))
                || line.contains("Server started")
            {
                ready.store(true, std::sync::atomic::Ordering::SeqCst);
                crate::version_change::crash::first_start_ok(&server_dir);
            }
            let _ = line_tx.send(line.clone());
            if pending_queries.load(std::sync::atomic::Ordering::SeqCst) > 0
                && crate::server::live::is_query_reply(&line)
            {
                continue;
            }

            let level = if line.contains("WARN]") || line.contains("WARN:") {
                "warn".to_string()
            } else if line.contains("ERROR]")
                || line.contains("ERROR:")
                || line.contains("Exception:")
            {
                "error".to_string()
            } else {
                "info".to_string()
            };

            process_player_events(&line, &online_players, &player_xuids).await;

            {
                let mut guard = log_history.lock().await;
                guard.push(line.clone());
                if guard.len() > 1000 {
                    guard.remove(0);
                }
            }

            on_log(ServerLogEvent {
                server_id: server_id.clone(),
                line,
                level,
            });
        }

        let mut guard = online_players.lock().await;
        guard.clear();
        let mut x_guard = player_xuids.lock().await;
        x_guard.clear();
    })
}

async fn process_player_events(
    line: &str,
    online_players: &Mutex<Vec<String>>,
    player_xuids: &Mutex<HashMap<String, String>>,
) {
    if line.contains("joined the game") {
        if let Some(name) = extract_player_keyword(line, " joined the game") {
            let mut guard = online_players.lock().await;
            if !guard.contains(&name) {
                guard.push(name);
            }
        }
    } else if line.contains("left the game") {
        if let Some(name) = extract_player_keyword(line, " left the game") {
            let mut guard = online_players.lock().await;
            guard.retain(|p| p != &name);
        }
    } else if line.contains("Player connected: ") {
        if let Some((name, xuid_opt)) = parse_bedrock_connected(line) {
            let mut guard = online_players.lock().await;
            if !guard.contains(&name) {
                guard.push(name.clone());
            }
            if let Some(xuid) = xuid_opt {
                let mut x_guard = player_xuids.lock().await;
                x_guard.insert(name, xuid);
            }
        }
    } else if line.contains("Player disconnected: ") {
        if let Some(name) = parse_bedrock_disconnected(line) {
            let mut guard = online_players.lock().await;
            guard.retain(|p| p != &name);
            let mut x_guard = player_xuids.lock().await;
            x_guard.remove(&name);
        }
    }
}

pub(crate) fn spawn_stderr_reader(
    stderr: ChildStderr,
    server_id: String,
    log_history: Arc<Mutex<Vec<String>>>,
    on_log: Arc<dyn Fn(ServerLogEvent) + Send + Sync + 'static>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(raw)) = lines.next_line().await {
            let line = clean_console_line(&raw);
            {
                let mut guard = log_history.lock().await;
                guard.push(line.clone());
                if guard.len() > 1000 {
                    guard.remove(0);
                }
            }

            on_log(ServerLogEvent {
                server_id: server_id.clone(),
                line,
                level: "error".to_string(),
            });
        }
    })
}

pub(crate) struct ExitMonitorContext {
    pub(crate) pm: ServerProcessManager,
    pub(crate) child_arc: Arc<Mutex<Child>>,
    pub(crate) readers: Vec<tokio::task::JoinHandle<()>>,
    pub(crate) server_id: String,
    pub(crate) server_dir: PathBuf,
    pub(crate) server_name: String,
    pub(crate) core: ServerCoreType,
    pub(crate) pid: u32,
    pub(crate) ready_flag: Arc<std::sync::atomic::AtomicBool>,
    pub(crate) hooks: Arc<dyn ServerLifecycleHooks>,
    pub(crate) on_status: Arc<dyn Fn(ServerStatusEvent) + Send + Sync + 'static>,
}

pub(crate) fn spawn_exit_monitor(ctx: ExitMonitorContext) -> tokio::task::JoinHandle<()> {
    let ExitMonitorContext {
        pm,
        child_arc,
        readers,
        server_id,
        server_dir,
        server_name,
        core,
        pid,
        ready_flag,
        hooks,
        on_status,
    } = ctx;
    let started_at = SystemTime::now() - Duration::from_secs(2);
    tokio::spawn(async move {
        let mut child = child_arc.lock().await;
        let exited = child.wait().await;
        for mut reader in readers {
            if tokio::time::timeout(Duration::from_secs(1), &mut reader)
                .await
                .is_err()
            {
                reader.abort();
            }
        }
        if !pm.owns(&server_id, pid).await {
            return;
        }
        let stopping = pm.get_server_status(&server_id).await == ServerStatus::Stopping;
        let started = ready_flag.load(std::sync::atomic::Ordering::SeqCst);
        if !started && !stopping {
            let clean = exited.is_ok_and(|s| s.success());
            let folder = crate::server::plugins::platform(&core).map(|p| p.folder);
            if let Some(crash) = crate::version_change::crash::after_exit(
                &server_dir,
                clean,
                crate::version_change::TargetKind::Server,
                &server_id,
                &server_name,
                folder.as_slice(),
                started_at,
            ) {
                hooks.on_crash(crash);
            }
        }

        hooks.on_process_exited(&server_id, pid);
        if !pm.remove_if_pid_matches(&server_id, pid).await {
            return;
        }

        on_status(ServerStatusEvent {
            server_id,
            status: ServerStatus::Stopped,
            pid: None,
        });
    })
}
