use crate::server::config::{
    ServerConfig, ServerLogEvent, ServerStatus, ServerStatusEvent,
};
use crate::server::process::{launch_server, ServerProcessManager};
use crate::server::proxy::{start_server_proxy, ProxyConfig, ServerProxyHandle};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::Runtime;
use tokio::net::TcpStream;
use tokio::sync::{oneshot, Mutex};

/// A managed supervisor for a single Minecraft server
pub struct SupervisedServer {
    pub config: ServerConfig,
    pub proxy_handle: Option<ServerProxyHandle>,
    pub last_active_timestamp: Arc<AtomicU64>,
}

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

    /// Starts or registers a server with the supervisor, starting the smart proxy if sleep is enabled
    pub async fn supervise_and_start<R: Runtime, FLog, FStatus>(
        &self,
        app: tauri::AppHandle<R>,
        pm: ServerProcessManager,
        client: reqwest::Client,
        config: ServerConfig,
        on_log: FLog,
        on_status: FStatus,
    ) -> Result<u32, String>
    where
        FLog: Fn(ServerLogEvent) + Send + Sync + 'static + Clone,
        FStatus: Fn(ServerStatusEvent) + Send + Sync + 'static + Clone,
    {
        let server_id = config.id.clone();
        let sleep_enabled = config.sleep_enabled.unwrap_or(true);
        let public_port = config.port;
        let internal_port = config.internal_port.unwrap_or(public_port.saturating_add(1));
        let idle_timeout = config.idle_timeout_seconds.unwrap_or(600); // 10 minutes default

        let last_active = Arc::new(AtomicU64::new(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        ));

        // If sleep/proxy is enabled, launch internal server on internal_port
        let mut internal_config = config.clone();
        if sleep_enabled {
            internal_config.port = internal_port;
        }

        // Launch the initial server process
        let pid = launch_server(
            app.clone(),
            pm.clone(),
            client.clone(),
            internal_config,
            on_log.clone(),
            on_status.clone(),
        )
        .await?;

        if !sleep_enabled {
            return Ok(pid);
        }

        // Start the Smart TCP Proxy on public_port (e.g. 25565)
        let proxy_cfg = ProxyConfig {
            server_id: server_id.clone(),
            server_name: config.name.clone(),
            public_port,
            internal_port,
            max_players: 20,
            sleep_enabled: true,
        };

        let pm_wake = pm.clone();
        let app_wake = app.clone();
        let client_wake = client.clone();
        let config_wake = config.clone();
        let on_log_wake = on_log.clone();
        let on_status_wake = on_status.clone();
        let s_id_wake = server_id.clone();

        let wake_fn = move || {
            let (tx, rx) = oneshot::channel();
            let pm_w = pm_wake.clone();
            let app_w = app_wake.clone();
            let client_w = client_wake.clone();
            let mut cfg_w = config_wake.clone();
            cfg_w.port = internal_port;
            let on_l = on_log_wake.clone();
            let on_s = on_status_wake.clone();
            let s_id = s_id_wake.clone();

            tokio::spawn(async move {
                eprintln!("[Supervisor] Player join detected, waking server: {s_id}");
                on_s(ServerStatusEvent {
                    server_id: s_id.clone(),
                    status: ServerStatus::Starting,
                    pid: None,
                });

                // Launch internal server
                match launch_server(app_w, pm_w, client_w, cfg_w, on_l, on_s).await {
                    Ok(_) => {
                        // Poll internal port until accepting connections
                        for _ in 0..50 {
                            tokio::time::sleep(Duration::from_millis(500)).await;
                            if TcpStream::connect(format!("127.0.0.1:{internal_port}")).await.is_ok() {
                                let _ = tx.send(true);
                                return;
                            }
                        }
                        let _ = tx.send(false);
                    }
                    Err(e) => {
                        eprintln!("[Supervisor] Failed to wake server: {e}");
                        let _ = tx.send(false);
                    }
                }
            });

            rx
        };

        let pm_running = pm.clone();
        let s_id_running = server_id.clone();
        let is_running_fn = move || {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    pm_running.get_server_status(&s_id_running).await == ServerStatus::Running
                })
            })
        };

        let pm_sleeping = pm.clone();
        let s_id_sleeping = server_id.clone();
        let is_sleeping_fn = move || {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    pm_sleeping.get_server_status(&s_id_sleeping).await == ServerStatus::Sleeping
                })
            })
        };

        let proxy_handle = start_server_proxy(proxy_cfg, wake_fn, is_running_fn, is_sleeping_fn)
            .await
            .map_err(|e| format!("Failed to start TCP proxy on port {public_port}: {e}"))?;

        let active_conns = proxy_handle.active_connections.clone();

        // Register in supervisor
        {
            let mut guard = self.supervised.lock().await;
            guard.insert(
                server_id.clone(),
                SupervisedServer {
                    config: config.clone(),
                    proxy_handle: Some(proxy_handle),
                    last_active_timestamp: last_active.clone(),
                },
            );
        }

        // Spawn background Idle Monitor Loop (checks every 10 seconds)
        let s_id_idle = server_id.clone();
        let pm_idle = pm.clone();
        let on_status_idle = on_status.clone();
        tokio::spawn(async move {
            eprintln!("[Supervisor] Idle detection loop started for {s_id_idle} (Timeout: {idle_timeout}s)");
            let mut idle_seconds = 0u64;

            loop {
                tokio::time::sleep(Duration::from_secs(10)).await;

                let status = pm_idle.get_server_status(&s_id_idle).await;
                if status == ServerStatus::Stopped {
                    // Server was manually stopped, terminate idle loop
                    break;
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
                            let _ = pm_idle.stop_server(&s_id_idle).await;

                            // Wait for process to terminate
                            for _ in 0..20 {
                                tokio::time::sleep(Duration::from_millis(500)).await;
                                if pm_idle.get_server_status(&s_id_idle).await == ServerStatus::Stopped {
                                    break;
                                }
                            }

                            // Transition status to Sleeping
                            pm_idle.set_server_status(&s_id_idle, ServerStatus::Sleeping).await;
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

        Ok(pid)
    }

    /// Manually triggers sleep mode for a server
    pub async fn put_to_sleep<FStatus>(
        &self,
        pm: &ServerProcessManager,
        server_id: &str,
        on_status: FStatus,
    ) -> Result<(), String>
    where
        FStatus: Fn(ServerStatusEvent) + Send + Sync + 'static,
    {
        let status = pm.get_server_status(server_id).await;
        if status != ServerStatus::Running {
            return Err("Server must be running to put it into sleep mode".to_string());
        }

        // Gracefully stop internal process
        pm.stop_server(server_id).await?;

        // Wait for shutdown
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if pm.get_server_status(server_id).await == ServerStatus::Stopped {
                break;
            }
        }

        pm.set_server_status(server_id, ServerStatus::Sleeping).await;
        on_status(ServerStatusEvent {
            server_id: server_id.to_string(),
            status: ServerStatus::Sleeping,
            pid: None,
        });

        Ok(())
    }

    /// Stops the server and shuts down its proxy
    pub async fn stop_supervised(&self, pm: &ServerProcessManager, server_id: &str) -> Result<(), String> {
        let mut guard = self.supervised.lock().await;
        if let Some(supervised) = guard.remove(server_id) {
            if let Some(proxy) = supervised.proxy_handle {
                let _ = proxy.shutdown_tx.send(());
            }
        }

        // Stop the server process if running
        let _ = pm.stop_server(server_id).await;
        Ok(())
    }
}
