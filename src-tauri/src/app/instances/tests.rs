use super::run_maintenance;
use ingot_minecraft::launcher::ProcessManager;
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
        Err(ingot_minecraft::instance::InstanceError::EmptyName)
    })
    .await;
    assert!(matches!(
        result,
        Err(ingot_minecraft::instance::InstanceError::EmptyName)
    ));
    assert!(manager.reserve_maintenance("instance").is_ok());
}

#[test]
fn resolve_shortcut_directory_handles_all_locations() {
    let app = tauri::test::mock_app();
    let handle = app.handle();

    #[cfg(not(target_os = "android"))]
    {
        let desktop = super::resolve_shortcut_directory(
            handle,
            ingot_minecraft::instance::ShortcutLocation::Desktop,
        );
        assert!(
            desktop.is_ok()
                || matches!(
                    desktop,
                    Err(ingot_minecraft::instance::ShortcutError::DirectoryNotFound(
                        _
                    ))
                )
        );
    }

    #[cfg(target_os = "android")]
    {
        let desktop = super::resolve_shortcut_directory(
            handle,
            ingot_minecraft::instance::ShortcutLocation::Desktop,
        );
        assert!(matches!(
            desktop,
            Err(ingot_minecraft::instance::ShortcutError::UnsupportedPlatform(_))
        ));
    }

    let steam = super::resolve_shortcut_directory(
        handle,
        ingot_minecraft::instance::ShortcutLocation::Steam,
    );
    assert!(matches!(
        steam,
        Err(ingot_minecraft::instance::ShortcutError::DirectoryNotFound(
            _
        ))
    ));
}

struct RemoveFileOnDrop(std::path::PathBuf);

impl Drop for RemoveFileOnDrop {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

struct RemoveDirOnDrop(std::path::PathBuf);

impl Drop for RemoveDirOnDrop {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn build_test_app(dir: &std::path::Path) -> tauri::App<tauri::test::MockRuntime> {
    let state = crate::app::AppState::new(crate::app::AppPaths::new(dir.to_owned())).unwrap();
    tauri::test::mock_builder()
        .manage(state)
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap()
}

fn register_instance(app: &tauri::AppHandle<tauri::test::MockRuntime>, id: &str) {
    let mut instance = ingot_minecraft::instance::prepare_instance(
        "Test Instance".to_owned(),
        "1.21.4".to_owned(),
        ingot_minecraft::instance::ModLoaderType::Vanilla,
        None,
    )
    .unwrap();
    instance.id = id.to_owned();
    crate::app::state(app)
        .instances
        .update(|records| {
            records.push(instance);
            Ok(())
        })
        .unwrap();
}

#[test]
fn resolve_registered_instance_rejects_missing_id_without_creating_orphan_dir() {
    use tauri::Manager;

    let dir = tempfile::tempdir().unwrap();
    let app = build_test_app(dir.path());
    let missing_id = format!("missing-inst-{}", uuid::Uuid::new_v4());

    let result = super::resolve_registered_instance(app.handle(), &missing_id);
    assert!(matches!(
        result,
        Err(ingot_minecraft::instance::InstanceError::NotFound(ref id)) if id == &missing_id
    ));

    let orphan_dir = app
        .path()
        .app_data_dir()
        .unwrap()
        .join("instances")
        .join(&missing_id);
    assert!(!orphan_dir.exists());
}

#[test]
fn resolve_registered_instance_preserves_corrupt_and_recovery_blocked_metadata() {
    let corrupt_dir = tempfile::tempdir().unwrap();
    let instances_file = corrupt_dir.path().join("instances.json");
    let corrupt = b"{corrupt-instances";
    std::fs::write(&instances_file, corrupt).unwrap();
    let corrupt_app = build_test_app(corrupt_dir.path());

    let corrupt_err = super::resolve_registered_instance(corrupt_app.handle(), "any-id")
        .err()
        .expect("corrupt instances.json must fail");
    assert!(matches!(
        corrupt_err,
        ingot_minecraft::instance::InstanceError::Metadata(
            ingot_core::metadata::MetadataError::Decode { ref path, .. }
        ) if path == &instances_file
    ));
    assert_eq!(std::fs::read(&instances_file).unwrap(), corrupt);

    let recovery_dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(recovery_dir.path().join("instances.deletion.json")).unwrap();
    let recovery_app = build_test_app(recovery_dir.path());

    let recovery_err = super::resolve_registered_instance(recovery_app.handle(), "any-id")
        .err()
        .expect("recovery-blocked store must fail");
    assert!(matches!(
        recovery_err,
        ingot_minecraft::instance::InstanceError::Metadata(
            ingot_core::metadata::MetadataError::RecoveryBlocked(_)
        )
    ));
}

#[test]
fn resolve_registered_instance_preserves_io_invalid_id_and_resolves_valid_instance() {
    use tauri::Manager;

    let dir = tempfile::tempdir().unwrap();
    let app = build_test_app(dir.path());
    let valid_id = format!("valid-inst-{}", uuid::Uuid::new_v4());
    let io_id = format!("io-blocker-{}", uuid::Uuid::new_v4());
    let invalid_id = "../invalid-inst";

    register_instance(app.handle(), &valid_id);
    register_instance(app.handle(), &io_id);
    register_instance(app.handle(), invalid_id);

    let instances_root = app.path().app_data_dir().unwrap().join("instances");
    std::fs::create_dir_all(&instances_root).unwrap();
    let blocker = instances_root.join(&io_id);
    std::fs::write(&blocker, b"blocking-file").unwrap();
    let _blocker_cleanup = RemoveFileOnDrop(blocker);
    let _valid_cleanup = RemoveDirOnDrop(instances_root.join(&valid_id));

    let valid = super::resolve_registered_instance(app.handle(), &valid_id).unwrap();
    assert_eq!(valid.config.id, valid_id);
    assert_eq!(valid.dir, instances_root.join(&valid_id));
    assert!(valid.dir.is_dir());

    let io_err = super::resolve_registered_instance(app.handle(), &io_id)
        .err()
        .expect("file blocker should return Io");
    assert!(matches!(
        io_err,
        ingot_minecraft::instance::InstanceError::Io(_)
    ));

    let invalid_err = super::resolve_registered_instance(app.handle(), invalid_id)
        .err()
        .expect("invalid ID should return InvalidId");
    assert!(matches!(
        invalid_err,
        ingot_minecraft::instance::InstanceError::InvalidId
    ));
}

#[tokio::test]
async fn read_worlds_missing_instance_returns_not_found_without_orphan_dir() {
    use tauri::Manager;

    let dir = tempfile::tempdir().unwrap();
    let app = build_test_app(dir.path());
    let missing_id = format!("missing-worlds-{}", uuid::Uuid::new_v4());

    let err = super::read_worlds(app.handle(), missing_id.clone())
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        ingot_minecraft::instance::InstanceError::NotFound(ref id) if id == &missing_id
    ));

    let orphan_dir = app
        .path()
        .app_data_dir()
        .unwrap()
        .join("instances")
        .join(&missing_id);
    assert!(!orphan_dir.exists());
}
