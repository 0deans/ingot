use super::models::{ActiveServer, PendingQueryGuard, ProcessError, StartingGuard};
use crate::server::config::{RunningServerSummary, ServerCoreType, ServerStatus};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

#[derive(Clone, Default)]
pub struct ServerProcessManager {
    pub(crate) servers: Arc<Mutex<HashMap<String, ActiveServer>>>,
    pub(crate) sleeping_servers: Arc<Mutex<HashSet<String>>>,
    /// Servers between "Start" and a spawned process (jar download, sandbox setup).
    /// A std Mutex so the guard in `launch_server` can release it in `Drop`.
    pub(crate) starting_servers: Arc<std::sync::Mutex<HashSet<String>>>,
}

impl ServerProcessManager {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(Mutex::new(HashMap::new())),
            sleeping_servers: Arc::new(Mutex::new(HashSet::new())),
            starting_servers: Arc::new(std::sync::Mutex::new(HashSet::new())),
        }
    }

    pub async fn get_running_servers(&self) -> Vec<RunningServerSummary> {
        let guard = self.servers.lock().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        guard
            .values()
            .map(|s| RunningServerSummary {
                server_id: s.server_id.clone(),
                pid: s.pid,
                port: s.port,
                uptime_seconds: now.saturating_sub(s.started_at),
                status: s.status.clone(),
            })
            .collect()
    }

    pub async fn get_server_status(&self, server_id: &str) -> ServerStatus {
        if self.is_starting(server_id) {
            return ServerStatus::Starting;
        }
        let guard = self.servers.lock().await;
        if let Some(s) = guard.get(server_id) {
            return s.status.clone();
        }
        let sleeping_guard = self.sleeping_servers.lock().await;
        if sleeping_guard.contains(server_id) {
            return ServerStatus::Sleeping;
        }
        ServerStatus::Stopped
    }

    pub fn is_starting(&self, server_id: &str) -> bool {
        self.starting_servers
            .lock()
            .map(|s| s.contains(server_id))
            .unwrap_or(false)
    }

    pub fn try_claim_starting(&self, server_id: &str) -> Result<StartingGuard, ProcessError> {
        let claimed = self
            .starting_servers
            .lock()
            .map(|mut s| s.insert(server_id.to_string()))
            .unwrap_or(false);
        if !claimed {
            return Err(ProcessError::AlreadyStarting);
        }
        Ok(StartingGuard {
            set: self.starting_servers.clone(),
            server_id: server_id.to_string(),
        })
    }

    pub(crate) async fn insert_server(&self, server: ActiveServer) {
        let mut guard = self.servers.lock().await;
        guard.insert(server.server_id.clone(), server);
    }

    pub(crate) async fn remove_server(&self, server_id: &str) {
        let mut guard = self.servers.lock().await;
        guard.remove(server_id);
    }

    pub async fn set_server_status(&self, server_id: &str, status: ServerStatus) {
        if status == ServerStatus::Sleeping {
            let mut sleeping_guard = self.sleeping_servers.lock().await;
            sleeping_guard.insert(server_id.to_string());
        } else {
            let mut sleeping_guard = self.sleeping_servers.lock().await;
            sleeping_guard.remove(server_id);
        }

        let mut guard = self.servers.lock().await;
        if let Some(s) = guard.get_mut(server_id) {
            s.status = status;
        }
    }

    pub async fn get_server_logs(&self, server_id: &str) -> Vec<String> {
        let guard = self.servers.lock().await;
        if let Some(s) = guard.get(server_id) {
            let logs = s.log_history.lock().await;
            logs.clone()
        } else {
            Vec::new()
        }
    }

    pub async fn get_server_online_players(&self, server_id: &str) -> Vec<String> {
        let guard = self.servers.lock().await;
        if let Some(s) = guard.get(server_id) {
            let players = s.online_players.lock().await;
            players.clone()
        } else {
            Vec::new()
        }
    }

    pub async fn get_server_core(&self, server_id: &str) -> Option<ServerCoreType> {
        let guard = self.servers.lock().await;
        guard.get(server_id).map(|s| s.core.clone())
    }

    pub async fn is_bedrock(&self, server_id: &str) -> bool {
        let guard = self.servers.lock().await;
        guard
            .get(server_id)
            .map(|s| s.core == ServerCoreType::Bedrock)
            .unwrap_or(false)
    }

    pub async fn get_bedrock_online_players(
        &self,
        server_id: &str,
    ) -> Vec<(String, Option<String>)> {
        let guard = self.servers.lock().await;
        if let Some(s) = guard.get(server_id) {
            let names = s.online_players.lock().await.clone();
            let xuids = s.player_xuids.lock().await.clone();
            names
                .into_iter()
                .map(|name| {
                    let xuid = xuids.get(&name).cloned();
                    (name, xuid)
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    pub async fn get_player_xuid(&self, server_id: &str, name: &str) -> Option<String> {
        let guard = self.servers.lock().await;
        if let Some(s) = guard.get(server_id) {
            let xuids = s.player_xuids.lock().await;
            xuids.get(name).cloned()
        } else {
            None
        }
    }

    pub async fn send_command(&self, server_id: &str, command: &str) -> Result<(), String> {
        self.send_command_typed(server_id, command)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn send_command_typed(
        &self,
        server_id: &str,
        command: &str,
    ) -> Result<(), ProcessError> {
        let stdin_opt = {
            let guard = self.servers.lock().await;
            guard.get(server_id).map(|s| s.stdin.clone())
        };

        let stdin_arc = stdin_opt.ok_or_else(|| ProcessError::NotRunning(server_id.to_string()))?;
        let mut stdin = stdin_arc.lock().await;
        let line = format!("{}\n", command.trim());
        stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| ProcessError::StdinWrite(e.to_string()))?;
        stdin
            .flush()
            .await
            .map_err(|e| ProcessError::StdinFlush(e.to_string()))?;
        Ok(())
    }

    /// Sends a console command and returns the first stdout line accepted by `is_reply`.
    /// Used for live data (`data get entity`, `list`) without any server plugin.
    pub async fn query(
        &self,
        server_id: &str,
        command: &str,
        is_reply: impl Fn(&str) -> bool,
        timeout: Duration,
    ) -> Result<String, String> {
        let (query_lock, pending, line_tx) = {
            let guard = self.servers.lock().await;
            let s = guard
                .get(server_id)
                .filter(|s| s.status == ServerStatus::Running)
                .ok_or_else(|| "Server is not running".to_string())?;
            if !s.ready.load(std::sync::atomic::Ordering::SeqCst) {
                return Err("Server is still starting".to_string());
            }
            (
                s.query_lock.clone(),
                s.pending_queries.clone(),
                s.line_tx.clone(),
            )
        };

        let _serial = query_lock.lock().await;
        let mut rx = line_tx.subscribe();
        let query_guard = PendingQueryGuard::new(pending);
        let result = async {
            self.send_command(server_id, command).await?;
            tokio::time::timeout(timeout, async {
                loop {
                    match rx.recv().await {
                        Ok(line) if is_reply(&line) => return Ok(line),
                        Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                        Err(_) => return Err("Server stopped".to_string()),
                    }
                }
            })
            .await
            .map_err(|_| format!("No reply to `{command}`"))?
        }
        .await;
        // Keep hiding briefly so a late reply doesn't leak into the user's console
        query_guard.complete_delayed(Duration::from_millis(300));
        result
    }

    pub async fn stop_server(&self, server_id: &str) -> Result<(), String> {
        self.stop_server_within(server_id, Duration::from_secs(10))
            .await
    }

    /// Asks the server to stop (it saves its worlds first) and ends it if it hasn't
    /// exited after `timeout`
    pub async fn stop_server_within(
        &self,
        server_id: &str,
        timeout: Duration,
    ) -> Result<(), String> {
        let (stdin_arc, pid) = {
            let mut guard = self.servers.lock().await;
            if let Some(s) = guard.get_mut(server_id) {
                s.status = ServerStatus::Stopping;
                (s.stdin.clone(), s.pid)
            } else {
                return Err(format!("Server is not running: {server_id}"));
            }
        };

        // Send "stop" command to server
        {
            let mut stdin = stdin_arc.lock().await;
            let _ = stdin.write_all(b"stop\n").await;
            let _ = stdin.flush().await;
        }

        // The exit task (which holds the child while it waits) removes the server once
        // the process is gone
        let server_id_clone = server_id.to_string();
        let servers_map = self.servers.clone();
        tokio::spawn(async move {
            let deadline = tokio::time::Instant::now() + timeout;
            while tokio::time::Instant::now() < deadline {
                tokio::time::sleep(Duration::from_millis(500)).await;
                if !has_process(&servers_map, &server_id_clone, pid).await {
                    return;
                }
            }
            eprintln!(
                "[ServerManager] Graceful stop timed out for {server_id_clone}, killing process..."
            );
            crate::running::kill_tree(pid);
        });

        Ok(())
    }

    /// Waits until the server's process has exited, up to `timeout`
    pub async fn wait_stopped(&self, server_id: &str, timeout: Duration) -> bool {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if !self.servers.lock().await.contains_key(server_id) {
                return true;
            }
            if tokio::time::Instant::now() >= deadline {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    /// Servers with a running process
    pub async fn process_ids(&self) -> Vec<String> {
        self.servers.lock().await.keys().cloned().collect()
    }

    pub async fn sleeping_ids(&self) -> Vec<String> {
        self.sleeping_servers.lock().await.iter().cloned().collect()
    }

    /// Whether this exact process is one Ingot is running now
    pub async fn owns(&self, server_id: &str, pid: u32) -> bool {
        has_process(&self.servers, server_id, pid).await
    }

    pub async fn kill_server(&self, server_id: &str) -> Result<(), String> {
        let pid = {
            let guard = self.servers.lock().await;
            match guard.get(server_id) {
                Some(s) => s.pid,
                None => return Err(format!("Server is not running: {server_id}")),
            }
        };
        // By pid: the exit task holds the child while it waits, and cleans up after it
        crate::running::kill_tree(pid);
        Ok(())
    }
}

async fn has_process(
    servers: &Mutex<HashMap<String, ActiveServer>>,
    server_id: &str,
    pid: u32,
) -> bool {
    servers
        .lock()
        .await
        .get(server_id)
        .is_some_and(|s| s.pid == pid)
}
