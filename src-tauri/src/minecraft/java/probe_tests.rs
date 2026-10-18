use super::discovery::{executable_name, resolve_probe_binary};
use super::install::{find_existing, ArchiveFormat, RuntimeRequest};
use super::probe::{JavaProbe, JavaProbeFuture, NativeProbe, ProbeChild, ProbeExit, ProcessRunner};
use super::JavaError;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::AsyncRead;

#[derive(Clone)]
struct FakeProcessConfig {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exit: Result<ProbeExit, std::io::ErrorKind>,
    hang: bool,
    kill_error: Option<std::io::ErrorKind>,
    cleanup_hangs: bool,
    spawn_error: Option<std::io::ErrorKind>,
    entered_tx: Option<Arc<tokio::sync::Notify>>,
}

impl Default for FakeProcessConfig {
    fn default() -> Self {
        Self {
            stdout: Vec::new(),
            stderr: Vec::new(),
            exit: Ok(ProbeExit::success()),
            hang: false,
            kill_error: None,
            cleanup_hangs: false,
            spawn_error: None,
            entered_tx: None,
        }
    }
}

struct FakeRunner {
    config: Mutex<FakeProcessConfig>,
    killed: Arc<AtomicBool>,
    reaped: Arc<AtomicBool>,
    cleaned: Arc<tokio::sync::Notify>,
}

impl FakeRunner {
    fn new(config: FakeProcessConfig) -> (Arc<Self>, Arc<AtomicBool>, Arc<AtomicBool>) {
        let killed = Arc::new(AtomicBool::new(false));
        let reaped = Arc::new(AtomicBool::new(false));
        let runner = Arc::new(Self {
            config: Mutex::new(config),
            killed: killed.clone(),
            reaped: reaped.clone(),
            cleaned: Arc::new(tokio::sync::Notify::new()),
        });
        (runner, killed, reaped)
    }
}

impl ProcessRunner for FakeRunner {
    fn spawn(
        &self,
        _command: &Path,
        _args: &[&str],
    ) -> Result<Box<dyn ProbeChild>, std::io::Error> {
        let config = self.config.lock().unwrap().clone();
        if let Some(err_kind) = config.spawn_error {
            return Err(std::io::Error::new(err_kind, "spawn failed"));
        }
        if let Some(ref notify) = config.entered_tx {
            notify.notify_one();
        }
        Ok(Box::new(FakeChild {
            stdout: Some(config.stdout),
            stderr: Some(config.stderr),
            exit: config.exit,
            hang: config.hang,
            kill_error: config.kill_error,
            cleanup_hangs: config.cleanup_hangs,
            killed: self.killed.clone(),
            reaped: self.reaped.clone(),
            cleaned: self.cleaned.clone(),
        }))
    }
}

struct FakeChild {
    stdout: Option<Vec<u8>>,
    stderr: Option<Vec<u8>>,
    exit: Result<ProbeExit, std::io::ErrorKind>,
    hang: bool,
    kill_error: Option<std::io::ErrorKind>,
    cleanup_hangs: bool,
    killed: Arc<AtomicBool>,
    reaped: Arc<AtomicBool>,
    cleaned: Arc<tokio::sync::Notify>,
}

impl ProbeChild for FakeChild {
    fn take_stdout(&mut self) -> Option<Box<dyn AsyncRead + Send + Unpin>> {
        self.stdout
            .take()
            .map(|bytes| Box::new(Cursor::new(bytes)) as Box<dyn AsyncRead + Send + Unpin>)
    }

    fn take_stderr(&mut self) -> Option<Box<dyn AsyncRead + Send + Unpin>> {
        self.stderr
            .take()
            .map(|bytes| Box::new(Cursor::new(bytes)) as Box<dyn AsyncRead + Send + Unpin>)
    }

    fn wait<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ProbeExit, std::io::Error>> + Send + 'a>>
    {
        let hang = self.hang;
        let exit = self.exit;
        let reaped = self.reaped.clone();
        Box::pin(async move {
            if hang {
                std::future::pending::<()>().await;
            }
            reaped.store(true, Ordering::SeqCst);
            match exit {
                Ok(status) => Ok(status),
                Err(kind) => Err(std::io::Error::new(kind, "wait failed")),
            }
        })
    }

    fn terminate_and_reap(
        &mut self,
    ) -> Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + '_>> {
        Box::pin(async move {
            self.killed.store(true, Ordering::SeqCst);
            if self.cleanup_hangs {
                std::future::pending::<()>().await;
            }
            if let Some(kind) = self.kill_error {
                return Err(std::io::Error::new(kind, "cleanup termination failed"));
            }
            self.reaped.store(true, Ordering::SeqCst);
            self.cleaned.notify_one();
            Ok(())
        })
    }
}

fn fake_request(root: &Path) -> RuntimeRequest {
    RuntimeRequest {
        data_dir: root.to_path_buf(),
        major_version: 21,
        custom_path: None,
        system_roots: Vec::new(),
        download_url: "http://127.0.0.1:1/not-requested".into(),
        archive_format: ArchiveFormat::Zip,
    }
}

fn create_fake_binary(root: &Path) -> PathBuf {
    let path = root.join("bin").join(executable_name());
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"fake binary").unwrap();
    path
}

#[tokio::test]
async fn successful_version_parsing_from_stdout_and_stderr() {
    let dummy_path = PathBuf::from("java");

    let stdout_config = FakeProcessConfig {
        stdout: b"openjdk version \"21.0.6\" 2025-01-21\n".to_vec(),
        ..FakeProcessConfig::default()
    };
    let (runner, _, _) = FakeRunner::new(stdout_config);
    let probe = NativeProbe::with_runner(runner);
    let version = probe.major_version(&dummy_path).await.unwrap();
    assert_eq!(version, 21);

    let stderr_config = FakeProcessConfig {
        stderr: b"java version \"1.8.0_351\"\nJava(TM) SE Runtime Environment\n".to_vec(),
        ..FakeProcessConfig::default()
    };
    let (runner, _, _) = FakeRunner::new(stderr_config);
    let probe = NativeProbe::with_runner(runner);
    let version = probe.major_version(&dummy_path).await.unwrap();
    assert_eq!(version, 8);
}

#[tokio::test]
async fn spawn_failure_and_unsuccessful_exit() {
    let dummy_path = PathBuf::from("java");

    let spawn_fail_config = FakeProcessConfig {
        spawn_error: Some(std::io::ErrorKind::NotFound),
        ..FakeProcessConfig::default()
    };
    let (runner, killed, _) = FakeRunner::new(spawn_fail_config);
    let probe = NativeProbe::with_runner(runner);
    let err = probe.major_version(&dummy_path).await.unwrap_err();
    assert!(matches!(err, JavaError::ProbeSpawn(e) if e.kind() == std::io::ErrorKind::NotFound));
    assert!(!killed.load(Ordering::SeqCst));

    let exit_fail_config = FakeProcessConfig {
        exit: Ok(ProbeExit::failure(1)),
        ..FakeProcessConfig::default()
    };
    let (runner, killed, reaped) = FakeRunner::new(exit_fail_config);
    let probe = NativeProbe::with_runner(runner);
    let err = probe.major_version(&dummy_path).await.unwrap_err();
    assert!(matches!(err, JavaError::ProbeFailed));
    assert!(reaped.load(Ordering::SeqCst));
    assert!(!killed.load(Ordering::SeqCst));
}

#[tokio::test]
async fn hanging_probe_times_out_and_is_reaped() {
    tokio::time::pause();
    let dummy_path = PathBuf::from("java");

    let hang_config = FakeProcessConfig {
        hang: true,
        ..FakeProcessConfig::default()
    };
    let (runner, killed, reaped) = FakeRunner::new(hang_config);
    let probe = NativeProbe::with_options(Duration::from_millis(100), 1024, runner);

    let probe_task = tokio::spawn(async move { probe.major_version(&dummy_path).await });

    tokio::time::advance(Duration::from_millis(150)).await;
    let result = probe_task.await.unwrap();

    assert!(matches!(
        result,
        Err(JavaError::ProbeTimeout(d)) if d == Duration::from_millis(100)
    ));
    assert!(killed.load(Ordering::SeqCst));
    assert!(reaped.load(Ordering::SeqCst));
}

#[tokio::test]
async fn cancellation_cleans_up_the_probe() {
    let dummy_path = PathBuf::from("java");
    let notify = Arc::new(tokio::sync::Notify::new());

    let hang_config = FakeProcessConfig {
        hang: true,
        entered_tx: Some(notify.clone()),
        ..FakeProcessConfig::default()
    };
    let (runner, killed, reaped) = FakeRunner::new(hang_config);
    let cleaned = runner.cleaned.clone();
    let probe = NativeProbe::with_options(Duration::from_secs(10), 1024, runner);

    let probe_task = tokio::spawn(async move { probe.major_version(&dummy_path).await });

    notify.notified().await;
    probe_task.abort();
    let _ = probe_task.await;
    cleaned.notified().await;

    assert!(killed.load(Ordering::SeqCst));
    assert!(reaped.load(Ordering::SeqCst));
}

#[tokio::test]
async fn excessive_output_is_bounded() {
    let dummy_path = PathBuf::from("java");

    let mut stdout_data = b"openjdk version \"17.0.2\"\n".to_vec();
    stdout_data.extend(vec![b'A'; 100_000]);
    let stderr_data = vec![b'B'; 100_000];

    let config = FakeProcessConfig {
        stdout: stdout_data,
        stderr: stderr_data,
        ..FakeProcessConfig::default()
    };
    let (runner, _, reaped) = FakeRunner::new(config);
    let probe = NativeProbe::with_options(Duration::from_secs(5), 256, runner);

    let version = probe.major_version(&dummy_path).await.unwrap();
    assert_eq!(version, 17);
    assert!(reaped.load(Ordering::SeqCst));

    let garbage_config = FakeProcessConfig {
        stdout: vec![b'X'; 50_000],
        stderr: vec![b'Y'; 50_000],
        ..FakeProcessConfig::default()
    };
    let (runner, _, _) = FakeRunner::new(garbage_config);
    let probe = NativeProbe::with_options(Duration::from_secs(5), 256, runner);

    let err = probe.major_version(&dummy_path).await.unwrap_err();
    assert!(matches!(err, JavaError::InvalidVersion));
}

#[tokio::test]
async fn termination_cleanup_failure_is_reported() {
    tokio::time::pause();
    let dummy_path = PathBuf::from("java");

    let hang_config = FakeProcessConfig {
        hang: true,
        kill_error: Some(std::io::ErrorKind::PermissionDenied),
        ..FakeProcessConfig::default()
    };
    let (runner, _, _) = FakeRunner::new(hang_config);
    let probe = NativeProbe::with_options(Duration::from_millis(50), 1024, runner);

    let dummy_clone = dummy_path.clone();
    let probe_task = tokio::spawn(async move { probe.major_version(&dummy_clone).await });
    tokio::time::advance(Duration::from_millis(80)).await;
    let err = probe_task.await.unwrap().unwrap_err();

    match err {
        JavaError::ProbeCleanup { primary, cleanup } => {
            assert!(
                matches!(primary.as_deref(), Some(JavaError::ProbeTimeout(d)) if *d == Duration::from_millis(50))
            );
            assert_eq!(cleanup.kind(), std::io::ErrorKind::PermissionDenied);
            let display_str = format!("{}", JavaError::ProbeCleanup { primary, cleanup });
            assert!(display_str.contains("primary failure"));
        }
        other => panic!("expected ProbeCleanup error, got: {other:?}"),
    }

    let fail_config = FakeProcessConfig {
        exit: Err(std::io::ErrorKind::BrokenPipe),
        kill_error: Some(std::io::ErrorKind::BrokenPipe),
        ..FakeProcessConfig::default()
    };
    let (runner, _, _) = FakeRunner::new(fail_config);
    let probe = NativeProbe::with_runner(runner);
    let err = probe.major_version(&dummy_path).await.unwrap_err();

    match err {
        JavaError::ProbeCleanup { primary, cleanup } => {
            assert!(matches!(primary.as_deref(), Some(JavaError::Io(_))));
            assert_eq!(cleanup.kind(), std::io::ErrorKind::BrokenPipe);
        }
        other => panic!("expected ProbeCleanup error, got: {other:?}"),
    }
}

#[tokio::test]
async fn wrong_major_candidates_follow_the_documented_selection_policy() {
    struct TestCandidateProbe {
        results: Mutex<Vec<(PathBuf, Result<u32, JavaError>)>>,
    }
    impl JavaProbe for TestCandidateProbe {
        fn major_version<'a>(&'a self, path: &'a Path) -> JavaProbeFuture<'a> {
            let res = self
                .results
                .lock()
                .unwrap()
                .iter()
                .find(|(p, _)| p == path)
                .map(|(_, r)| match r {
                    Ok(v) => Ok(*v),
                    Err(JavaError::ProbeTimeout(d)) => Err(JavaError::ProbeTimeout(*d)),
                    Err(JavaError::ProbeFailed) => Err(JavaError::ProbeFailed),
                    Err(_) => Err(JavaError::InvalidVersion),
                })
                .unwrap_or(Err(JavaError::MissingBinary));
            Box::pin(async move { res })
        }
    }

    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("jdk_roots");
    let jdk17 = create_fake_binary(&root.join("jdk-17"));
    let jdk21 = create_fake_binary(&root.join("jdk-21"));
    let jdk25 = create_fake_binary(&root.join("jdk-25"));

    let probe = TestCandidateProbe {
        results: Mutex::new(vec![
            (jdk17.clone(), Ok(17)),
            (jdk21.clone(), Ok(21)),
            (jdk25.clone(), Ok(25)),
        ]),
    };

    let mut request = fake_request(temp.path());
    request.system_roots.push(root.clone());
    let selected = find_existing(&request, &probe).await.unwrap();
    assert_eq!(selected, Some(jdk21.clone()));

    let failing_probe = TestCandidateProbe {
        results: Mutex::new(vec![
            (
                jdk17.clone(),
                Err(JavaError::ProbeTimeout(Duration::from_secs(1))),
            ),
            (jdk21.clone(), Ok(21)),
        ]),
    };
    let mut req_fail = fake_request(temp.path());
    req_fail.system_roots.push(root.clone());
    let err = find_existing(&req_fail, &failing_probe).await.unwrap_err();
    assert!(matches!(err, JavaError::ProbeTimeout(_)));

    let mut req_custom = fake_request(temp.path());
    req_custom.custom_path = Some(jdk17.clone());
    let err = find_existing(&req_custom, &probe).await.unwrap_err();
    assert!(matches!(
        err,
        JavaError::VersionMismatch {
            required: 21,
            actual: 17
        }
    ));

    let no_match_probe = TestCandidateProbe {
        results: Mutex::new(vec![
            (jdk17.clone(), Ok(17)),
            (jdk21.clone(), Ok(17)),
            (jdk25.clone(), Ok(25)),
        ]),
    };
    let mut req_none = fake_request(temp.path());
    req_none.system_roots.push(root);
    let res = find_existing(&req_none, &no_match_probe).await.unwrap();
    assert_eq!(res, None);
}

#[test]
fn resolve_probe_binary_prefers_java_exe_on_windows() {
    let temp = tempfile::tempdir().unwrap();
    let bin_dir = temp.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    let javaw = bin_dir.join(executable_name());
    std::fs::write(&javaw, b"fake javaw").unwrap();

    let resolved_before = resolve_probe_binary(&javaw);
    if cfg!(windows) {
        assert_eq!(resolved_before, javaw);
    }

    let java_exe = bin_dir.join("java.exe");
    std::fs::write(&java_exe, b"fake java").unwrap();

    let resolved_after = resolve_probe_binary(&javaw);
    if cfg!(windows) {
        assert_eq!(resolved_after, java_exe);
    } else {
        assert_eq!(resolved_after, javaw);
    }
}

#[tokio::test]
async fn native_adapter_hermetic_probe() {
    struct EchoRunner;
    impl ProcessRunner for EchoRunner {
        fn spawn(
            &self,
            _command: &Path,
            _args: &[&str],
        ) -> Result<Box<dyn ProbeChild>, std::io::Error> {
            let mut cmd = if cfg!(windows) {
                let mut c = tokio::process::Command::new("cmd.exe");
                c.args(["/C", "echo openjdk version \"21.0.1\" 2024-01-16"]);
                c
            } else {
                let mut c = tokio::process::Command::new("sh");
                c.args(["-c", "echo 'openjdk version \"21.0.1\" 2024-01-16'"]);
                c
            };
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());
            cmd.kill_on_drop(true);
            #[cfg(windows)]
            {
                cmd.creation_flags(0x08000000);
            }
            let child = cmd.spawn()?;
            struct DirectChild(tokio::process::Child);
            impl ProbeChild for DirectChild {
                fn take_stdout(&mut self) -> Option<Box<dyn AsyncRead + Send + Unpin>> {
                    self.0
                        .stdout
                        .take()
                        .map(|s| Box::new(s) as Box<dyn AsyncRead + Send + Unpin>)
                }
                fn take_stderr(&mut self) -> Option<Box<dyn AsyncRead + Send + Unpin>> {
                    self.0
                        .stderr
                        .take()
                        .map(|s| Box::new(s) as Box<dyn AsyncRead + Send + Unpin>)
                }
                fn wait<'a>(
                    &'a mut self,
                ) -> Pin<
                    Box<
                        dyn std::future::Future<Output = Result<ProbeExit, std::io::Error>>
                            + Send
                            + 'a,
                    >,
                > {
                    Box::pin(async move {
                        let status = self.0.wait().await?;
                        Ok(ProbeExit::from(status))
                    })
                }
                fn terminate_and_reap(
                    &mut self,
                ) -> Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + '_>>
                {
                    Box::pin(async move {
                        if self.0.try_wait()?.is_none() {
                            self.0.start_kill()?;
                        }
                        self.0.wait().await?;
                        Ok(())
                    })
                }
            }
            Ok(Box::new(DirectChild(child)))
        }
    }

    let probe = NativeProbe::with_runner(Arc::new(EchoRunner));
    let version = probe.major_version(Path::new("java")).await.unwrap();
    assert_eq!(version, 21);
}

#[tokio::test(start_paused = true)]
async fn cleanup_itself_has_a_deadline_and_never_claims_reaping() {
    let config = FakeProcessConfig {
        hang: true,
        cleanup_hangs: true,
        ..FakeProcessConfig::default()
    };
    let (runner, killed, reaped) = FakeRunner::new(config);
    let probe = NativeProbe::with_options(Duration::from_millis(10), 1024, runner);
    let error = probe.major_version(Path::new("java")).await.unwrap_err();
    assert!(matches!(error, JavaError::ProbeCleanup { primary, cleanup }
        if matches!(primary.as_deref(), Some(JavaError::ProbeTimeout(_))) && cleanup.kind() == std::io::ErrorKind::TimedOut));
    assert!(killed.load(Ordering::SeqCst));
    assert!(!reaped.load(Ordering::SeqCst));
}
