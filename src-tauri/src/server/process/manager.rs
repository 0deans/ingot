use super::models::{
    ActiveServer, PendingQueryGuard, ProcessError, ServerLaunchGeneration, StartAdmissionPause,
    StartingGuard,
};
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
    pub(crate) starting_servers: Arc<std::sync::Mutex<HashMap<String, ServerLaunchGeneration>>>,
    paused_starts: Arc<std::sync::atomic::AtomicUsize>,
}

impl ServerProcessManager {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(Mutex::new(HashMap::new())),
            sleeping_servers: Arc::new(Mutex::new(HashSet::new())),
            starting_servers: Arc::new(std::sync::Mutex::new(HashMap::new())),
            paused_starts: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
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

    /// A synchronous proxy callback must never block a Tokio worker.
    pub(crate) fn observed_status(&self, server_id: &str) -> ServerStatus {
        if self.is_starting(server_id) {
            return ServerStatus::Starting;
        }
        let Ok(servers) = self.servers.try_lock() else {
            return ServerStatus::Starting;
        };
        if let Some(server) = servers.get(server_id) {
            return server.status.clone();
        }
        let Ok(sleeping) = self.sleeping_servers.try_lock() else {
            return ServerStatus::Starting;
        };
        if sleeping.contains(server_id) {
            return ServerStatus::Sleeping;
        }
        ServerStatus::Stopped
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
            .map(|s| s.contains_key(server_id))
            .unwrap_or(false)
    }

    pub fn try_claim_starting(&self, server_id: &str) -> Result<StartingGuard, ProcessError> {
        let mut starting =
            self.starting_servers
                .lock()
                .map_err(|_| ProcessError::StatePoisoned {
                    state: "start reservations",
                })?;
        if self
            .paused_starts
            .load(std::sync::atomic::Ordering::Acquire)
            > 0
        {
            return Err(ProcessError::StartAdmissionPaused);
        }
        if starting.contains_key(server_id) {
            return Err(ProcessError::AlreadyStarting);
        }
        let generation = ServerLaunchGeneration::default();
        starting.insert(server_id.to_string(), generation.clone());
        Ok(StartingGuard {
            set: self.starting_servers.clone(),
            server_id: server_id.to_string(),
            generation,
        })
    }

    pub(crate) fn cancel_start(&self, server_id: &str) {
        let starting = match self.starting_servers.lock() {
            Ok(starting) => starting,
            Err(poisoned) => poisoned.into_inner(),
        };
        let generation = starting.get(server_id).cloned();
        drop(starting);
        if let Some(generation) = generation {
            generation.cancel();
        }
    }

    pub(crate) fn cancel_all_starts(&self) {
        let starting = match self.starting_servers.lock() {
            Ok(starting) => starting,
            Err(poisoned) => poisoned.into_inner(),
        };
        let generations: Vec<_> = starting.values().cloned().collect();
        drop(starting);
        for generation in generations {
            generation.cancel();
        }
    }

    pub(crate) fn pause_starts(&self) -> StartAdmissionPause {
        let starting = match self.starting_servers.lock() {
            Ok(starting) => starting,
            Err(poisoned) => poisoned.into_inner(),
        };
        // Admission changes share the reservation lock. Once paused, no new
        // reservation can escape the cancellation snapshot below.
        self.paused_starts
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        let pause = StartAdmissionPause(self.paused_starts.clone());
        drop(starting);
        self.cancel_all_starts();
        pause
    }

    pub(crate) async fn claim_launch(
        &self,
        server_id: &str,
    ) -> Result<StartingGuard, ProcessError> {
        let ownership = self.try_claim_starting(server_id)?;
        if self.servers.lock().await.contains_key(server_id) {
            return Err(ProcessError::AlreadyRunning);
        }
        Ok(ownership)
    }

    pub(crate) async fn reserve_maintenance(
        &self,
        server_id: &str,
    ) -> Result<StartingGuard, ProcessError> {
        let ownership = self.try_claim_starting(server_id)?;
        if self.servers.lock().await.contains_key(server_id)
            || self.sleeping_servers.lock().await.contains(server_id)
        {
            return Err(ProcessError::AlreadyStarting);
        }
        Ok(ownership)
    }

    #[cfg(test)]
    pub(crate) async fn insert_server(&self, server: ActiveServer) {
        let mut guard = self.servers.lock().await;
        guard.insert(server.server_id.clone(), server);
    }

    pub(crate) async fn remove_if_pid_matches(&self, server_id: &str, pid: u32) -> bool {
        let mut guard = self.servers.lock().await;
        if guard.get(server_id).is_some_and(|server| server.pid == pid) {
            if let Some(server) = guard.remove(server_id) {
                server.cancel_tasks();
            }
            return true;
        }
        false
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

    pub async fn send_command(&self, server_id: &str, command: &str) -> Result<(), ProcessError> {
        self.send_command_typed(server_id, command).await
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
            .map_err(ProcessError::StdinWrite)?;
        stdin.flush().await.map_err(ProcessError::StdinFlush)?;
        Ok(())
    }

    /// Sends a console command and returns the first stdout line accepted by `is_reply`.
    /// Used for live data (`data get entity`, `list`) without any server plugin.
    pub async fn query_typed(
        &self,
        server_id: &str,
        command: &str,
        is_reply: impl Fn(&str) -> bool,
        timeout: Duration,
    ) -> Result<String, ProcessError> {
        let (query_lock, pending, line_tx) = {
            let guard = self.servers.lock().await;
            let s = guard
                .get(server_id)
                .filter(|s| s.status == ServerStatus::Running)
                .ok_or_else(|| ProcessError::NotRunning(server_id.to_string()))?;
            if !s.ready.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(ProcessError::StillStarting);
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
            self.send_command_typed(server_id, command).await?;
            tokio::time::timeout(timeout, async {
                loop {
                    match rx.recv().await {
                        Ok(line) if is_reply(&line) => return Ok(line),
                        Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                        Err(_) => return Err(ProcessError::ServerStopped),
                    }
                }
            })
            .await
            .map_err(|_| ProcessError::NoReply(command.to_string()))?
        }
        .await;
        // Keep hiding briefly so a late reply doesn't leak into the user's console
        query_guard.complete_delayed(Duration::from_millis(300));
        result
    }

    pub async fn query(
        &self,
        server_id: &str,
        command: &str,
        is_reply: impl Fn(&str) -> bool,
        timeout: Duration,
    ) -> Result<String, ProcessError> {
        self.query_typed(server_id, command, is_reply, timeout)
            .await
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
        .is_some_and(|server| server.pid == pid)
}
