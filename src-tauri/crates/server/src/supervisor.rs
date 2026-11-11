mod sleep;

use crate::config::{ServerConfig, ServerStatus, ServerStatusEvent};
use crate::process::{
    claim_server_launch, launch_claimed_server, ClaimedServerLaunch, ProcessError,
    ServerLaunchContext, ServerProcessManager,
};
use crate::proxy::{start_server_proxy, ProxyConfig, ServerProxyHandle};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
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
    publication: crate::process::ServerLaunchGeneration,
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
    launch_generation: crate::process::ServerLaunchGeneration,
    complete: bool,
}
impl StartupProcessGuard {
    fn new(
        manager: ServerProcessManager,
        server_id: String,
        pid: u32,
        launch_generation: crate::process::ServerLaunchGeneration,
    ) -> Self {
        Self {
            manager,
            server_id,
            pid,
            launch_generation,
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
        let launch_generation = self.launch_generation.clone();
        tokio::spawn(async move {
            if let Err(error) = manager
                .kill_if_owned(&server_id, pid, &launch_generation)
                .await
            {
                eprintln!("Failed to terminate abandoned startup {server_id}: {error}");
            }
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

async fn proxy_start_failure(
    port: u16,
    source: std::io::Error,
    cleanup: impl std::future::Future<Output = Result<(), ProcessError>>,
) -> ProcessError {
    let primary = if source.kind() == std::io::ErrorKind::AddrInUse {
        ProcessError::PortInUse { port, source }
    } else {
        ProcessError::ProxyStartup { port, source }
    };
    match cleanup.await {
        Ok(()) => primary,
        Err(cleanup) => ProcessError::ProxyStartupCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        },
    }
}

static GENERATION: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Default)]
pub struct ServerSupervisorManager {
    supervised: Arc<Mutex<HashMap<String, SupervisedServer>>>,
    sleep: sleep::SleepCoordinator,
}

impl ServerSupervisorManager {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    fn with_test_stop(stopper: Arc<dyn sleep::TestSleepStop>) -> Self {
        Self {
            supervised: Arc::new(Mutex::new(HashMap::new())),
            sleep: sleep::SleepCoordinator::with_test_stop(stopper),
        }
    }

    #[cfg(test)]
    async fn supervise_idle_for_test(
        &self,
        pm: ServerProcessManager,
        config: ServerConfig,
        idle_timeout: u64,
        active_conns: Arc<AtomicUsize>,
        proxy_handle: Option<ServerProxyHandle>,
        on_status: Arc<dyn Fn(ServerStatusEvent) + Send + Sync + 'static>,
    ) -> u64 {
        let server_id = config.id.clone();
        self.release(&server_id).await;
        let generation = GENERATION.fetch_add(1, Ordering::SeqCst);
        self.register_supervised(SupervisedRegistration {
            config,
            generation,
            idle_timeout,
            pm,
            active_conns,
            proxy_handle,
            last_active_timestamp: Arc::new(AtomicU64::new(0)),
            wake_task: Arc::new(std::sync::Mutex::new(WakeControl::default())),
            on_status,
        })
        .await;
        generation
    }

    #[cfg(test)]
    async fn generation_of(&self, server_id: &str) -> Option<u64> {
        self.supervised
            .lock()
            .await
            .get(server_id)
            .map(|server| server.generation)
    }

    async fn register_supervised(&self, registration: SupervisedRegistration) {
        let server_id = registration.config.id.clone();
        // No task is spawned until its registry entry can be published without
        // another cancellation point.
        let mut registry = self.supervised.lock().await;
        let idle_task = sleep::spawn_idle_monitor(sleep::IdleMonitorParams {
            server_id: server_id.clone(),
            generation: registration.generation,
            idle_timeout: registration.idle_timeout,
            pm: registration.pm,
            active_conns: registration.active_conns,
            on_status: registration.on_status,
            supervised: self.supervised.clone(),
            sleep: self.sleep.clone(),
        });
        registry.insert(
            server_id,
            SupervisedServer {
                config: registration.config,
                proxy_handle: registration.proxy_handle,
                last_active_timestamp: registration.last_active_timestamp,
                generation: registration.generation,
                idle_task: Some(idle_task),
                wake_task: registration.wake_task,
            },
        );
    }

    /// Starts a claimed process and owns its proxy, idle monitor, and wake tasks.
    pub async fn supervise_and_start(
        &self,
        context: ServerLaunchContext,
        mut claimed: ClaimedServerLaunch,
    ) -> Result<u32, ProcessError> {
        let pm = context.processes.clone();
        let config = claimed.config.clone();
        let server_id = config.id.clone();
        let sleep_enabled = if config.core == super::config::ServerCoreType::Bedrock {
            false
        } else {
            config.sleep_enabled.unwrap_or(true)
        };
        let idle_timeout = config.idle_timeout_seconds.unwrap_or(600); // 10 minutes default

        let last_active = Arc::new(AtomicU64::new(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        ));

        // If sleep/proxy is enabled, launch internal server on internal_port
        if sleep_enabled {
            claimed.config.port = super::config::internal_port(&config);
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

        let mut startup =
            StartupProcessGuard::new(pm.clone(), server_id, pid, claimed.generation());

        let wake_task = Arc::new(std::sync::Mutex::new(WakeControl::default()));
        let proxy_handle =
            start_proxy_for_server(&context, &config, pid, &mut startup, wake_task.clone()).await?;

        let active_conns = proxy_handle.active_connections.clone();
        self.register_supervised(SupervisedRegistration {
            config,
            generation,
            idle_timeout,
            pm,
            active_conns,
            proxy_handle: Some(proxy_handle),
            last_active_timestamp: last_active,
            wake_task,
            on_status: context.on_status,
        })
        .await;

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
        self.sleep
            .put_to_sleep(&self.supervised, pm, server_id, &on_status)
            .await
    }

    /// Shuts down the server's proxy (freeing its public port) and forgets it
    pub async fn release(&self, server_id: &str) {
        let removed = self.supervised.lock().await.remove(server_id);
        self.sleep.invalidate(server_id).await;
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
        self.sleep.invalidate(server_id).await;
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

struct SupervisedRegistration {
    config: ServerConfig,
    generation: u64,
    idle_timeout: u64,
    pm: ServerProcessManager,
    active_conns: Arc<AtomicUsize>,
    proxy_handle: Option<ServerProxyHandle>,
    last_active_timestamp: Arc<AtomicU64>,
    wake_task: Arc<std::sync::Mutex<WakeControl>>,
    on_status: Arc<dyn Fn(ServerStatusEvent) + Send + Sync + 'static>,
}

async fn start_proxy_for_server(
    context: &ServerLaunchContext,
    config: &ServerConfig,
    pid: u32,
    startup: &mut StartupProcessGuard,
    wake_task: Arc<std::sync::Mutex<WakeControl>>,
) -> Result<ServerProxyHandle, ProcessError> {
    let pm = context.processes.clone();
    let server_id = config.id.clone();
    let public_port = config.port;
    let internal_port = super::config::internal_port(config);

    let proxy_cfg = ProxyConfig {
        server_id: server_id.clone(),
        server_name: config.name.clone(),
        public_port,
        internal_port,
        max_players: 20,
        sleep_enabled: true,
    };

    let wake_fn = wake_callback(context.clone(), server_id.clone(), internal_port, wake_task);

    let pm_running = pm.clone();
    let s_id_running = server_id.clone();
    let is_running_fn = move || pm_running.observed_status(&s_id_running) == ServerStatus::Running;
    let pm_sleeping = pm.clone();
    let s_id_sleeping = server_id.clone();
    let is_sleeping_fn =
        move || pm_sleeping.observed_status(&s_id_sleeping) == ServerStatus::Sleeping;

    match start_server_proxy(proxy_cfg, wake_fn, is_running_fn, is_sleeping_fn).await {
        Ok(handle) => Ok(handle),
        Err(source) => Err(proxy_start_failure(public_port, source, async {
            pm.stop_owned_and_wait(
                &server_id,
                pid,
                &startup.launch_generation,
                Duration::from_secs(10),
            )
            .await?;
            startup.complete = true;
            Ok(())
        })
        .await),
    }
}

#[cfg(test)]
#[path = "supervisor_tests.rs"]
mod tests;
