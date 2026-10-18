use crate::server::config::{ServerConfig, ServerStatus, ServerStatusEvent};
use crate::server::process::{
    claim_server_launch, launch_claimed_server, ClaimedServerLaunch, ProcessError,
    ServerLaunchContext, ServerProcessManager,
};
use crate::server::proxy::{start_server_proxy, ProxyConfig, ServerProxyHandle};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{oneshot, Mutex};

/// A managed supervisor for a single Minecraft server
pub struct SupervisedServer {
    pub config: ServerConfig,
    pub proxy_handle: Option<ServerProxyHandle>,
    pub last_active_timestamp: Arc<AtomicU64>,
    /// Which start this entry belongs to, so a watcher from an earlier run never
    /// touches a newer one
    pub generation: u64,
    pub idle_task: Option<tokio::task::AbortHandle>,
    wake_task: Arc<std::sync::Mutex<WakeControl>>,
}

#[derive(Default)]
struct WakeControl {
    released: bool,
    task: Option<tokio::task::AbortHandle>,
    publication: crate::server::process::ServerLaunchGeneration,
}
impl WakeControl {
    fn release(&mut self) {
        self.released = true;
        self.publication.cancel();
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

impl Drop for SupervisedServer {
    fn drop(&mut self) {
        if let Some(idle) = self.idle_task.take() {
            idle.abort();
        }
        if let Ok(mut wake) = self.wake_task.lock() {
            wake.release();
        }
        if let Some(proxy) = self.proxy_handle.take() {
            let _ = proxy.shutdown_tx.send(());
        }
    }
}
struct StartupProcessGuard {
    manager: ServerProcessManager,
    server_id: String,
    pid: u32,
    complete: bool,
}
impl StartupProcessGuard {
    fn new(manager: ServerProcessManager, server_id: String, pid: u32) -> Self {
        Self {
            manager,
            server_id,
            pid,
            complete: false,
        }
    }
}
impl Drop for StartupProcessGuard {
    fn drop(&mut self) {
        if self.complete {
            return;
        }
        let manager = self.manager.clone();
        let server_id = self.server_id.clone();
        let pid = self.pid;
        tokio::spawn(async move {
            manager.kill_if_owned(&server_id, pid).await;
        });
    }
}
fn wake_callback(
    mut context_wake: ServerLaunchContext,
    s_id_wake: String,
    internal_port: u16,
    wake_task_slot: Arc<std::sync::Mutex<WakeControl>>,
) -> impl Fn() -> oneshot::Receiver<bool> + Send + Sync {
    if let Ok(control) = wake_task_slot.lock() {
        context_wake.publication = control.publication.clone();
    }
    move || {
        let (tx, rx) = oneshot::channel();
        let Ok(mut control) = wake_task_slot.lock() else {
            let _ = tx.send(false);
            return rx;
        };
        if control.released
            || control
                .task
                .as_ref()
                .is_some_and(|task| !task.is_finished())
        {
            let _ = tx.send(false);
            return rx;
        }
        let context = context_wake.clone();
        let server_id = s_id_wake.clone();
        // Registration and release share this gate; tasks await registration.
        let (begin, registered) = oneshot::channel();
        let task = tokio::spawn(async move {
            if registered.await.is_err() {
                return;
            }
            let result = async {
                let mut claimed =
                    claim_server_launch(&context.repository, &context.processes, &server_id)
                        .await?;
                claimed.config.port = internal_port;
                launch_claimed_server(&context, &claimed).await?;
                Ok::<(), ProcessError>(())
            }
            .await;
            if let Err(error) = result {
                eprintln!("[Supervisor] Failed to wake server: {error}");
                let _ = tx.send(false);
                return;
            }
            for _ in 0..50 {
                if TcpStream::connect(("127.0.0.1", internal_port))
                    .await
                    .is_ok()
                {
                    let _ = tx.send(true);
                    return;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            let _ = tx.send(false);
        });
        control.task = Some(task.abort_handle());
        let _ = begin.send(());
        rx
    }
}

static GENERATION: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Default)]
pub struct ServerSupervisorManager {
    supervised: Arc<Mutex<HashMap<String, SupervisedServer>>>,
}

impl ServerSupervisorManager {
    pub fn new() -> Self {
        Self {
            supervised: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Starts a claimed process and owns its proxy, idle monitor, and wake tasks.
    pub(crate) async fn supervise_and_start(
        &self,
        context: ServerLaunchContext,
        mut claimed: ClaimedServerLaunch,
    ) -> Result<u32, ProcessError> {
        let pm = context.processes.clone();
        let on_status = context.on_status.clone();
        let config = claimed.config.clone();
        let server_id = config.id.clone();
        let sleep_enabled = if config.core == super::config::ServerCoreType::Bedrock {
            false
        } else {
            config.sleep_enabled.unwrap_or(true)
        };
        let public_port = config.port;
        let internal_port = super::config::internal_port(&config);
        let idle_timeout = config.idle_timeout_seconds.unwrap_or(600); // 10 minutes default

        let last_active = Arc::new(AtomicU64::new(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        ));

        // If sleep/proxy is enabled, launch internal server on internal_port
        if sleep_enabled {
            claimed.config.port = internal_port;
        }

        // A proxy left from an earlier run (crash, or waking a sleeping server) still
        // holds the public port; free it before starting again
        self.release(&server_id).await;
        let generation = GENERATION.fetch_add(1, Ordering::SeqCst);

        // Launch the initial server process
        let pid = launch_claimed_server(&context, &claimed).await?;

        if !sleep_enabled {
            return Ok(pid);
        }

        let mut startup = StartupProcessGuard::new(pm.clone(), server_id.clone(), pid);

        let wake_task = Arc::new(std::sync::Mutex::new(WakeControl::default()));

        // Start the Smart TCP Proxy on public_port (e.g. 25565)
        let proxy_cfg = ProxyConfig {
            server_id: server_id.clone(),
            server_name: config.name.clone(),
            public_port,
            internal_port,
            max_players: 20,
            sleep_enabled: true,
        };

        let wake_fn = wake_callback(
            context.clone(),
            server_id.clone(),
            internal_port,
            wake_task.clone(),
        );

        let pm_running = pm.clone();
        let s_id_running = server_id.clone();
        let is_running_fn =
            move || pm_running.observed_status(&s_id_running) == ServerStatus::Running;
        let pm_sleeping = pm.clone();
        let s_id_sleeping = server_id.clone();
        let is_sleeping_fn =
            move || pm_sleeping.observed_status(&s_id_sleeping) == ServerStatus::Sleeping;

        let proxy_handle =
            match start_server_proxy(proxy_cfg, wake_fn, is_running_fn, is_sleeping_fn).await {
                Ok(handle) => handle,
                Err(e) => {
                    // Don't leave the game server running without its public port
                    let _ = pm
                        .stop_server_within_typed(&server_id, Duration::from_secs(10))
                        .await;
                    return Err(ProcessError::PortInUse {
                        port: public_port,
                        details: e.to_string(),
                    });
                }
            };

        let active_conns = proxy_handle.active_connections.clone();
        // No task is spawned until its registry entry can be published without
        // another cancellation point.
        let mut registry = self.supervised.lock().await;

        // Spawn background Idle Monitor Loop (checks every 10 seconds)
        let s_id_idle = server_id.clone();
        let pm_idle = pm.clone();
        let on_status_idle = on_status.clone();
        let supervised = self.supervised.clone();
        let idle_task = tokio::spawn(async move {
            eprintln!("[Supervisor] Idle detection loop started for {s_id_idle} (Timeout: {idle_timeout}s)");
            let mut idle_seconds = 0u64;

            loop {
                tokio::time::sleep(Duration::from_secs(10)).await;

                // A newer start (or a manual stop) replaced this run: nothing left to watch
                let current = supervised
                    .lock()
                    .await
                    .get(&s_id_idle)
                    .map(|s| s.generation);
                if current != Some(generation) {
                    break;
                }

                let status = pm_idle.get_server_status(&s_id_idle).await;
                if status == ServerStatus::Stopped {
                    // Going to sleep passes through "stopped" for a moment; still stopped a
                    // bit later means the server exited on its own (crash, failed start)
                    tokio::time::sleep(Duration::from_secs(12)).await;
                    if pm_idle.get_server_status(&s_id_idle).await == ServerStatus::Stopped {
                        let mut guard = supervised.lock().await;
                        if guard.get(&s_id_idle).map(|s| s.generation) == Some(generation) {
                            guard.remove(&s_id_idle);
                            eprintln!("[Supervisor] {s_id_idle} exited; released its port");
                        }
                        break;
                    }
                    continue;
                }

                if status == ServerStatus::Running {
                    let conns = active_conns.load(Ordering::SeqCst);
                    let online_players = pm_idle.get_server_online_players(&s_id_idle).await;

                    if conns == 0 && online_players.is_empty() {
                        idle_seconds += 10;
                        if idle_seconds >= idle_timeout {
                            eprintln!(
                                "[Supervisor] Server {s_id_idle} has been idle for {idle_seconds}s. Entering sleep mode..."
                            );

                            // Send graceful stop to save battery and RAM
                            let _ = pm_idle
                                .stop_server_within_typed(&s_id_idle, Duration::from_secs(10))
                                .await;

                            // Wait for process to terminate
                            for _ in 0..20 {
                                tokio::time::sleep(Duration::from_millis(500)).await;
                                if pm_idle.get_server_status(&s_id_idle).await
                                    == ServerStatus::Stopped
                                {
                                    break;
                                }
                            }

                            if pm_idle.get_server_status(&s_id_idle).await != ServerStatus::Stopped
                            {
                                continue;
                            }
                            // Transition status to Sleeping
                            pm_idle
                                .set_server_status(&s_id_idle, ServerStatus::Sleeping)
                                .await;
                            on_status_idle(ServerStatusEvent {
                                server_id: s_id_idle.clone(),
                                status: ServerStatus::Sleeping,
                                pid: None,
                            });

                            idle_seconds = 0;
                        }
                    } else {
                        // Reset idle counter if players are connected
                        idle_seconds = 0;
                    }
                }
            }
        });

        // Register in supervisor
        {
            registry.insert(
                server_id.clone(),
                SupervisedServer {
                    config: config.clone(),
                    proxy_handle: Some(proxy_handle),
                    last_active_timestamp: last_active,
                    generation,
                    idle_task: Some(idle_task.abort_handle()),
                    wake_task,
                },
            );
        }

        startup.complete = true;
        Ok(pid)
    }

    /// Manually triggers sleep mode for a server
    pub async fn put_to_sleep<FStatus>(
        &self,
        pm: &ServerProcessManager,
        server_id: &str,
        on_status: FStatus,
    ) -> Result<(), ProcessError>
    where
        FStatus: Fn(ServerStatusEvent) + Send + Sync + 'static,
    {
        let status = pm.get_server_status(server_id).await;
        if status != ServerStatus::Running {
            return Err(ProcessError::MustBeRunningToSleep);
        }

        // Gracefully stop internal process
        pm.stop_server_within_typed(server_id, Duration::from_secs(10))
            .await?;

        // Wait for shutdown
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if pm.get_server_status(server_id).await == ServerStatus::Stopped {
                break;
            }
        }

        if pm.get_server_status(server_id).await != ServerStatus::Stopped {
            return Err(ProcessError::Other(
                "Server did not stop before the sleep deadline".into(),
            ));
        }
        pm.set_server_status(server_id, ServerStatus::Sleeping)
            .await;
        on_status(ServerStatusEvent {
            server_id: server_id.to_string(),
            status: ServerStatus::Sleeping,
            pid: None,
        });

        Ok(())
    }

    /// Shuts down the server's proxy (freeing its public port) and forgets it
    pub async fn release(&self, server_id: &str) {
        let removed = self.supervised.lock().await.remove(server_id);
        if let Some(mut server) = removed {
            let proxy = server.proxy_handle.take();
            drop(server);
            if let Some(proxy) = proxy {
                proxy.shutdown().await;
            }
        }
    }

    /// Stops the server and shuts down its proxy
    pub async fn stop_supervised(
        &self,
        pm: &ServerProcessManager,
        server_id: &str,
    ) -> Result<(), ProcessError> {
        pm.cancel_start(server_id);
        self.release(server_id).await;

        match pm
            .stop_server_within_typed(server_id, Duration::from_secs(10))
            .await
        {
            Ok(()) => {}
            Err(ProcessError::NotRunning(_)) => {
                pm.set_server_status(server_id, ServerStatus::Stopped).await;
            }
            Err(error) => return Err(error),
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "supervisor_tests.rs"]
mod tests;
