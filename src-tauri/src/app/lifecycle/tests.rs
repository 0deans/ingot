use super::*;
use std::sync::{atomic::AtomicUsize, Arc};

#[test]
fn every_non_stopped_server_blocks_quitting_and_only_sleeping_is_marked_as_sleeping() {
    use crate::server::ServerStatus;
    assert!(quit_blocker("id".into(), "name".into(), ServerStatus::Stopped).is_none());
    for status in [
        ServerStatus::Starting,
        ServerStatus::Running,
        ServerStatus::Stopping,
        ServerStatus::Sleeping,
    ] {
        let blocker = quit_blocker("id".into(), "name".into(), status.clone()).unwrap();
        assert_eq!(blocker.server_id, "id");
        assert_eq!(blocker.name, "name");
        assert_eq!(blocker.sleeping, status == ServerStatus::Sleeping);
    }
}

#[tokio::test]
async fn concurrent_adoption_runs_once_and_another_application_can_adopt_independently() {
    let lifecycle = LifecycleState::default();
    let runs = AtomicUsize::new(0);
    let adopt = || async {
        runs.fetch_add(1, Ordering::SeqCst);
        tokio::task::yield_now().await;
    };
    tokio::join!(
        lifecycle.adopt_games_once(adopt),
        lifecycle.adopt_games_once(adopt)
    );
    lifecycle.adopt_games_once(adopt).await;
    assert_eq!(runs.load(Ordering::SeqCst), 1);
    LifecycleState::default().adopt_games_once(adopt).await;
    assert_eq!(runs.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn cancelled_adoption_can_retry_then_stays_completed() {
    let lifecycle = Arc::new(LifecycleState::default());
    let initial = lifecycle.clone();
    let (started, ready) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        initial
            .adopt_games_once(|| async {
                started.send(()).unwrap();
                std::future::pending::<()>().await;
            })
            .await;
    });
    ready.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let runs = AtomicUsize::new(0);
    let adopt = || async {
        runs.fetch_add(1, Ordering::SeqCst);
    };
    lifecycle.adopt_games_once(adopt).await;
    lifecycle.adopt_games_once(adopt).await;
    assert_eq!(runs.load(Ordering::SeqCst), 1);
}

fn leftover_entry() -> crate::running::Entry {
    crate::running::Entry {
        id: "server".into(),
        pid: 10,
        started_at: 20,
        proc_start: 30,
    }
}

#[tokio::test]
async fn failed_leftover_termination_preserves_the_discovery_record_and_cause() {
    let acknowledged = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let saved = acknowledged.clone();
    let error = stop_leftover_owned(
        leftover_entry(),
        |_| async {
            Err(crate::process_tree::TerminationError::Spawn {
                pid: 10,
                source: std::io::ErrorKind::PermissionDenied.into(),
            })
        },
        move |_| {
            saved.store(true, Ordering::SeqCst);
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(error,
        LifecycleError::Termination(crate::process_tree::TerminationError::Spawn { pid: 10, ref source })
            if source.kind() == std::io::ErrorKind::PermissionDenied));
    assert!(!acknowledged.load(Ordering::SeqCst));
    assert!(std::error::Error::source(&error).is_some());
}

#[tokio::test]
async fn cancelled_leftover_caller_does_not_abandon_termination_acknowledgement() {
    let original = leftover_entry();
    let expected = original.clone();
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let (acknowledged, done) = tokio::sync::oneshot::channel();
    let caller = tokio::spawn(stop_leftover_owned(
        original,
        move |identity| async move {
            assert_eq!(identity.pid(), 10);
            assert_eq!(identity.start(), 30);
            entered.send(()).unwrap();
            released.await.unwrap();
            Ok(())
        },
        move |entry| {
            acknowledged.send(entry).unwrap();
        },
    ));
    ready.await.unwrap();
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    release.send(()).unwrap();
    assert_eq!(done.await.unwrap(), expected);
}

#[tokio::test]
async fn shutdown_batch_waits_for_every_server_and_retains_all_failures() {
    use crate::server::process::ProcessError;
    let finished = Arc::new(AtomicUsize::new(0));
    let completed = finished.clone();
    let jobs = vec![
        ShutdownJob {
            server_id: "unwritable".into(),
            work: tokio::spawn(async {
                Err(ProcessError::StdinWrite(
                    std::io::ErrorKind::BrokenPipe.into(),
                ))
            }),
        },
        ShutdownJob {
            server_id: "stopped".into(),
            work: tokio::spawn(async move {
                completed.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }),
        },
        ShutdownJob {
            server_id: "unconfirmed".into(),
            work: tokio::spawn(async {
                Err(ProcessError::ShutdownTimeout {
                    server_id: "unconfirmed".into(),
                    timeout: std::time::Duration::from_secs(3),
                })
            }),
        },
    ];
    let error = collect_shutdowns(jobs).await.unwrap_err();
    let LifecycleError::Servers(failures) = error else {
        panic!("Expected shutdown failures");
    };
    assert_eq!(finished.load(Ordering::SeqCst), 1);
    assert_eq!(failures.0.len(), 2);
    assert_eq!(failures.0[0].server_id, "unwritable");
    assert!(
        matches!(failures.0[0].source, ProcessError::StdinWrite(ref source) if source.kind() == std::io::ErrorKind::BrokenPipe)
    );
    assert_eq!(failures.0[1].server_id, "unconfirmed");
    assert!(matches!(
        failures.0[1].source,
        ProcessError::ShutdownTimeout { .. }
    ));
}

#[tokio::test]
async fn shutdown_batch_reports_abandoned_worker_instead_of_success() {
    let work = tokio::spawn(std::future::pending::<
        Result<(), server::process::ProcessError>,
    >());
    work.abort();
    let result = collect_shutdowns(vec![ShutdownJob {
        server_id: "worker".into(),
        work,
    }])
    .await;
    let Err(LifecycleError::Servers(failures)) = result else {
        panic!("Expected worker failure");
    };
    assert_eq!(failures.0[0].server_id, "worker");
    assert!(
        matches!(failures.0[0].source, server::process::ProcessError::ShutdownWorker(ref source) if source.is_cancelled())
    );
}
