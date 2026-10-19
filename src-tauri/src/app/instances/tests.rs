use super::run_maintenance;
use crate::minecraft::launcher::ProcessManager;
use std::sync::Arc;
use tokio::sync::Notify;

#[derive(Debug)]
struct WorkerResult(Arc<Notify>);

impl Drop for WorkerResult {
    fn drop(&mut self) {
        self.0.notify_one();
    }
}

#[tokio::test]
async fn cancelled_caller_keeps_maintenance_until_blocking_worker_exits() {
    let manager = ProcessManager::new();
    let ownership = manager.reserve_maintenance("instance").unwrap();
    let entered = Arc::new(Notify::new());
    let finished = Arc::new(Notify::new());
    let (release, blocked) = std::sync::mpsc::channel();
    let worker_entered = Arc::clone(&entered);
    let worker_finished = Arc::clone(&finished);
    let caller = tokio::spawn(run_maintenance(ownership, move || {
        worker_entered.notify_one();
        blocked.recv().unwrap();
        Ok(WorkerResult(worker_finished))
    }));
    entered.notified().await;
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    assert!(manager.reserve_maintenance("instance").is_err());
    assert!(manager.reserve_starting("instance").await.is_err());

    release.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), finished.notified())
        .await
        .unwrap();
    assert!(manager.reserve_maintenance("instance").is_ok());
}

#[tokio::test]
async fn failed_blocking_operation_releases_maintenance_and_preserves_error() {
    let manager = ProcessManager::new();
    let ownership = manager.reserve_maintenance("instance").unwrap();
    let result = run_maintenance::<()>(ownership, || {
        Err(crate::minecraft::instance::InstanceError::EmptyName)
    })
    .await;
    assert!(matches!(
        result,
        Err(crate::minecraft::instance::InstanceError::EmptyName)
    ));
    assert!(manager.reserve_maintenance("instance").is_ok());
}
