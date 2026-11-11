use super::sleep::{TestSleepStop, TestStopFuture};
use super::{wake_callback, ServerSupervisorManager, WakeControl};
use crate::config::{ServerConfig, ServerCoreType, ServerStatus, ServerStatusEvent};
use crate::process::lifecycle_tests::{CommandFuture, Fixture};
use crate::process::models::{ActiveServer, ProcessError, ServerLaunchGeneration};
use crate::process::shutdown::ServerTermination;
use crate::process::{claim_server_launch, ServerCommandBuilder, ServerProcessManager};
use crate::proxy::{start_server_proxy, ProxyConfig, ServerProxyHandle};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

struct WaitingPreparation {
    entered: Arc<Notify>,
    seen: Arc<Mutex<Vec<ServerConfig>>>,
}
impl ServerCommandBuilder for WaitingPreparation {
    fn build_command(
        &self,
        _data_dir: &Path,
        _client: &reqwest::Client,
        config: &ServerConfig,
        _server_dir: &Path,
        _step: Arc<dyn Fn(&str) + Send + Sync>,
    ) -> CommandFuture {
        let entered = self.entered.clone();
        self.seen.lock().unwrap().push(config.clone());
        Box::pin(async move {
            entered.notify_one();
            std::future::pending().await
        })
    }
}

#[tokio::test]
async fn released_generation_cannot_start_a_new_wake() {
    let fixture = Fixture::new();
    let control = Arc::new(Mutex::new(WakeControl::default()));
    let wake = wake_callback(
        fixture.context.clone(),
        "server".into(),
        25566,
        control.clone(),
    );
    control.lock().unwrap().release();
    assert!(!wake().await.unwrap());
    assert!(!fixture.context.processes.is_starting("server"));
    assert!(fixture.context.processes.process_ids().await.is_empty());
}

#[tokio::test]
async fn wake_reads_fresh_settings_and_release_cancels_its_registered_task() {
    let mut fixture = Fixture::new();
    let entered = Arc::new(Notify::new());
    let seen = Arc::new(Mutex::new(Vec::new()));
    fixture.context.command_builder = Arc::new(WaitingPreparation {
        entered: entered.clone(),
        seen: seen.clone(),
    });
    let control = Arc::new(Mutex::new(WakeControl::default()));
    let wake = wake_callback(
        fixture.context.clone(),
        "server".into(),
        25566,
        control.clone(),
    );
    fixture
        .context
        .repository
        .update(|records| {
            records[0].memory_max_mb = 8192;
            records[0].jvm_args = Some(vec!["-Dupdated=true".into()]);
            Ok(())
        })
        .unwrap();
    let result = wake();
    entered.notified().await;
    assert!(fixture.context.processes.is_starting("server"));
    assert_eq!(seen.lock().unwrap()[0].memory_max_mb, 8192);
    assert_eq!(
        seen.lock().unwrap()[0].jvm_args,
        Some(vec!["-Dupdated=true".into()])
    );
    assert!(!wake().await.unwrap());
    let task = control.lock().unwrap().task.as_ref().unwrap().clone();
    control.lock().unwrap().release();
    tokio::time::timeout(Duration::from_secs(2), async {
        while !task.is_finished() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(result.await.is_err());
    assert!(!fixture.context.processes.is_starting("server"));
    assert!(fixture.context.processes.process_ids().await.is_empty());
    assert!(!wake().await.unwrap());
}

#[tokio::test]
async fn proxy_address_conflict_preserves_bind_cause() {
    use crate::process::ProcessError;
    use std::error::Error;
    let failure = super::proxy_start_failure(
        25565,
        std::io::Error::new(std::io::ErrorKind::AddrInUse, "injected bind conflict"),
        async { Ok(()) },
    )
    .await;
    assert!(matches!(
        failure,
        ProcessError::PortInUse { port: 25565, .. }
    ));
    let source = failure
        .source()
        .unwrap()
        .downcast_ref::<std::io::Error>()
        .unwrap();
    assert_eq!(source.kind(), std::io::ErrorKind::AddrInUse);
}

#[tokio::test]
async fn proxy_permission_failure_is_not_a_port_conflict() {
    use crate::process::ProcessError;
    use std::error::Error;
    let failure = super::proxy_start_failure(
        25565,
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, "injected permissions"),
        async { Ok(()) },
    )
    .await;
    assert!(matches!(
        failure,
        ProcessError::ProxyStartup { port: 25565, .. }
    ));
    let source = failure
        .source()
        .unwrap()
        .downcast_ref::<std::io::Error>()
        .unwrap();
    assert_eq!(source.kind(), std::io::ErrorKind::PermissionDenied);
}

#[tokio::test]
async fn proxy_cleanup_failure_preserves_primary_and_secondary_causes() {
    use crate::process::ProcessError;
    use std::error::Error;
    let failure = super::proxy_start_failure(
        25565,
        std::io::Error::new(std::io::ErrorKind::AddrInUse, "injected bind conflict"),
        async {
            Err(ProcessError::StdinWrite(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "injected stop failure",
            )))
        },
    )
    .await;
    let ProcessError::ProxyStartupCleanup { primary, cleanup } = failure else {
        panic!("expected incomplete startup cleanup");
    };
    assert!(matches!(
        *primary,
        ProcessError::PortInUse { port: 25565, .. }
    ));
    assert!(matches!(*cleanup, ProcessError::StdinWrite(_)));
    assert_eq!(
        primary
            .source()
            .unwrap()
            .downcast_ref::<std::io::Error>()
            .unwrap()
            .kind(),
        std::io::ErrorKind::AddrInUse
    );
    assert_eq!(
        cleanup
            .source()
            .unwrap()
            .downcast_ref::<std::io::Error>()
            .unwrap()
            .kind(),
        std::io::ErrorKind::BrokenPipe
    );
}

type StopCallback =
    dyn Fn(&ServerProcessManager, &str, Duration) -> TestStopFuture<'static> + Send + Sync;

struct FakeSleepStop(Box<StopCallback>);

impl TestSleepStop for FakeSleepStop {
    fn stop_for_sleep<'a>(
        &'a self,
        pm: &'a ServerProcessManager,
        server_id: &'a str,
        grace: Duration,
    ) -> TestStopFuture<'a> {
        self.0(pm, server_id, grace)
    }
}

fn sleep_stopper(
    callback: impl Fn(&ServerProcessManager, &str, Duration) -> TestStopFuture<'static>
        + Send
        + Sync
        + 'static,
) -> Arc<dyn TestSleepStop> {
    Arc::new(FakeSleepStop(Box::new(callback)))
}

fn server_config(server_id: &str, idle_timeout_seconds: u64) -> ServerConfig {
    serde_json::from_value(serde_json::json!({
        "id": server_id,
        "name": "Fixture",
        "core": "paper",
        "gameVersion": "1.20",
        "port": 25565,
        "createdAt": 0,
        "memoryMinMb": 1024,
        "memoryMaxMb": 2048,
        "sleepEnabled": true,
        "idleTimeoutSeconds": idle_timeout_seconds
    }))
    .unwrap()
}

async fn running_server(server_id: &str, pid: u32) -> ActiveServer {
    let mut fixture = tokio::process::Command::new(std::env::current_exe().unwrap());
    fixture
        .arg("--list")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    let mut child = fixture.spawn().unwrap();
    let stdin = child.stdin.take().unwrap();
    child.wait().await.unwrap();
    let (line_tx, _) = tokio::sync::broadcast::channel(4);
    ActiveServer {
        server_id: server_id.to_owned(),
        core: ServerCoreType::Paper,
        pid,
        generation: ServerLaunchGeneration::default(),
        port: 25565,
        started_at: 0,
        status: ServerStatus::Running,
        stdin: Arc::new(tokio::sync::Mutex::new(stdin)),
        termination: ServerTermination::capture(pid),
        log_history: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        online_players: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        player_xuids: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        line_tx,
        query_lock: Arc::new(tokio::sync::Mutex::new(())),
        pending_queries: Arc::new(AtomicUsize::new(0)),
        ready: Arc::new(AtomicBool::new(true)),
        abort_handles: Vec::new(),
    }
}

async fn proxy_fixture(server_id: &str) -> (ServerProxyHandle, Arc<AtomicBool>) {
    let handle = start_server_proxy(
        ProxyConfig {
            server_id: server_id.to_owned(),
            server_name: "Fixture".to_owned(),
            public_port: 0,
            internal_port: 25566,
            max_players: 20,
            sleep_enabled: true,
        },
        || {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = tx.send(false);
            rx
        },
        || true,
        || false,
    )
    .await
    .unwrap();
    let is_running = handle.is_running.clone();
    (handle, is_running)
}

async fn drive_tasks() {
    for _ in 0..12 {
        tokio::task::yield_now().await;
    }
}

async fn advance_idle_ticks(ticks: usize) {
    for _ in 0..ticks {
        tokio::time::advance(Duration::from_secs(10)).await;
        drive_tasks().await;
    }
}

type StatusSink = Arc<dyn Fn(ServerStatusEvent) + Send + Sync>;

fn recording_status_sink() -> (Arc<Mutex<Vec<ServerStatusEvent>>>, StatusSink) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_sink = events.clone();
    let sink: StatusSink = Arc::new(move |event| {
        events_sink.lock().unwrap().push(event);
    });
    (events, sink)
}

#[tokio::test(start_paused = true)]
async fn put_to_sleep_requires_running_status_and_transitions_cleanly() {
    let pm = ServerProcessManager::new();
    let stops = Arc::new(AtomicUsize::new(0));
    let supervisor = ServerSupervisorManager::with_test_stop(sleep_stopper({
        let stops = stops.clone();
        move |pm, server_id, _grace| {
            let pm = pm.clone();
            let server_id = server_id.to_owned();
            stops.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                pm.set_server_status(&server_id, ServerStatus::Stopping)
                    .await;
                pm.remove_if_pid_matches(&server_id, 10).await;
                Ok(())
            })
        }
    }));

    let (events, sink) = recording_status_sink();

    // Stopped -> rejected
    assert!(matches!(
        supervisor
            .put_to_sleep(&pm, "server", {
                let sink = sink.clone();
                move |event| sink(event)
            })
            .await,
        Err(ProcessError::MustBeRunningToSleep)
    ));

    // Starting -> rejected
    let starting = pm.try_claim_starting("server").unwrap();
    assert!(matches!(
        supervisor
            .put_to_sleep(&pm, "server", {
                let sink = sink.clone();
                move |event| sink(event)
            })
            .await,
        Err(ProcessError::MustBeRunningToSleep)
    ));
    drop(starting);

    // Stopping -> rejected
    let mut stopping_server = running_server("server", 10).await;
    stopping_server.status = ServerStatus::Stopping;
    pm.insert_server(stopping_server).await;
    assert!(matches!(
        supervisor
            .put_to_sleep(&pm, "server", {
                let sink = sink.clone();
                move |event| sink(event)
            })
            .await,
        Err(ProcessError::MustBeRunningToSleep)
    ));

    // Running -> Sleeping
    pm.set_server_status("server", ServerStatus::Running).await;
    assert_eq!(stops.load(Ordering::SeqCst), 0);
    supervisor
        .put_to_sleep(&pm, "server", {
            let sink = sink.clone();
            move |event| sink(event)
        })
        .await
        .unwrap();
    assert_eq!(stops.load(Ordering::SeqCst), 1);
    assert_eq!(pm.get_server_status("server").await, ServerStatus::Sleeping);
    assert_eq!(pm.sleeping_ids().await, vec!["server".to_string()]);

    // Sleeping -> rejected
    assert!(matches!(
        supervisor
            .put_to_sleep(&pm, "server", {
                let sink = sink.clone();
                move |event| sink(event)
            })
            .await,
        Err(ProcessError::MustBeRunningToSleep)
    ));

    let recorded = events.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].server_id, "server");
    assert_eq!(recorded[0].status, ServerStatus::Sleeping);
    assert_eq!(recorded[0].pid, None);
}

#[tokio::test]
async fn put_to_sleep_with_real_process_manager_stops_fixture_and_enters_sleep() {
    let probe = std::net::TcpListener::bind("0.0.0.0:0").unwrap();
    let free_port = probe.local_addr().unwrap().port();
    drop(probe);

    let mut fixture = Fixture::new();
    fixture
        .context
        .repository
        .update(|records| {
            records[0].port = free_port;
            records[0].sleep_enabled = Some(true);
            records[0].idle_timeout_seconds = Some(600);
            Ok(())
        })
        .unwrap();

    let ready = Arc::new(Notify::new());
    let ready_log = ready.clone();
    fixture.context.on_log = Arc::new(move |event| {
        if event.line.contains("Done (1.0s)") {
            ready_log.notify_one();
        }
    });

    let (events, sink) = recording_status_sink();
    let upstream_status = fixture.context.on_status.clone();
    let sink_for_context = sink.clone();
    fixture.context.on_status = Arc::new(move |event| {
        sink_for_context(event.clone());
        upstream_status(event);
    });

    let claimed = claim_server_launch(
        &fixture.context.repository,
        &fixture.context.processes,
        "server",
    )
    .await
    .unwrap();
    let supervisor = ServerSupervisorManager::new();
    supervisor
        .supervise_and_start(fixture.context.clone(), claimed)
        .await
        .unwrap();

    tokio::time::timeout(Duration::from_secs(10), ready.notified())
        .await
        .unwrap();

    supervisor
        .put_to_sleep(&fixture.context.processes, "server", move |event| {
            sink(event)
        })
        .await
        .unwrap();

    assert_eq!(
        fixture.context.processes.get_server_status("server").await,
        ServerStatus::Sleeping
    );
    assert_eq!(
        fixture.context.processes.sleeping_ids().await,
        vec!["server".to_string()]
    );
    assert!(fixture.context.processes.process_ids().await.is_empty());
    assert!(supervisor.generation_of("server").await.is_some());
    assert!(events
        .lock()
        .unwrap()
        .iter()
        .any(|event| event.status == ServerStatus::Sleeping && event.pid.is_none()));

    supervisor.release("server").await;
    assert!(supervisor.generation_of("server").await.is_none());
}

#[tokio::test(start_paused = true)]
async fn idle_monitor_counts_idle_ticks_resets_on_activity_and_sleeps_once() {
    let pm = ServerProcessManager::new();
    let server = running_server("server", 10).await;
    let online_players = server.online_players.clone();
    pm.insert_server(server).await;

    let stops = Arc::new(AtomicUsize::new(0));
    let supervisor = ServerSupervisorManager::with_test_stop(sleep_stopper({
        let stops = stops.clone();
        move |pm, server_id, _grace| {
            let pm = pm.clone();
            let server_id = server_id.to_owned();
            stops.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                pm.set_server_status(&server_id, ServerStatus::Stopping)
                    .await;
                pm.remove_if_pid_matches(&server_id, 10).await;
                Ok(())
            })
        }
    }));

    let (proxy_handle, proxy_running) = proxy_fixture("server").await;
    let active_conns = proxy_handle.active_connections.clone();
    let (events, sink) = recording_status_sink();

    let generation = supervisor
        .supervise_idle_for_test(
            pm.clone(),
            server_config("server", 30),
            30,
            active_conns.clone(),
            Some(proxy_handle),
            sink,
        )
        .await;
    drive_tasks().await;

    // 20s idle (< 30s threshold): not yet sleeping.
    advance_idle_ticks(2).await;
    assert_eq!(stops.load(Ordering::SeqCst), 0);
    assert_eq!(pm.get_server_status("server").await, ServerStatus::Running);

    // Active proxy connection resets idle counter.
    active_conns.store(1, Ordering::SeqCst);
    advance_idle_ticks(1).await;
    assert_eq!(stops.load(Ordering::SeqCst), 0);
    active_conns.store(0, Ordering::SeqCst);

    // Connected in-game player keeps idle counter reset.
    online_players.lock().await.push("Steve".to_owned());
    advance_idle_ticks(2).await;
    assert_eq!(stops.load(Ordering::SeqCst), 0);
    online_players.lock().await.clear();

    // 20s idle after reset: still waiting on the final 10s tick.
    advance_idle_ticks(2).await;
    assert_eq!(stops.load(Ordering::SeqCst), 0);
    assert_eq!(pm.get_server_status("server").await, ServerStatus::Running);

    // 30s total idle: transitions once to Sleeping while preserving proxy & supervisor.
    advance_idle_ticks(1).await;
    assert_eq!(stops.load(Ordering::SeqCst), 1);
    assert_eq!(pm.get_server_status("server").await, ServerStatus::Sleeping);
    assert_eq!(supervisor.generation_of("server").await, Some(generation));
    assert!(proxy_running.load(Ordering::SeqCst));
    assert_eq!(events.lock().unwrap().len(), 1);
    assert_eq!(events.lock().unwrap()[0].status, ServerStatus::Sleeping);

    // Additional ticks while Sleeping do not trigger repeat stops.
    advance_idle_ticks(4).await;
    assert_eq!(stops.load(Ordering::SeqCst), 1);
    assert_eq!(events.lock().unwrap().len(), 1);

    supervisor.release("server").await;
    assert!(!proxy_running.load(Ordering::SeqCst));
}

#[tokio::test(start_paused = true)]
async fn manual_and_auto_sleep_propagate_failures_keep_stopping_and_reset_idle_timer() {
    let pm = ServerProcessManager::new();
    pm.insert_server(running_server("manual-timeout", 10).await)
        .await;
    pm.insert_server(running_server("manual-failed", 20).await)
        .await;
    pm.insert_server(running_server("auto", 30).await).await;

    let auto_attempts = Arc::new(AtomicUsize::new(0));
    let supervisor = ServerSupervisorManager::with_test_stop(sleep_stopper({
        let auto_attempts = auto_attempts.clone();
        move |pm, server_id, grace| {
            let pm = pm.clone();
            let server_id = server_id.to_owned();
            let auto_attempts = auto_attempts.clone();
            Box::pin(async move {
                pm.set_server_status(&server_id, ServerStatus::Stopping)
                    .await;
                match server_id.as_str() {
                    "manual-timeout" => {
                        tokio::time::sleep(grace).await;
                        Err(ProcessError::ShutdownTimeout {
                            server_id,
                            timeout: grace,
                        })
                    }
                    "manual-failed" | "auto" => {
                        if server_id == "auto" {
                            auto_attempts.fetch_add(1, Ordering::SeqCst);
                        }
                        Err(ProcessError::ShutdownFailed {
                            graceful: Box::new(ProcessError::ShutdownTimeout {
                                server_id,
                                timeout: grace,
                            }),
                            termination: Box::new(ProcessError::NoReply("kill failed".into())),
                        })
                    }
                    other => panic!("unexpected server {other}"),
                }
            })
        }
    }));

    let (events, sink) = recording_status_sink();

    // Manual timeout propagates ShutdownTimeout without marking Sleeping.
    let timeout_err = supervisor
        .put_to_sleep(&pm, "manual-timeout", {
            let sink = sink.clone();
            move |event| sink(event)
        })
        .await
        .unwrap_err();
    assert!(matches!(
        timeout_err,
        ProcessError::ShutdownTimeout { timeout, .. } if timeout == Duration::from_secs(10)
    ));
    assert_eq!(
        pm.get_server_status("manual-timeout").await,
        ServerStatus::Stopping
    );

    // Manual failure propagates ShutdownFailed without marking Sleeping.
    let failed_err = supervisor
        .put_to_sleep(&pm, "manual-failed", {
            let sink = sink.clone();
            move |event| sink(event)
        })
        .await
        .unwrap_err();
    assert!(matches!(failed_err, ProcessError::ShutdownFailed { .. }));
    assert_eq!(
        pm.get_server_status("manual-failed").await,
        ServerStatus::Stopping
    );

    // Auto-sleep failure preserves Stopping, never marks Sleeping, and resets idle timer.
    let (proxy_handle, proxy_running) = proxy_fixture("auto").await;
    supervisor
        .supervise_idle_for_test(
            pm.clone(),
            server_config("auto", 20),
            20,
            proxy_handle.active_connections.clone(),
            Some(proxy_handle),
            sink.clone(),
        )
        .await;
    drive_tasks().await;

    advance_idle_ticks(2).await;
    assert_eq!(auto_attempts.load(Ordering::SeqCst), 1);
    assert_eq!(pm.get_server_status("auto").await, ServerStatus::Stopping);
    assert!(pm.sleeping_ids().await.is_empty());
    assert!(events.lock().unwrap().is_empty());

    // Even if status later returns to Running, a full 20s idle window is required again.
    pm.set_server_status("auto", ServerStatus::Running).await;
    advance_idle_ticks(1).await;
    assert_eq!(auto_attempts.load(Ordering::SeqCst), 1);

    advance_idle_ticks(1).await;
    assert_eq!(auto_attempts.load(Ordering::SeqCst), 2);
    assert_eq!(pm.get_server_status("auto").await, ServerStatus::Stopping);

    // Delayed process exit after failed auto-sleep still cleans up supervisor & proxy after 12s grace.
    assert!(pm.remove_if_pid_matches("auto", 30).await);
    tokio::time::advance(Duration::from_secs(10)).await;
    drive_tasks().await;
    tokio::time::advance(Duration::from_secs(12)).await;
    drive_tasks().await;
    assert!(supervisor.generation_of("auto").await.is_none());
    assert!(!proxy_running.load(Ordering::SeqCst));
}

#[tokio::test(start_paused = true)]
async fn unexpected_exit_releases_supervisor_and_shuts_down_proxy_after_grace() {
    let pm = ServerProcessManager::new();
    let supervisor = ServerSupervisorManager::new();
    let (proxy_handle, proxy_running) = proxy_fixture("server").await;
    let active_conns = proxy_handle.active_connections.clone();
    let (_events, sink) = recording_status_sink();

    let generation = supervisor
        .supervise_idle_for_test(
            pm.clone(),
            server_config("server", 30),
            30,
            active_conns,
            Some(proxy_handle),
            sink,
        )
        .await;
    drive_tasks().await;

    // 10s check observes Stopped and enters 12s unexpected-exit grace window.
    tokio::time::advance(Duration::from_secs(10)).await;
    drive_tasks().await;
    assert_eq!(supervisor.generation_of("server").await, Some(generation));
    assert!(proxy_running.load(Ordering::SeqCst));

    // Still inside 12s grace period: proxy remains alive.
    tokio::time::advance(Duration::from_secs(11)).await;
    drive_tasks().await;
    assert_eq!(supervisor.generation_of("server").await, Some(generation));
    assert!(proxy_running.load(Ordering::SeqCst));

    // Grace expires while still Stopped: supervisor releases and shuts down proxy.
    tokio::time::advance(Duration::from_secs(1)).await;
    drive_tasks().await;
    assert!(supervisor.generation_of("server").await.is_none());
    assert!(!proxy_running.load(Ordering::SeqCst));
}

#[tokio::test(start_paused = true)]
async fn concurrent_idle_and_manual_sleep_serialize_without_crash_release() {
    let pm = ServerProcessManager::new();
    pm.insert_server(running_server("server", 10).await).await;
    let second_server = running_server("server", 10).await;

    let entered = Arc::new(Notify::new());
    let finish_stop = Arc::new(Notify::new());
    let stops = Arc::new(AtomicUsize::new(0));

    let supervisor = ServerSupervisorManager::with_test_stop(sleep_stopper({
        let entered = entered.clone();
        let finish_stop = finish_stop.clone();
        let stops = stops.clone();
        move |pm, server_id, _grace| {
            let pm = pm.clone();
            let server_id = server_id.to_owned();
            let entered = entered.clone();
            let finish_stop = finish_stop.clone();
            stops.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                pm.set_server_status(&server_id, ServerStatus::Stopping)
                    .await;
                pm.remove_if_pid_matches(&server_id, 10).await;
                entered.notify_one();
                finish_stop.notified().await;
                Ok(())
            })
        }
    }));

    let (proxy_handle, proxy_running) = proxy_fixture("server").await;
    let active_conns = proxy_handle.active_connections.clone();
    let (events, sink) = recording_status_sink();

    let generation = supervisor
        .supervise_idle_for_test(
            pm.clone(),
            server_config("server", 20),
            20,
            active_conns,
            Some(proxy_handle),
            sink.clone(),
        )
        .await;
    drive_tasks().await;

    // First manual put_to_sleep enters and removes process (transient Stopped state) before sleeping.
    let manual = tokio::spawn({
        let supervisor = supervisor.clone();
        let pm = pm.clone();
        let sink = sink.clone();
        async move {
            supervisor
                .put_to_sleep(&pm, "server", move |event| sink(event))
                .await
        }
    });
    entered.notified().await;
    assert_eq!(pm.get_server_status("server").await, ServerStatus::Stopped);

    // Idle loop ticks across the transient Stopped handoff and 20s mark without releasing proxy
    // or starting a duplicate stop.
    advance_idle_ticks(2).await;
    assert_eq!(stops.load(Ordering::SeqCst), 1);
    assert_eq!(supervisor.generation_of("server").await, Some(generation));
    assert!(proxy_running.load(Ordering::SeqCst));

    // Release manual sleep transition and advance one more idle tick to prove idle doesn't re-stop.
    finish_stop.notify_one();
    manual.await.unwrap().unwrap();
    advance_idle_ticks(1).await;

    assert_eq!(stops.load(Ordering::SeqCst), 1);
    assert_eq!(pm.get_server_status("server").await, ServerStatus::Sleeping);
    assert_eq!(events.lock().unwrap().len(), 1);
    assert!(proxy_running.load(Ordering::SeqCst));

    // Reset back to Running to verify idle_task -> concurrent put_to_sleep serialization too.
    pm.set_server_status("server", ServerStatus::Running).await;
    pm.insert_server(second_server).await;
    advance_idle_ticks(2).await;
    entered.notified().await;
    assert_eq!(stops.load(Ordering::SeqCst), 2);

    let competing_manual = tokio::spawn({
        let supervisor = supervisor.clone();
        let pm = pm.clone();
        let sink = sink.clone();
        async move {
            supervisor
                .put_to_sleep(&pm, "server", move |event| sink(event))
                .await
        }
    });
    drive_tasks().await;
    assert_eq!(stops.load(Ordering::SeqCst), 2);

    finish_stop.notify_one();
    assert!(matches!(
        competing_manual.await.unwrap(),
        Err(ProcessError::MustBeRunningToSleep)
    ));
    drive_tasks().await;
    assert_eq!(stops.load(Ordering::SeqCst), 2);
    assert_eq!(pm.get_server_status("server").await, ServerStatus::Sleeping);
    assert_eq!(events.lock().unwrap().len(), 2);

    supervisor.release("server").await;
    assert!(!proxy_running.load(Ordering::SeqCst));
}

#[tokio::test(start_paused = true)]
async fn release_stop_supervised_replacement_and_restart_cancel_inflight_sleep() {
    let entered = Arc::new(Notify::new());
    let finish_stop = Arc::new(Notify::new());
    let supervisor = ServerSupervisorManager::with_test_stop(sleep_stopper({
        let entered = entered.clone();
        let finish_stop = finish_stop.clone();
        move |pm, server_id, _grace| {
            let pm = pm.clone();
            let server_id = server_id.to_owned();
            let entered = entered.clone();
            let finish_stop = finish_stop.clone();
            Box::pin(async move {
                pm.set_server_status(&server_id, ServerStatus::Stopping)
                    .await;
                pm.remove_if_pid_matches(&server_id, 10).await;
                entered.notify_one();
                finish_stop.notified().await;
                Ok(())
            })
        }
    }));

    let pm = ServerProcessManager::new();
    let (events, sink) = recording_status_sink();

    // (a) release() while supervised put_to_sleep is in flight
    pm.insert_server(running_server("server", 10).await).await;
    let (proxy_handle, proxy_running) = proxy_fixture("server").await;
    supervisor
        .supervise_idle_for_test(
            pm.clone(),
            server_config("server", 60),
            60,
            proxy_handle.active_connections.clone(),
            Some(proxy_handle),
            sink.clone(),
        )
        .await;
    let task = tokio::spawn({
        let supervisor = supervisor.clone();
        let pm = pm.clone();
        let sink = sink.clone();
        async move {
            supervisor
                .put_to_sleep(&pm, "server", move |event| sink(event))
                .await
        }
    });
    entered.notified().await;
    supervisor.release("server").await;
    assert!(!proxy_running.load(Ordering::SeqCst));
    finish_stop.notify_one();
    assert!(matches!(
        task.await.unwrap(),
        Err(ProcessError::LaunchCancelled)
    ));
    assert_eq!(pm.get_server_status("server").await, ServerStatus::Stopped);
    assert!(pm.sleeping_ids().await.is_empty());
    assert!(events.lock().unwrap().is_empty());

    // (b) stop_supervised() while unsupervised put_to_sleep is in flight
    pm.insert_server(running_server("server", 10).await).await;
    let task = tokio::spawn({
        let supervisor = supervisor.clone();
        let pm = pm.clone();
        let sink = sink.clone();
        async move {
            supervisor
                .put_to_sleep(&pm, "server", move |event| sink(event))
                .await
        }
    });
    entered.notified().await;
    supervisor.stop_supervised(&pm, "server").await.unwrap();
    finish_stop.notify_one();
    assert!(matches!(
        task.await.unwrap(),
        Err(ProcessError::LaunchCancelled)
    ));
    assert_eq!(pm.get_server_status("server").await, ServerStatus::Stopped);
    assert!(pm.sleeping_ids().await.is_empty());
    assert!(events.lock().unwrap().is_empty());

    // (c) replacement supervised generation supersedes in-flight put_to_sleep
    pm.insert_server(running_server("server", 10).await).await;
    let (proxy_a, _proxy_a_running) = proxy_fixture("server").await;
    let gen_a = supervisor
        .supervise_idle_for_test(
            pm.clone(),
            server_config("server", 60),
            60,
            proxy_a.active_connections.clone(),
            Some(proxy_a),
            sink.clone(),
        )
        .await;
    let task = tokio::spawn({
        let supervisor = supervisor.clone();
        let pm = pm.clone();
        let sink = sink.clone();
        async move {
            supervisor
                .put_to_sleep(&pm, "server", move |event| sink(event))
                .await
        }
    });
    entered.notified().await;
    let (proxy_b, proxy_b_running) = proxy_fixture("server").await;
    let gen_b = supervisor
        .supervise_idle_for_test(
            pm.clone(),
            server_config("server", 60),
            60,
            proxy_b.active_connections.clone(),
            Some(proxy_b),
            sink.clone(),
        )
        .await;
    assert_ne!(gen_a, gen_b);
    finish_stop.notify_one();
    assert!(matches!(
        task.await.unwrap(),
        Err(ProcessError::LaunchCancelled)
    ));
    assert_eq!(supervisor.generation_of("server").await, Some(gen_b));
    assert!(proxy_b_running.load(Ordering::SeqCst));
    assert!(pm.sleeping_ids().await.is_empty());
    assert!(events.lock().unwrap().is_empty());

    // (d) concurrent claim_launch in the gap prevents late Sleeping commit
    supervisor.release("server").await;
    assert!(!proxy_b_running.load(Ordering::SeqCst));
    pm.insert_server(running_server("server", 10).await).await;
    let task = tokio::spawn({
        let supervisor = supervisor.clone();
        let pm = pm.clone();
        let sink = sink.clone();
        async move {
            supervisor
                .put_to_sleep(&pm, "server", move |event| sink(event))
                .await
        }
    });
    entered.notified().await;
    let _restarting = pm.claim_launch("server").await.unwrap();
    finish_stop.notify_one();
    assert!(matches!(
        task.await.unwrap(),
        Err(ProcessError::LaunchCancelled)
    ));
    assert_eq!(pm.get_server_status("server").await, ServerStatus::Starting);
    assert!(pm.sleeping_ids().await.is_empty());
    assert!(events.lock().unwrap().is_empty());
}
