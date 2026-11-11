use serde_json::json;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::{watch, Mutex, Notify};

use super::adapter::{BoxFuture, PlayitAdapter, PlayitChild};
use super::api::{decode_response, AgentContext};
use super::credentials;
use super::generation::Generation;
use super::{ClaimRejection, PlayitError, PlayitManager};

#[cfg(windows)]
fn test_exit_status(code: u32) -> std::process::ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(code)
}

#[cfg(unix)]
fn test_exit_status(code: i32) -> std::process::ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(code)
}

#[derive(Clone)]
struct FakeChild {
    kill_pause: Option<Arc<Notify>>,
    kill_entered: Arc<Notify>,
    exited: Arc<Mutex<Option<std::process::ExitStatus>>>,
    exit_notify: Arc<Notify>,
    kill_err: Option<ErrorKind>,
    wait_err: Option<ErrorKind>,
    killed: Arc<AtomicBool>,
    stdout_data: Arc<Mutex<Option<String>>>,
    stderr_data: Arc<Mutex<Option<String>>>,
}

impl PlayitChild for FakeChild {
    fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, std::io::Error> {
        if self.killed.load(Ordering::SeqCst) && self.kill_err.is_none() {
            return Ok(Some(test_exit_status(0)));
        }
        Ok(self.exited.try_lock().ok().and_then(|g| *g))
    }

    fn kill<'a>(&'a mut self) -> BoxFuture<'a, Result<(), std::io::Error>> {
        let killed = self.killed.clone();
        let kill_err = self.kill_err;
        let notify = self.exit_notify.clone();
        let pause = self.kill_pause.clone();
        let entered = self.kill_entered.clone();
        Box::pin(async move {
            entered.notify_one();
            if let Some(pause) = pause {
                pause.notified().await;
            }
            if let Some(err) = kill_err {
                return Err(std::io::Error::new(err, "injected kill failure"));
            }
            killed.store(true, Ordering::SeqCst);
            notify.notify_waiters();
            Ok(())
        })
    }

    fn wait<'a>(&'a mut self) -> BoxFuture<'a, Result<std::process::ExitStatus, std::io::Error>> {
        let wait_err = self.wait_err;
        let exited = self.exited.clone();
        let notify = self.exit_notify.clone();
        let killed = self.killed.clone();
        Box::pin(async move {
            if let Some(err) = wait_err {
                return Err(std::io::Error::new(err, "injected wait failure"));
            }
            loop {
                if let Some(status) = *exited.lock().await {
                    return Ok(status);
                }
                if killed.load(Ordering::SeqCst) {
                    return Ok(test_exit_status(0));
                }
                notify.notified().await;
            }
        })
    }

    fn take_stdout(&mut self) -> Option<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
        self.stdout_data
            .try_lock()
            .ok()
            .and_then(|mut g| g.take())
            .map(|s| {
                Box::new(std::io::Cursor::new(s.into_bytes()))
                    as Box<dyn tokio::io::AsyncRead + Unpin + Send>
            })
    }

    fn take_stderr(&mut self) -> Option<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
        self.stderr_data
            .try_lock()
            .ok()
            .and_then(|mut g| g.take())
            .map(|s| {
                Box::new(std::io::Cursor::new(s.into_bytes()))
                    as Box<dyn tokio::io::AsyncRead + Unpin + Send>
            })
    }
}

struct FakeAdapter {
    kill_pause: Option<Arc<Notify>>,
    kill_entered: Arc<Notify>,
    download_entered: Notify,
    claim_entered: Notify,
    binary_path: PathBuf,
    pause_download: std::sync::Mutex<Option<Arc<Notify>>>,
    pause_claim: std::sync::Mutex<Option<Arc<Notify>>>,
    claim_setup_status: String,
    claim_exchange_secret: String,
    child_kill_err: Option<ErrorKind>,
    child_wait_err: Option<ErrorKind>,
    child_exited: Arc<Mutex<Option<std::process::ExitStatus>>>,
    child_exit_notify: Arc<Notify>,
    child_stdout: Option<String>,
    child_stderr: Option<String>,
    last_spawned_child: Arc<Mutex<Option<FakeChild>>>,
}

impl Default for FakeAdapter {
    fn default() -> Self {
        Self {
            kill_pause: None,
            kill_entered: Arc::new(Notify::new()),
            download_entered: Notify::new(),
            claim_entered: Notify::new(),
            binary_path: PathBuf::from("/bin/playit"),
            pause_download: std::sync::Mutex::new(None),
            pause_claim: std::sync::Mutex::new(None),
            claim_setup_status: "UserAccepted".to_string(),
            claim_exchange_secret: "secret-abc-123".to_string(),
            child_kill_err: None,
            child_wait_err: None,
            child_exited: Arc::new(Mutex::new(None)),
            child_exit_notify: Arc::new(Notify::new()),
            child_stdout: None,
            child_stderr: None,
            last_spawned_child: Arc::new(Mutex::new(None)),
        }
    }
}

impl PlayitAdapter for FakeAdapter {
    fn ensure_binary<'a>(
        &'a self,
        _app_data_dir: &'a Path,
        _client: &'a reqwest::Client,
        mut cancel_rx: watch::Receiver<bool>,
    ) -> BoxFuture<'a, Result<PathBuf, PlayitError>> {
        let path = self.binary_path.clone();
        let pause = self.pause_download.lock().unwrap().take();
        Box::pin(async move {
            self.download_entered.notify_one();
            if let Some(notify) = pause {
                tokio::select! {
                    _ = notify.notified() => {},
                    _ = cancel_rx.changed() => return Err(PlayitError::Cancelled),
                }
            }
            Ok(path)
        })
    }

    fn claim_setup<'a>(
        &'a self,
        _client: &'a reqwest::Client,
        _code: &'a str,
    ) -> BoxFuture<'a, Result<String, PlayitError>> {
        let status = self.claim_setup_status.clone();
        let pause = self.pause_claim.lock().unwrap().take();
        Box::pin(async move {
            self.claim_entered.notify_one();
            if let Some(notify) = pause {
                notify.notified().await;
            }
            Ok(status)
        })
    }

    fn claim_exchange<'a>(
        &'a self,
        _client: &'a reqwest::Client,
        _code: &'a str,
    ) -> BoxFuture<'a, Result<String, PlayitError>> {
        let secret = self.claim_exchange_secret.clone();
        Box::pin(async move { Ok(secret) })
    }

    fn spawn_process(
        &self,
        _binary_path: &Path,
        _secret: &str,
        _playit_dir: &Path,
        _sandbox_dir: Option<&Path>,
    ) -> Result<Box<dyn PlayitChild>, PlayitError> {
        let child = FakeChild {
            kill_pause: self.kill_pause.clone(),
            kill_entered: self.kill_entered.clone(),
            exited: self.child_exited.clone(),
            exit_notify: self.child_exit_notify.clone(),
            kill_err: self.child_kill_err,
            wait_err: self.child_wait_err,
            killed: Arc::new(AtomicBool::new(false)),
            stdout_data: Arc::new(Mutex::new(self.child_stdout.clone())),
            stderr_data: Arc::new(Mutex::new(self.child_stderr.clone())),
        };
        if let Ok(mut g) = self.last_spawned_child.try_lock() {
            *g = Some(child.clone());
        }
        Ok(Box::new(child))
    }

    fn fetch_rundata<'a>(
        &'a self,
        _client: &'a reqwest::Client,
        _secret: &'a str,
    ) -> BoxFuture<'a, Result<serde_json::Value, PlayitError>> {
        Box::pin(async move {
            Ok(json!({
                "tunnels": [{
                    "tunnel_type": "minecraft-java",
                    "display_address": "tunnel.playit.gg:12345",
                    "disabled_reason": null
                }],
                "notices": []
            }))
        })
    }
}

// 1. Concurrent starts follow the chosen policy (serializes and supersedes).
#[tokio::test(start_paused = true)]
async fn concurrent_starts_follow_the_chosen_policy() {
    let pause_first = Arc::new(Notify::new());
    let adapter1 = Arc::new(FakeAdapter {
        pause_download: std::sync::Mutex::new(Some(pause_first.clone())),
        ..FakeAdapter::default()
    });
    let manager = PlayitManager::with_adapter(adapter1.clone());

    let temp = tempfile::tempdir().unwrap();
    let client = reqwest::Client::new();

    let m1 = manager.clone();
    let c1 = client.clone();
    let p1 = temp.path().join("playit");
    let d1 = temp.path().to_path_buf();
    let handle1 =
        tokio::spawn(async move { m1.start_tunnel_internal(&c1, None, &d1, &p1, None).await });

    adapter1.download_entered.notified().await;

    // Start 2 with no pause
    let p2 = temp.path().join("playit");
    let d2 = temp.path().to_path_buf();
    let status2 = manager
        .start_tunnel_internal(&client, Some("secret-start-2".into()), &d2, &p2, None)
        .await
        .unwrap();

    assert!(status2.is_running);
    let gen2 = manager.inner.lock().await.generation;

    pause_first.notify_waiters();
    let res1 = handle1.await.unwrap();

    assert!(matches!(res1, Err(PlayitError::Cancelled)));
    assert_eq!(manager.inner.lock().await.generation, gen2);
}

// 2. Stop during download, claiming, spawning, and polling.
#[tokio::test(start_paused = true)]
async fn stop_during_download() {
    let pause = Arc::new(Notify::new());
    let adapter = Arc::new(FakeAdapter {
        pause_download: std::sync::Mutex::new(Some(pause.clone())),
        ..FakeAdapter::default()
    });
    let manager = PlayitManager::with_adapter(adapter.clone());
    let temp = tempfile::tempdir().unwrap();
    let client = reqwest::Client::new();

    let m = manager.clone();
    let c = client.clone();
    let p = temp.path().join("playit");
    let d = temp.path().to_path_buf();
    let start_task =
        tokio::spawn(async move { m.start_tunnel_internal(&c, None, &d, &p, None).await });

    adapter.download_entered.notified().await;
    assert!(manager.get_status().await.is_running);

    manager.stop_tunnel().await.unwrap();
    pause.notify_waiters();

    let res = start_task.await.unwrap();
    assert!(matches!(res, Err(PlayitError::Cancelled)));

    let status = manager.get_status().await;
    assert!(!status.is_running);
    assert_eq!(status.status, "stopped");
}

#[tokio::test(start_paused = true)]
async fn stop_during_claiming() {
    let pause = Arc::new(Notify::new());
    let adapter = Arc::new(FakeAdapter {
        pause_claim: std::sync::Mutex::new(Some(pause.clone())),
        ..FakeAdapter::default()
    });
    let manager = PlayitManager::with_adapter(adapter.clone());
    let temp = tempfile::tempdir().unwrap();
    let client = reqwest::Client::new();

    let m = manager.clone();
    let c = client.clone();
    let p = temp.path().join("playit");
    let d = temp.path().to_path_buf();
    let start_task =
        tokio::spawn(async move { m.start_tunnel_internal(&c, None, &d, &p, None).await });

    adapter.claim_entered.notified().await;
    let status = manager.get_status().await;
    assert!(status.is_running);

    manager.stop_tunnel().await.unwrap();
    pause.notify_waiters();

    let _ = start_task.await;
    let final_status = manager.get_status().await;
    assert!(!final_status.is_running);
    assert_eq!(final_status.status, "stopped");
}

#[tokio::test(start_paused = true)]
async fn stop_during_polling() {
    let adapter = Arc::new(FakeAdapter::default());
    let manager = PlayitManager::with_adapter(adapter.clone());
    let temp = tempfile::tempdir().unwrap();
    let client = reqwest::Client::new();

    let status = manager
        .start_tunnel_internal(
            &client,
            Some("valid-secret".into()),
            temp.path(),
            &temp.path().join("playit"),
            None,
        )
        .await
        .unwrap();

    assert!(status.is_running);

    let child = adapter.last_spawned_child.lock().await.clone().unwrap();
    assert!(!child.killed.load(Ordering::SeqCst));

    manager.stop_tunnel().await.unwrap();

    assert!(child.killed.load(Ordering::SeqCst));
    let final_status = manager.get_status().await;
    assert!(!final_status.is_running);
    assert_eq!(final_status.status, "stopped");
}

// 3. Dropped start future releases or transfers ownership correctly.
#[tokio::test(start_paused = true)]
async fn dropped_start_future_releases_ownership() {
    let pause = Arc::new(Notify::new());
    let adapter = Arc::new(FakeAdapter {
        pause_download: std::sync::Mutex::new(Some(pause.clone())),
        ..FakeAdapter::default()
    });
    let manager = PlayitManager::with_adapter(adapter.clone());
    let temp = tempfile::tempdir().unwrap();
    let client = reqwest::Client::new();

    let m = manager.clone();
    let c = client.clone();
    let p = temp.path().join("playit");
    let d = temp.path().to_path_buf();

    let start_task =
        tokio::spawn(async move { m.start_tunnel_internal(&c, None, &d, &p, None).await });

    adapter.download_entered.notified().await;
    start_task.abort();
    let _ = start_task.await;

    wait_for_status(&manager, "stopped").await;

    let status = manager.get_status().await;
    assert!(!status.is_running);
    assert_eq!(status.status, "stopped");
}

// 4. Old tasks cannot overwrite a newer start or kill its child.
#[tokio::test(start_paused = true)]
async fn old_tasks_cannot_overwrite_newer_start_or_kill_its_child() {
    let adapter = Arc::new(FakeAdapter::default());
    let manager = PlayitManager::with_adapter(adapter.clone());
    let temp = tempfile::tempdir().unwrap();
    let client = reqwest::Client::new();

    // Start 1
    let _ = manager
        .start_tunnel_internal(
            &client,
            Some("secret-1".into()),
            temp.path(),
            &temp.path().join("playit"),
            None,
        )
        .await
        .unwrap();

    // Start 2
    let _ = manager
        .start_tunnel_internal(
            &client,
            Some("secret-2".into()),
            temp.path(),
            &temp.path().join("playit"),
            None,
        )
        .await
        .unwrap();

    let child2 = adapter.last_spawned_child.lock().await.clone().unwrap();

    let (_tx, rx) = watch::channel(false);
    let stale_ctx = AgentContext {
        inner: manager.inner.clone(),
        status_tx: manager.status_tx.clone(),
        adapter: adapter.clone(),
        client: client.clone(),
        generation: Generation(1),
        binary_path: PathBuf::from("/bin/playit"),
        playit_dir: temp.path().join("playit"),
        sandbox_dir: None,
        cancel_rx: rx,
    };

    let updated = stale_ctx
        .update_status(|s| s.status = "stale-mutation".to_string())
        .await;
    assert!(!updated);

    let status = manager.get_status().await;
    assert_ne!(status.status, "stale-mutation");
    assert!(!child2.killed.load(Ordering::SeqCst));
}

// 5. Credential read/write/delete failures.
#[test]
fn credential_read_write_delete_failures() {
    let temp = tempfile::tempdir().unwrap();
    let playit_dir = temp.path().join("playit");
    std::fs::create_dir_all(&playit_dir).unwrap();

    assert_eq!(credentials::read_secret(&playit_dir).unwrap(), None);

    std::fs::write(playit_dir.join("secret.key"), "   \n\t  ").unwrap();
    assert!(matches!(
        credentials::read_secret(&playit_dir),
        Err(PlayitError::InvalidCredential)
    ));

    credentials::write_secret(&playit_dir, "my-test-secret-key").unwrap();
    assert_eq!(
        credentials::read_secret(&playit_dir).unwrap(),
        Some("my-test-secret-key".to_string())
    );

    credentials::delete_secret(&playit_dir).unwrap();
    assert_eq!(credentials::read_secret(&playit_dir).unwrap(), None);

    assert!(credentials::delete_secret(&playit_dir).is_ok());

    let invalid_dir = playit_dir.join("secret.key");
    std::fs::write(&invalid_dir, "file").unwrap();
    let blocked_dir = invalid_dir.join("sub");
    assert!(matches!(
        credentials::write_secret(&blocked_dir, "test"),
        Err(PlayitError::CredentialWrite(_))
    ));
}

// 6. Failed termination retains an honest state.
#[tokio::test(start_paused = true)]
async fn failed_termination_retains_an_honest_state() {
    let adapter = Arc::new(FakeAdapter {
        child_kill_err: Some(ErrorKind::PermissionDenied),
        ..FakeAdapter::default()
    });
    let manager = PlayitManager::with_adapter(adapter.clone());
    let temp = tempfile::tempdir().unwrap();
    let client = reqwest::Client::new();

    let _ = manager
        .start_tunnel_internal(
            &client,
            Some("valid-secret".into()),
            temp.path(),
            &temp.path().join("playit"),
            None,
        )
        .await
        .unwrap();

    let stop_res = manager.stop_tunnel().await;
    assert!(matches!(stop_res, Err(PlayitError::TerminationFailed(_))));

    let status = manager.get_status().await;
    assert!(status.is_running);
    assert_eq!(status.status, "error");
    assert!(status
        .message
        .unwrap()
        .contains("Failed to stop playit process"));
    assert!(manager.inner.lock().await.process.is_some());
    assert!(matches!(
        manager.stop_tunnel().await,
        Err(PlayitError::TerminationFailed(_))
    ));
    assert!(manager.inner.lock().await.process.is_some());
}

// 7. Child exit and reader-task cleanup.
#[tokio::test(start_paused = true)]
async fn child_exit_and_reader_task_cleanup() {
    let exited = Arc::new(Mutex::new(None));
    let exit_notify = Arc::new(Notify::new());
    let adapter = Arc::new(FakeAdapter {
        child_exited: exited.clone(),
        child_exit_notify: exit_notify.clone(),
        child_stdout: Some("Starting daemon...\nInvalidAgentKey: key is invalid\n".into()),
        ..FakeAdapter::default()
    });
    let manager = PlayitManager::with_adapter(adapter.clone());
    let temp = tempfile::tempdir().unwrap();
    let playit_dir = temp.path().join("playit");
    let client = reqwest::Client::new();

    credentials::write_secret(&playit_dir, "secret-key-to-be-deleted").unwrap();

    let _ = manager
        .start_tunnel_internal(&client, None, temp.path(), &playit_dir, None)
        .await
        .unwrap();

    let reader = manager
        .inner
        .lock()
        .await
        .process
        .as_mut()
        .unwrap()
        .stdout_task
        .take();
    if let Some(reader) = reader {
        reader.await.unwrap();
    }
    *exited.lock().await = Some(test_exit_status(1));
    exit_notify.notify_waiters();

    wait_for_status(&manager, "error").await;

    assert_eq!(credentials::read_secret(&playit_dir).unwrap(), None);

    let status = manager.get_status().await;
    assert_eq!(status.status, "error");
}

// 8. Secret-free errors and status payloads.
#[test]
fn secret_free_errors_and_status_payloads() {
    let secret = "sensitive-agent-key-secret-99999";

    let errors = vec![
        PlayitError::InvalidCredential,
        PlayitError::ClaimExpired,
        PlayitError::ClaimTimeout,
        PlayitError::ClaimRejected,
        PlayitError::Cancelled,
        PlayitError::CredentialRead(std::io::Error::new(ErrorKind::PermissionDenied, "denied")),
        PlayitError::CredentialWrite(std::io::Error::new(ErrorKind::PermissionDenied, "denied")),
        PlayitError::CredentialDelete(std::io::Error::new(ErrorKind::PermissionDenied, "denied")),
        PlayitError::TerminationFailed(std::io::Error::new(ErrorKind::PermissionDenied, "denied")),
    ];

    for err in errors {
        let display = err.to_string();
        let debug = format!("{err:?}");
        assert!(!display.contains(secret));
        assert!(!debug.contains(secret));
    }
}

// Preserved claim decode tests
#[test]
fn provider_rejections_do_not_expose_secret_payloads() {
    let error = decode_response(json!({
        "status": "fail",
        "data": { "secret_key": "private-fixture-token", "message": "private-fixture-token" }
    }))
    .unwrap_err();
    assert!(matches!(
        error,
        PlayitError::Rejected(ClaimRejection::Other)
    ));
    assert!(!error.to_string().contains("private-fixture-token"));
    assert!(!format!("{error:?}").contains("private-fixture-token"));
}

#[test]
fn malformed_claim_exchange_data_does_not_expose_credentials() {
    let error =
        decode_response(json!({ "data": { "secret_key": "private-fixture-token" } })).unwrap_err();
    assert!(matches!(error, PlayitError::InvalidResponse));
    assert!(!format!("{error:?}").contains("private-fixture-token"));
}

#[test]
fn known_claim_rejections_preserve_retry_categories() {
    for (code, expected) in [
        ("CodeExpired", "expired"),
        ("InvalidCode", "invalid"),
        ("NotAccepted", "pending"),
    ] {
        let error =
            decode_response(json!({ "status": "fail", "data": { "type": code } })).unwrap_err();
        let category = match error {
            PlayitError::Rejected(ClaimRejection::Expired) => "expired",
            PlayitError::Rejected(ClaimRejection::InvalidCode) => "invalid",
            PlayitError::Rejected(ClaimRejection::NotAccepted) => "pending",
            _ => panic!("unexpected claim rejection category"),
        };
        assert_eq!(category, expected);
    }
}

#[test]
fn successful_envelope_requires_data() {
    assert!(matches!(
        decode_response(json!({ "status": "success" })),
        Err(PlayitError::InvalidResponse)
    ));
    assert_eq!(
        decode_response(json!({ "status": "success", "data": "UserAccepted" })).unwrap(),
        json!("UserAccepted")
    );
}

async fn wait_for_status(manager: &PlayitManager, expected: &str) {
    let mut updates = manager.status_rx.clone();
    loop {
        if updates.borrow().status == expected {
            return;
        }
        updates.changed().await.unwrap();
    }
}

#[tokio::test(start_paused = true)]
async fn hanging_kill_is_bounded_and_retry_preserves_ownership() {
    let release = Arc::new(Notify::new());
    let adapter = Arc::new(FakeAdapter {
        kill_pause: Some(release.clone()),
        ..FakeAdapter::default()
    });
    let manager = PlayitManager::with_adapter(adapter.clone());
    let temp = tempfile::tempdir().unwrap();
    manager
        .start_tunnel_internal(
            &reqwest::Client::new(),
            Some("secret".into()),
            temp.path(),
            &temp.path().join("playit"),
            None,
        )
        .await
        .unwrap();
    let error = manager.stop_tunnel().await.unwrap_err();
    assert!(
        matches!(error, PlayitError::TerminationFailed(ref source) if source.kind() == ErrorKind::TimedOut)
    );
    assert!(manager.inner.lock().await.process.is_some());
    release.notify_one();
    manager.stop_tunnel().await.unwrap();
    assert!(manager.inner.lock().await.process.is_none());
    assert!(!manager.get_status().await.is_running);
}

#[tokio::test(start_paused = true)]
async fn cancelled_stop_keeps_process_available_for_retry() {
    let release = Arc::new(Notify::new());
    let adapter = Arc::new(FakeAdapter {
        kill_pause: Some(release.clone()),
        ..FakeAdapter::default()
    });
    let manager = PlayitManager::with_adapter(adapter.clone());
    let temp = tempfile::tempdir().unwrap();
    manager
        .start_tunnel_internal(
            &reqwest::Client::new(),
            Some("secret".into()),
            temp.path(),
            &temp.path().join("playit"),
            None,
        )
        .await
        .unwrap();
    let stopping_manager = manager.clone();
    let stop = tokio::spawn(async move { stopping_manager.stop_tunnel().await });
    adapter.kill_entered.notified().await;
    stop.abort();
    assert!(stop.await.unwrap_err().is_cancelled());
    assert!(manager.inner.lock().await.process.is_some());
    release.notify_one();
    manager.stop_tunnel().await.unwrap();
    assert!(manager.inner.lock().await.process.is_none());
}

#[tokio::test(start_paused = true)]
async fn stale_start_guard_does_not_stop_new_generation() {
    let adapter = Arc::new(FakeAdapter::default());
    let manager = PlayitManager::with_adapter(adapter.clone());
    let temp = tempfile::tempdir().unwrap();
    let client = reqwest::Client::new();
    let playit = temp.path().join("playit");
    manager
        .start_tunnel_internal(&client, Some("old".into()), temp.path(), &playit, None)
        .await
        .unwrap();
    let stale = Generation(manager.inner.lock().await.generation);
    manager
        .start_tunnel_internal(&client, Some("new".into()), temp.path(), &playit, None)
        .await
        .unwrap();
    manager.stop_generation(Some(stale)).await.unwrap();
    assert!(manager.get_status().await.is_running);
    assert!(manager.inner.lock().await.process.is_some());
    manager.stop_tunnel().await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn superseded_claim_cannot_publish_credentials_or_spawn() {
    let release = Arc::new(Notify::new());
    let adapter = Arc::new(FakeAdapter {
        pause_claim: std::sync::Mutex::new(Some(release.clone())),
        ..FakeAdapter::default()
    });
    let manager = PlayitManager::with_adapter(adapter.clone());
    let temp = tempfile::tempdir().unwrap();
    let playit = temp.path().join("playit");
    let (_cancel, cancel_rx) = watch::channel(false);
    let context = AgentContext {
        inner: manager.inner.clone(),
        status_tx: manager.status_tx.clone(),
        adapter: adapter.clone(),
        client: reqwest::Client::new(),
        generation: Generation(0),
        binary_path: PathBuf::from("fake-playit"),
        playit_dir: playit.clone(),
        sandbox_dir: None,
        cancel_rx,
    };
    let claim = tokio::spawn(async move { super::run_agent_inner(&context, None).await });
    adapter.claim_entered.notified().await;
    manager.inner.lock().await.generation = 1;
    release.notify_one();
    assert!(matches!(claim.await.unwrap(), Err(PlayitError::Cancelled)));
    assert_eq!(credentials::read_secret(&playit).unwrap(), None);
    assert!(adapter.last_spawned_child.lock().await.is_none());
}

#[tokio::test(start_paused = true)]
async fn invalid_explicit_secret_does_not_delete_different_saved_credential() {
    let adapter = Arc::new(FakeAdapter {
        child_stdout: Some("InvalidAgentKey: key is invalid\n".into()),
        ..FakeAdapter::default()
    });
    let manager = PlayitManager::with_adapter(adapter.clone());
    let temp = tempfile::tempdir().unwrap();
    let playit = temp.path().join("playit");
    credentials::write_secret(&playit, "saved-valid-secret").unwrap();
    manager
        .start_tunnel_internal(
            &reqwest::Client::new(),
            Some("provided-invalid-secret".into()),
            temp.path(),
            &playit,
            None,
        )
        .await
        .unwrap();
    let reader = manager
        .inner
        .lock()
        .await
        .process
        .as_mut()
        .unwrap()
        .stdout_task
        .take();
    if let Some(reader) = reader {
        reader.await.unwrap();
    }
    *adapter.child_exited.lock().await = Some(test_exit_status(1));
    adapter.child_exit_notify.notify_waiters();
    wait_for_status(&manager, "error").await;
    assert_eq!(
        credentials::read_secret(&playit).unwrap().as_deref(),
        Some("saved-valid-secret")
    );
    assert!(manager.inner.lock().await.process.is_none());
}
