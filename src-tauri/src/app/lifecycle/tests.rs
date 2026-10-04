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
