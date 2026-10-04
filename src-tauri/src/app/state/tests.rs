use super::*;

#[tokio::test]
async fn process_state_is_shared_by_clones_but_isolated_between_applications() {
    let first_dir = tempfile::tempdir().unwrap();
    let second_dir = tempfile::tempdir().unwrap();
    let first = AppState::new(AppPaths::new(first_dir.path().to_owned())).unwrap();
    let second = AppState::new(AppPaths::new(second_dir.path().to_owned())).unwrap();
    let servers = first.servers.clone();
    servers
        .set_server_status("same-id", crate::server::ServerStatus::Sleeping)
        .await;
    assert_eq!(
        first.servers.get_server_status("same-id").await,
        crate::server::ServerStatus::Sleeping
    );
    assert_eq!(
        second.servers.get_server_status("same-id").await,
        crate::server::ServerStatus::Stopped
    );
    first.lifecycle.mark_quitting();
    assert!(first.lifecycle.is_quitting());
    assert!(!second.lifecycle.is_quitting());
    assert!(!first_dir.path().join("settings.json").exists());
    assert!(!second_dir.path().join("settings.json").exists());
}
