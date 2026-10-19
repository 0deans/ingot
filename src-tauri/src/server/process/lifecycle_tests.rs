use super::lifecycle::{
    claim_server_launch, launch_claimed_server, setup_server_environment, ServerCommandBuilder,
    ServerLaunchContext, ServerLifecycleHooks,
};
use super::manager::ServerProcessManager;
use super::models::ProcessError;
use crate::minecraft::metadata::MetadataRepository;
use crate::server::config::{ServerConfig, ServerStatus, ServerStatusEvent};
use crate::server::supervisor::ServerSupervisorManager;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

pub(crate) type CommandFuture = std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<tokio::process::Command, ProcessError>> + Send>,
>;

struct FixtureCommand;
impl ServerCommandBuilder for FixtureCommand {
    fn build_command(
        &self,
        _data_dir: &Path,
        _client: &reqwest::Client,
        _config: &ServerConfig,
        _server_dir: &Path,
        _step: Arc<dyn Fn(&str) + Send + Sync>,
    ) -> CommandFuture {
        Box::pin(async {
            let mut command = tokio::process::Command::new(std::env::current_exe()?);
            command.args([
                "--ignored",
                "--exact",
                "server::process::lifecycle_tests::fixture_child",
                "--nocapture",
            ]);
            Ok(command)
        })
    }
}

struct FailedPreparation;
impl ServerCommandBuilder for FailedPreparation {
    fn build_command(
        &self,
        _data_dir: &Path,
        _client: &reqwest::Client,
        _config: &ServerConfig,
        _server_dir: &Path,
        _step: Arc<dyn Fn(&str) + Send + Sync>,
    ) -> CommandFuture {
        Box::pin(async {
            Err(ProcessError::Io(std::io::Error::other(
                "injected preparation failure",
            )))
        })
    }
}

struct ExitImmediately;
impl ServerCommandBuilder for ExitImmediately {
    fn build_command(
        &self,
        _data_dir: &Path,
        _client: &reqwest::Client,
        _config: &ServerConfig,
        _server_dir: &Path,
        _step: Arc<dyn Fn(&str) + Send + Sync>,
    ) -> CommandFuture {
        Box::pin(async {
            let mut command = tokio::process::Command::new(std::env::current_exe()?);
            command.arg("--list");
            Ok(command)
        })
    }
}

struct MissingExecutable;
impl ServerCommandBuilder for MissingExecutable {
    fn build_command(
        &self,
        _data_dir: &Path,
        _client: &reqwest::Client,
        _config: &ServerConfig,
        server_dir: &Path,
        _step: Arc<dyn Fn(&str) + Send + Sync>,
    ) -> CommandFuture {
        let missing = server_dir.join("missing-executable");
        Box::pin(async move { Ok(tokio::process::Command::new(missing)) })
    }
}

struct PendingPreparation {
    entered: Arc<Notify>,
}

struct ResumedPreparation {
    entered: Arc<Notify>,
    resume: Arc<Notify>,
}
impl ServerCommandBuilder for ResumedPreparation {
    fn build_command(
        &self,
        _data_dir: &Path,
        _client: &reqwest::Client,
        _config: &ServerConfig,
        _server_dir: &Path,
        _step: Arc<dyn Fn(&str) + Send + Sync>,
    ) -> CommandFuture {
        let entered = self.entered.clone();
        let resume = self.resume.clone();
        Box::pin(async move {
            entered.notify_one();
            resume.notified().await;
            let mut command = tokio::process::Command::new(std::env::current_exe()?);
            command.arg("--list");
            Ok(command)
        })
    }
}
impl ServerCommandBuilder for PendingPreparation {
    fn build_command(
        &self,
        _data_dir: &Path,
        _client: &reqwest::Client,
        _config: &ServerConfig,
        _server_dir: &Path,
        _step: Arc<dyn Fn(&str) + Send + Sync>,
    ) -> CommandFuture {
        let entered = self.entered.clone();
        Box::pin(async move {
            entered.notify_one();
            std::future::pending().await
        })
    }
}

#[derive(Default)]
struct RecordingHooks {
    transitions: Mutex<Vec<&'static str>>,
    exited: Notify,
}
impl ServerLifecycleHooks for RecordingHooks {
    fn on_process_started(&self, _id: &str, _pid: u32, _at: u64) {
        self.transitions.lock().unwrap().push("started");
    }
    fn on_process_exited(&self, _id: &str, _pid: u32) {
        self.transitions.lock().unwrap().push("exited");
        self.exited.notify_one();
    }
    fn on_crash(&self, _crash: crate::version_change::CrashReport) {}
}

fn config() -> ServerConfig {
    serde_json::from_value(serde_json::json!({
        "id": "server", "name": "Fixture", "core": "paper", "gameVersion": "1.20",
        "port": 25565, "createdAt": 0, "memoryMinMb": 1024, "memoryMaxMb": 2048,
        "sleepEnabled": false
    }))
    .unwrap()
}

pub(crate) struct Fixture {
    _directory: tempfile::TempDir,
    pub(crate) context: ServerLaunchContext,
    hooks: Arc<RecordingHooks>,
    ready: Arc<Notify>,
    stopped: Arc<Notify>,
    statuses: Arc<Mutex<Vec<ServerStatus>>>,
}
impl Fixture {
    pub(crate) fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let repository = Arc::new(MetadataRepository::new(
            directory.path().join("servers.json"),
        ));
        repository
            .update(|records| {
                records.push(config());
                Ok(())
            })
            .unwrap();
        let hooks = Arc::new(RecordingHooks::default());
        let ready = Arc::new(Notify::new());
        let ready_log = ready.clone();
        let stopped = Arc::new(Notify::new());
        let stopped_status = stopped.clone();
        let statuses = Arc::new(Mutex::new(Vec::new()));
        let statuses_log = statuses.clone();
        let context = ServerLaunchContext {
            repository,
            server_dir: directory.path().to_path_buf(),
            data_dir: directory.path().join("data"),
            processes: ServerProcessManager::new(),
            client: reqwest::Client::new(),
            hooks: hooks.clone(),
            command_builder: Arc::new(FixtureCommand),
            on_log: Arc::new(move |event| {
                if event.line.contains("Done (1.0s)") {
                    ready_log.notify_one();
                }
            }),
            on_status: Arc::new(move |event: ServerStatusEvent| {
                let is_stopped = event.status == ServerStatus::Stopped;
                statuses_log.lock().unwrap().push(event.status);
                if is_stopped {
                    stopped_status.notify_one();
                }
            }),
            publication: Default::default(),
        };
        Self {
            _directory: directory,
            context,
            hooks,
            ready,
            stopped,
            statuses,
        }
    }
    async fn launch(&self) -> u32 {
        let claimed =
            claim_server_launch(&self.context.repository, &self.context.processes, "server")
                .await
                .unwrap();
        launch_claimed_server(&self.context, &claimed)
            .await
            .unwrap()
    }
    async fn await_stopped(&self) {
        tokio::time::timeout(Duration::from_secs(10), self.stopped.notified())
            .await
            .unwrap();
        assert_eq!(
            self.context.processes.get_server_status("server").await,
            ServerStatus::Stopped
        );
    }
}

// Invoked only by FixtureCommand in a separate copy of this test executable.
#[test]
#[ignore = "hermetic child-process fixture, invoked by lifecycle tests"]
fn fixture_child() {
    use std::io::{BufRead, Write};
    println!("[Server thread/INFO]: Done (1.0s)! For help, type help");
    std::io::stdout().flush().unwrap();
    for line in std::io::stdin().lock().lines() {
        if line.unwrap().trim() == "stop" {
            break;
        }
    }
}

#[tokio::test]
async fn concurrent_claim_and_cancelled_preparation_release_ownership() {
    let mut fixture = Fixture::new();
    let entered = Arc::new(Notify::new());
    fixture.context.command_builder = Arc::new(PendingPreparation {
        entered: entered.clone(),
    });
    let context = fixture.context.clone();
    let task = tokio::spawn(async move {
        let claimed =
            claim_server_launch(&context.repository, &context.processes, "server").await?;
        launch_claimed_server(&context, &claimed).await
    });
    entered.notified().await;
    assert!(matches!(
        claim_server_launch(
            &fixture.context.repository,
            &fixture.context.processes,
            "server"
        )
        .await,
        Err(ProcessError::AlreadyStarting)
    ));
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(!fixture.context.processes.is_starting("server"));
    assert!(fixture.context.processes.process_ids().await.is_empty());
    assert!(claim_server_launch(
        &fixture.context.repository,
        &fixture.context.processes,
        "server"
    )
    .await
    .is_ok());
}

#[tokio::test]
async fn preparation_failure_is_hermetic_and_never_registers_a_process() {
    let mut fixture = Fixture::new();
    fixture.context.command_builder = Arc::new(FailedPreparation);
    let claimed = claim_server_launch(
        &fixture.context.repository,
        &fixture.context.processes,
        "server",
    )
    .await
    .unwrap();
    assert!(matches!(
        launch_claimed_server(&fixture.context, &claimed).await,
        Err(ProcessError::Io(_))
    ));
    drop(claimed);
    assert!(fixture.context.processes.process_ids().await.is_empty());
    assert_eq!(
        fixture.context.processes.get_server_status("server").await,
        ServerStatus::Stopped
    );
    assert!(fixture.hooks.transitions.lock().unwrap().is_empty());
    assert_eq!(
        *fixture.statuses.lock().unwrap(),
        vec![ServerStatus::Starting, ServerStatus::Stopped]
    );
}

#[test]
fn setup_write_failure_is_explicit_without_platform_specific_paths() {
    let directory = tempfile::tempdir().unwrap();
    let invalid = directory.path().join("file");
    std::fs::write(&invalid, b"file").unwrap();
    assert!(matches!(
        setup_server_environment(&invalid, &config()),
        Err(ProcessError::Io(_))
    ));
}

#[tokio::test]
async fn readiness_graceful_exit_and_restart_preserve_event_order() {
    let fixture = Fixture::new();
    fixture.launch().await;
    tokio::time::timeout(Duration::from_secs(10), fixture.ready.notified())
        .await
        .unwrap();
    let manager = &fixture.context.processes;
    assert!(manager
        .servers
        .lock()
        .await
        .get("server")
        .unwrap()
        .ready
        .load(std::sync::atomic::Ordering::SeqCst));
    manager
        .stop_server_within("server", Duration::from_secs(10))
        .await
        .unwrap();
    fixture.await_stopped().await;
    assert_eq!(
        *fixture.hooks.transitions.lock().unwrap(),
        vec!["started", "exited"]
    );
    assert_eq!(
        *fixture.statuses.lock().unwrap(),
        vec![
            ServerStatus::Starting,
            ServerStatus::Running,
            ServerStatus::Stopped
        ]
    );
    fixture.launch().await;
    manager.kill_server_typed("server").await.unwrap();
    fixture.await_stopped().await;
}

#[tokio::test]
async fn force_kill_keeps_exit_owner_alive_and_allows_restart() {
    let fixture = Fixture::new();
    fixture.launch().await;
    fixture
        .context
        .processes
        .kill_server_typed("server")
        .await
        .unwrap();
    fixture.await_stopped().await;
    assert_eq!(
        *fixture.hooks.transitions.lock().unwrap(),
        vec!["started", "exited"]
    );
    fixture.launch().await;
    fixture
        .context
        .processes
        .stop_server_within("server", Duration::from_secs(10))
        .await
        .unwrap();
    fixture.await_stopped().await;
}

#[tokio::test]
async fn proxy_bind_failure_reaps_owned_process() {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
    let fixture = Fixture::new();
    fixture
        .context
        .repository
        .update(|records| {
            records[0].port = listener.local_addr().unwrap().port();
            records[0].sleep_enabled = Some(true);
            records[0].internal_port = Some(25566);
            Ok(())
        })
        .unwrap();
    let claimed = claim_server_launch(
        &fixture.context.repository,
        &fixture.context.processes,
        "server",
    )
    .await
    .unwrap();
    let supervisor = ServerSupervisorManager::new();
    let result = supervisor
        .supervise_and_start(fixture.context.clone(), claimed)
        .await;
    assert!(matches!(result, Err(ProcessError::PortInUse { .. })));
    fixture.await_stopped().await;
    assert_eq!(
        *fixture.hooks.transitions.lock().unwrap(),
        vec!["started", "exited"]
    );
}

#[tokio::test]
async fn fresh_claim_reads_updated_settings_for_wake() {
    let fixture = Fixture::new();
    let old = claim_server_launch(
        &fixture.context.repository,
        &fixture.context.processes,
        "server",
    )
    .await
    .unwrap();
    drop(old);
    fixture
        .context
        .repository
        .update(|records| {
            records[0].memory_max_mb = 8192;
            Ok(())
        })
        .unwrap();
    let fresh = claim_server_launch(
        &fixture.context.repository,
        &fixture.context.processes,
        "server",
    )
    .await
    .unwrap();
    assert_eq!(fresh.config.memory_max_mb, 8192);
}

#[tokio::test]
async fn immediate_exit_cannot_publish_a_stale_registry_entry() {
    let mut fixture = Fixture::new();
    fixture.context.command_builder = Arc::new(ExitImmediately);
    fixture.launch().await;
    fixture.await_stopped().await;
    assert_eq!(
        *fixture.hooks.transitions.lock().unwrap(),
        vec!["started", "exited"]
    );
    assert_eq!(
        *fixture.statuses.lock().unwrap(),
        vec![
            ServerStatus::Starting,
            ServerStatus::Running,
            ServerStatus::Stopped
        ]
    );
    assert!(claim_server_launch(
        &fixture.context.repository,
        &fixture.context.processes,
        "server"
    )
    .await
    .is_ok());
}

#[tokio::test]
async fn spawn_failure_preserves_its_io_cause_and_releases_the_claim() {
    let mut fixture = Fixture::new();
    fixture.context.command_builder = Arc::new(MissingExecutable);
    let claimed = claim_server_launch(
        &fixture.context.repository,
        &fixture.context.processes,
        "server",
    )
    .await
    .unwrap();
    let error = launch_claimed_server(&fixture.context, &claimed)
        .await
        .unwrap_err();
    let ProcessError::SpawnFailed(source) = error else {
        panic!("expected spawn failure")
    };
    assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
    drop(claimed);
    assert!(fixture.context.processes.process_ids().await.is_empty());
    assert!(!fixture.context.processes.is_starting("server"));
}

#[tokio::test]
async fn stale_native_exit_monitor_cannot_remove_a_replacement() {
    let fixture = Fixture::new();
    let replacement = super::tests::server_fixture(20).await;
    fixture.context.processes.insert_server(replacement).await;
    let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
    command
        .arg("--list")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let child = command.spawn().unwrap();
    let pid = child.id().unwrap();
    let monitor = super::monitoring::spawn_exit_monitor(super::monitoring::ExitMonitorContext {
        pm: fixture.context.processes.clone(),
        child_arc: Arc::new(tokio::sync::Mutex::new(child)),
        readers: Vec::new(),
        server_id: "server".into(),
        server_dir: fixture.context.server_dir.clone(),
        server_name: "Fixture".into(),
        core: crate::server::config::ServerCoreType::Paper,
        pid,
        ready_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        hooks: fixture.hooks.clone(),
        on_status: fixture.context.on_status.clone(),
    });
    tokio::time::timeout(Duration::from_secs(10), monitor)
        .await
        .unwrap()
        .unwrap();
    assert!(fixture.context.processes.owns("server", 20).await);
    assert!(fixture.hooks.transitions.lock().unwrap().is_empty());
    assert!(fixture.statuses.lock().unwrap().is_empty());
}

#[tokio::test]
async fn stop_during_preparation_prevents_late_process_publication() {
    let mut fixture = Fixture::new();
    let entered = Arc::new(Notify::new());
    let resume = Arc::new(Notify::new());
    fixture.context.command_builder = Arc::new(ResumedPreparation {
        entered: entered.clone(),
        resume: resume.clone(),
    });
    let context = fixture.context.clone();
    let launch = tokio::spawn(async move {
        let claimed =
            claim_server_launch(&context.repository, &context.processes, "server").await?;
        launch_claimed_server(&context, &claimed).await
    });
    entered.notified().await;
    ServerSupervisorManager::new()
        .stop_supervised(&fixture.context.processes, "server")
        .await
        .unwrap();
    resume.notify_one();
    assert!(matches!(
        launch.await.unwrap(),
        Err(ProcessError::LaunchCancelled)
    ));
    assert!(fixture.context.processes.process_ids().await.is_empty());
    assert!(!fixture.context.processes.is_starting("server"));
    assert!(fixture.hooks.transitions.lock().unwrap().is_empty());
}

#[tokio::test]
async fn invalidated_supervisor_generation_blocks_a_prepared_spawn() {
    let mut fixture = Fixture::new();
    let entered = Arc::new(Notify::new());
    let resume = Arc::new(Notify::new());
    fixture.context.command_builder = Arc::new(ResumedPreparation {
        entered: entered.clone(),
        resume: resume.clone(),
    });
    let context = fixture.context.clone();
    let launch = tokio::spawn(async move {
        let claimed =
            claim_server_launch(&context.repository, &context.processes, "server").await?;
        launch_claimed_server(&context, &claimed).await
    });
    entered.notified().await;
    fixture.context.publication.cancel();
    resume.notify_one();
    assert!(matches!(
        launch.await.unwrap(),
        Err(ProcessError::LaunchCancelled)
    ));
    assert!(fixture.context.processes.process_ids().await.is_empty());
    assert!(fixture.hooks.transitions.lock().unwrap().is_empty());
}
