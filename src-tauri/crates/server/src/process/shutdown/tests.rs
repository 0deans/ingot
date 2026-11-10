use super::*;
use crate::server::process::tests::server_fixture;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Notify;

type FakeOperation = dyn Fn(ProcessIdentity) -> StopFuture<'static> + Send + Sync;
struct FakeStop(Box<FakeOperation>);
impl StopEffect for FakeStop {
    fn terminate(&self, identity: ProcessIdentity) -> StopFuture<'_> {
        self.0(identity)
    }
}
fn termination(
    pid: u32,
    effect: impl Fn(ProcessIdentity) -> StopFuture<'static> + Send + Sync + 'static,
) -> Arc<ServerTermination> {
    Arc::new(ServerTermination {
        pid,
        identity: Some(ProcessIdentity::new(pid, 100)),
        attempt: Mutex::new(Arc::new(OnceCell::new())),
        effect: Arc::new(FakeStop(Box::new(effect))),
    })
}
async fn poll_pending(future: &mut Pin<Box<impl Future>>) {
    std::future::poll_fn(|context| {
        assert!(future.as_mut().poll(context).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
}

#[tokio::test]
async fn concurrent_failure_is_shared_and_later_retry_is_allowed() {
    let calls = Arc::new(AtomicUsize::new(0));
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let target = termination(10, {
        let calls = calls.clone();
        let entered = entered.clone();
        let release = release.clone();
        move |identity| {
            let first = calls.fetch_add(1, Ordering::SeqCst) == 0;
            let entered = entered.clone();
            let release = release.clone();
            Box::pin(async move {
                if first {
                    entered.notify_one();
                    release.notified().await;
                    return Err(TerminationError::CommandTimeout(identity.pid()));
                }
                Ok(())
            })
        }
    });
    let first = tokio::spawn(target.clone().terminate());
    entered.notified().await;
    let mut second = Box::pin(target.clone().terminate());
    poll_pending(&mut second).await;
    release.notify_one();
    let first_error = first.await.unwrap().unwrap_err();
    let second_error = second.await.unwrap_err();
    match (first_error, second_error) {
        (ProcessError::Termination(first), ProcessError::Termination(second)) => {
            assert!(Arc::ptr_eq(&first.source, &second.source));
            assert!(matches!(
                first.cause(),
                TerminationError::CommandTimeout(10)
            ));
        }
        other => panic!("Expected shared termination failures: {other:?}"),
    }
    target.clone().terminate().await.unwrap();
    target.terminate().await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn failed_kill_keeps_process_owned_and_retry_can_confirm_exit() {
    let manager = ServerProcessManager::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let mut server = server_fixture(10).await;
    server.termination = termination(10, {
        let manager = manager.clone();
        let calls = calls.clone();
        move |_| {
            let manager = manager.clone();
            let first = calls.fetch_add(1, Ordering::SeqCst) == 0;
            Box::pin(async move {
                if first {
                    return Err(TerminationError::Spawn {
                        pid: 10,
                        source: std::io::ErrorKind::PermissionDenied.into(),
                    });
                }
                manager.remove_if_pid_matches("server", 10).await;
                Ok(())
            })
        }
    });
    manager.insert_server(server).await;
    assert!(matches!(
        manager.kill_server_typed("server").await,
        Err(ProcessError::Termination(_))
    ));
    assert!(manager.owns("server", 10).await);
    manager.kill_server_typed("server").await.unwrap();
    assert!(!manager.owns("server", 10).await);
}

#[tokio::test]
async fn cancelled_waiter_does_not_abandon_kill_or_block_registry_or_touch_replacement() {
    let manager = ServerProcessManager::new();
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let completed = Arc::new(Notify::new());
    let mut server = server_fixture(10).await;
    server.termination = termination(10, {
        let entered = entered.clone();
        let release = release.clone();
        let completed = completed.clone();
        move |_| {
            let entered = entered.clone();
            let release = release.clone();
            let completed = completed.clone();
            Box::pin(async move {
                entered.notify_one();
                release.notified().await;
                completed.notify_one();
                Ok(())
            })
        }
    });
    manager.insert_server(server).await;
    let caller = tokio::spawn({
        let manager = manager.clone();
        async move { manager.kill_server_typed("server").await }
    });
    entered.notified().await;
    assert_eq!(manager.get_running_servers().await.len(), 1);
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    manager.insert_server(server_fixture(10).await).await;
    release.notify_one();
    completed.notified().await;
    assert_eq!(
        manager.get_server_status("server").await,
        crate::server::ServerStatus::Running
    );
    assert!(manager.owns("server", 10).await);
}

#[tokio::test(start_paused = true)]
async fn blocked_stdin_cannot_disable_force_deadline() {
    let manager = ServerProcessManager::new();
    let called = Arc::new(AtomicUsize::new(0));
    let mut server = server_fixture(10).await;
    let stdin = server.stdin.clone();
    let generation = server.generation.clone();
    let _held = stdin.lock().await;
    server.termination = termination(10, {
        let manager = manager.clone();
        let called = called.clone();
        move |_| {
            let manager = manager.clone();
            called.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                manager.remove_if_pid_matches("server", 10).await;
                Ok(())
            })
        }
    });
    manager.insert_server(server).await;
    let mut stop =
        Box::pin(manager.stop_owned_and_wait("server", 10, &generation, Duration::from_secs(1)));
    poll_pending(&mut stop).await;
    tokio::time::advance(Duration::from_millis(999)).await;
    assert_eq!(called.load(Ordering::SeqCst), 0);
    tokio::time::advance(Duration::from_millis(1)).await;
    stop.await.unwrap();
    assert_eq!(called.load(Ordering::SeqCst), 1);
}

#[tokio::test(start_paused = true)]
async fn failed_graceful_and_forced_shutdown_keep_both_errors() {
    let manager = ServerProcessManager::new();
    let mut server = server_fixture(10).await;
    let stdin = server.stdin.clone();
    let generation = server.generation.clone();
    let _held = stdin.lock().await;
    server.termination = termination(10, |_| {
        Box::pin(async { Err(TerminationError::CommandTimeout(10)) })
    });
    manager.insert_server(server).await;
    let error = manager
        .stop_owned_and_wait("server", 10, &generation, Duration::from_secs(1))
        .await
        .unwrap_err();
    assert!(
        matches!(error, ProcessError::ShutdownFailed { graceful, termination }
        if matches!(*graceful, ProcessError::ShutdownTimeout { .. })
        && matches!(*termination, ProcessError::Termination(_)))
    );
    assert!(manager.owns("server", 10).await);
}

#[tokio::test]
async fn shutdown_worker_panic_is_typed_and_does_not_remove_process() {
    let manager = ServerProcessManager::new();
    let mut server = server_fixture(10).await;
    server.termination = termination(10, |_| Box::pin(async { panic!("injected effect panic") }));
    manager.insert_server(server).await;
    assert!(matches!(
        manager.kill_server_typed("server").await,
        Err(ProcessError::ShutdownWorker(_))
    ));
    assert!(manager.owns("server", 10).await);
}

#[tokio::test]
async fn stale_cleanup_cannot_claim_same_pid_with_a_new_generation() {
    let manager = ServerProcessManager::new();
    let original = server_fixture(10).await;
    let generation = original.generation.clone();
    manager.insert_server(original).await;
    let mut replacement = server_fixture(10).await;
    let calls = Arc::new(AtomicUsize::new(0));
    replacement.termination = termination(10, {
        let calls = calls.clone();
        move |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { panic!("replacement must never be terminated by stale cleanup") })
        }
    });
    manager.insert_server(replacement).await;
    manager
        .kill_if_owned("server", 10, &generation)
        .await
        .unwrap();
    manager
        .stop_owned_and_wait("server", 10, &generation, Duration::ZERO)
        .await
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        manager.get_server_status("server").await,
        crate::server::ServerStatus::Running
    );
}

#[tokio::test(start_paused = true)]
async fn successful_native_termination_still_requires_monitor_release() {
    let manager = ServerProcessManager::new();
    let mut server = server_fixture(10).await;
    server.termination = termination(10, |_| Box::pin(async { Ok(()) }));
    manager.insert_server(server).await;
    let error = manager.kill_server_typed("server").await.unwrap_err();
    assert!(
        matches!(error, ProcessError::ShutdownTimeout { timeout, .. } if timeout == MONITOR_TIMEOUT)
    );
    assert!(manager.owns("server", 10).await);
}
