use super::*;

#[tokio::test]
async fn poisoned_registry_is_an_explicit_failure() {
    let manager = ProcessManager::new();
    let entries = manager.running.clone();
    assert!(std::thread::spawn(move || {
        let _guard = entries.lock().unwrap();
        panic!("poison fixture");
    })
    .join()
    .is_err());
    assert!(matches!(
        manager.reserve_starting("instance").await,
        Err(LaunchError::RegistryPoisoned)
    ));
    assert!(matches!(
        manager.reserve_maintenance("instance"),
        Err(LaunchError::RegistryPoisoned)
    ));
    assert!(matches!(
        manager.kill_instance("instance").await,
        Err(LaunchError::RegistryPoisoned)
    ));
}

#[tokio::test]
async fn cancelled_reservations_are_typed_and_cannot_commit_over_a_new_launch() {
    let manager = ProcessManager::new();
    let old = manager.reserve_starting("instance").await.unwrap();
    manager.kill_instance("instance").await.unwrap();
    let current = manager.reserve_starting("instance").await.unwrap();
    assert!(matches!(
        old.check_active(),
        Err(LaunchError::ReservationCancelled)
    ));
    assert!(matches!(
        old.commit(1, 1),
        Err(LaunchError::ReservationCancelled)
    ));
    current.check_active().unwrap();
}

#[tokio::test]
async fn queued_termination_rejects_replaced_process_identity() {
    let manager = ProcessManager::new();
    manager.register("instance", 10, 20).await;
    let old = ActiveChild {
        pid: 10,
        started_at: 20,
        proc_start: None,
    };
    manager.register("instance", 11, 30).await;
    assert!(matches!(
        manager.check_running_identity("instance", old),
        Err(LaunchError::ReservationCancelled)
    ));
    assert_eq!(manager.get_running_instances().await[0].pid, 11);
}

#[tokio::test]
async fn termination_failure_retains_running_identity_for_retry() {
    let manager = ProcessManager::new();
    manager.register("instance", 10, 20).await;
    let error = manager
        .kill_instance_with("instance", |_| async {
            Err(super::super::termination::TerminationError::Command(
                std::io::Error::from(std::io::ErrorKind::PermissionDenied),
            ))
        })
        .await
        .unwrap_err();
    assert!(matches!(error, LaunchError::Termination(_)));
    assert_eq!(manager.get_running_instances().await[0].pid, 10);
}

#[tokio::test]
async fn termination_worker_keeps_ownership_after_caller_cancellation() {
    let manager = ProcessManager::new();
    manager.register("instance", 10, 20).await;
    let (entered, entered_rx) = tokio::sync::oneshot::channel();
    let (release, release_rx) = tokio::sync::oneshot::channel();
    let (finished, finished_rx) = tokio::sync::oneshot::channel();
    let owner = manager.clone();
    let caller = tokio::spawn(async move {
        owner
            .kill_instance_with("instance", |_| async move {
                entered.send(()).unwrap();
                release_rx.await.unwrap();
                finished.send(()).unwrap();
                Ok(())
            })
            .await
    });
    entered_rx.await.unwrap();
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    assert!(matches!(
        manager.reserve_starting("instance").await,
        Err(LaunchError::AlreadyRunning)
    ));
    release.send(()).unwrap();
    finished_rx.await.unwrap();
    assert_eq!(manager.get_running_instances().await[0].pid, 10);
}
