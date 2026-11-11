use crate::system::load_settings;
use ingot_minecraft::instance::{InstanceConfig, InstanceError};
use ingot_minecraft::sync::{
    execute::{ensure_shared_directories, file_exists, inspect_shared_status},
    models::{
        ConflictResolution, SharedSyncStatus, SyncCategory, SyncConflictInfo, SyncError, SyncReport,
    },
    policy::{evaluate_conflict, resolve_effective_sync, ConflictCheckInputs, EffectiveSync},
    service::{
        execute_sync_after_exit, execute_sync_before_launch, export_instance_category_files,
        import_shared_files_to_instance,
    },
};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tauri::{Manager, Runtime};
pub fn get_shared_dir<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, SyncError> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|source| SyncError::AppDataDir(std::io::Error::other(source)))?;
    let shared = data_dir.join("shared");
    ensure_shared_directories(&shared).map_err(SyncError::SharedDirCreation)?;
    Ok(shared)
}

pub fn get_effective_sync<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance: &InstanceConfig,
) -> Result<EffectiveSync, SyncError> {
    let global = load_settings(app).map_err(SyncError::Settings)?.sync;
    Ok(resolve_effective_sync(instance, &global))
}

pub fn mark_instance_synced<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<(), SyncError> {
    let now = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| SyncError::Io(std::io::Error::other(error)))?
        .as_secs();
    crate::app::instances::patch_instance(app, instance_id, |instance| {
        instance.last_synced_at = Some(now);
    })
    .map_err(SyncError::Instances)?;
    Ok(())
}

pub fn sync_before_launch<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance: &InstanceConfig,
    instance_dir: &Path,
) -> Result<(), SyncError> {
    let shared = get_shared_dir(app)?;
    let eff = get_effective_sync(app, instance)?;
    execute_sync_before_launch(&shared, instance_dir, &eff)?;
    mark_instance_synced(app, &instance.id)?;
    Ok(())
}

pub fn sync_after_exit<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance: &InstanceConfig,
    instance_dir: &Path,
) -> Result<(), SyncError> {
    let shared = get_shared_dir(app)?;
    let eff = get_effective_sync(app, instance)?;
    execute_sync_after_exit(&shared, instance_dir, &eff)?;
    mark_instance_synced(app, &instance.id)?;
    Ok(())
}

fn resolve_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<crate::app::instances::ResolvedInstance, SyncError> {
    crate::app::instances::resolve_registered_instance(app, instance_id).map_err(
        |error| match error {
            InstanceError::NotFound(id) => SyncError::InstanceNotFound(id),
            other => SyncError::Instances(other),
        },
    )
}

pub fn export_instance_category_to_shared<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    category: &str,
) -> Result<SyncReport, SyncError> {
    let cat = SyncCategory::parse(category)
        .ok_or_else(|| SyncError::InvalidCategory(category.to_string()))?;
    let resolved = resolve_instance(app, instance_id)?;
    let shared = get_shared_dir(app)?;
    let report = export_instance_category_files(&shared, &resolved.dir, cat)?;
    mark_instance_synced(app, instance_id)?;
    Ok(report)
}

pub fn export_instance_to_shared<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<SyncReport, SyncError> {
    export_instance_category_to_shared(app, instance_id, "all")
}

pub fn import_shared_to_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<SyncReport, SyncError> {
    let resolved = resolve_instance(app, instance_id)?;
    let shared = get_shared_dir(app)?;
    let report = import_shared_files_to_instance(&shared, &resolved.dir)?;
    mark_instance_synced(app, instance_id)?;
    Ok(report)
}

pub fn get_shared_sync_status<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<SharedSyncStatus, SyncError> {
    let shared = get_shared_dir(app)?;
    Ok(inspect_shared_status(&shared)?)
}

pub fn check_sync_conflict<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<Option<SyncConflictInfo>, SyncError> {
    let resolved = resolve_instance(app, instance_id)?;
    let instance = &resolved.config;
    let instance_dir = &resolved.dir;

    let eff = get_effective_sync(app, instance)?;
    let shared = get_shared_dir(app)?;

    let has_instance_options = file_exists(&instance_dir.join("options.txt"))?;
    let has_instance_servers = file_exists(&instance_dir.join("servers.dat"))?;
    let has_shared_options = file_exists(&shared.join("options.txt"))?;
    let has_shared_servers = file_exists(&shared.join("servers.dat"))?;

    let conflict_inputs = ConflictCheckInputs {
        has_last_synced: instance.last_synced_at.is_some(),
        effective: eff,
        has_instance_options,
        has_instance_servers,
        has_shared_options,
        has_shared_servers,
    };

    if evaluate_conflict(&conflict_inputs) {
        let shared_modified = inspect_shared_status(&shared)?.last_synced_timestamp;
        Ok(Some(SyncConflictInfo {
            instance_id: instance.id.clone(),
            instance_name: instance.name.clone(),
            has_instance_options,
            has_instance_servers,
            has_shared_options,
            has_shared_servers,
            shared_modified,
        }))
    } else {
        Ok(None)
    }
}

pub fn resolve_sync_conflict<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    resolution: &str,
) -> Result<SyncReport, SyncError> {
    let mode = ConflictResolution::parse(resolution)
        .ok_or_else(|| SyncError::InvalidResolution(resolution.to_string()))?;

    match mode {
        ConflictResolution::UseShared => {
            let report = import_shared_to_instance(app, instance_id)?;
            Ok(report)
        }
        ConflictResolution::UseInstance => {
            let report = export_instance_to_shared(app, instance_id)?;
            Ok(report)
        }
        ConflictResolution::DisableSync => {
            crate::app::instances::patch_instance(app, instance_id, |instance| {
                instance.sync_options = Some(false);
                instance.sync_servers = Some(false);
                instance.sync_resource_packs = Some(false);
                instance.sync_command_history = Some(false);
                instance.sync_creative_hotbars = Some(false);
            })
            .map_err(SyncError::Instances)?;
            Ok(SyncReport {
                options_synced: false,
                servers_synced: false,
                resource_packs_count: 0,
                command_history_synced: false,
                creative_hotbars_synced: false,
                message: "Sync disabled for this instance.".to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppPaths, AppState};
    use ingot_minecraft::instance::{prepare_instance, ModLoaderType};
    use ingot_settings::SettingsError;
    use std::error::Error;

    fn instance() -> InstanceConfig {
        prepare_instance(
            "Sync test".to_owned(),
            "1.21".to_owned(),
            ModLoaderType::Vanilla,
            None,
        )
        .unwrap()
    }

    #[test]
    fn missing_settings_state_does_not_become_default_sync_preferences() {
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();

        let error = get_effective_sync(app.handle(), &instance()).unwrap_err();

        assert!(matches!(
            error,
            SyncError::Settings(SettingsError::StateUnavailable)
        ));
        assert!(error
            .source()
            .unwrap()
            .downcast_ref::<SettingsError>()
            .is_some());
    }

    #[test]
    fn corrupt_settings_keep_path_and_decode_cause_without_replacing_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        let original = b"{ invalid settings";
        std::fs::write(&path, original).unwrap();
        let state = AppState::new(AppPaths::new(directory.path().to_owned())).unwrap();
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();

        let error = get_effective_sync(app.handle(), &instance()).unwrap_err();

        let SyncError::Settings(SettingsError::Decode {
            path: failed_path,
            source,
        }) = &error
        else {
            panic!("expected sourced settings decode failure: {error:?}");
        };
        assert_eq!(failed_path, &path);
        assert!(source.is_syntax());
        assert!(error
            .source()
            .unwrap()
            .source()
            .unwrap()
            .downcast_ref::<serde_json::Error>()
            .is_some());
        assert_eq!(std::fs::read(path).unwrap(), original);
    }

    #[test]
    fn native_directory_failure_keeps_underlying_native_cause() {
        let native = tauri::Error::Io(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
        let error = SyncError::AppDataDir(std::io::Error::other(native));

        let source = error
            .source()
            .unwrap()
            .downcast_ref::<std::io::Error>()
            .unwrap();
        let native = source
            .get_ref()
            .unwrap()
            .downcast_ref::<tauri::Error>()
            .unwrap();
        assert!(matches!(
            native,
            tauri::Error::Io(source) if source.kind() == std::io::ErrorKind::PermissionDenied
        ));
    }

    struct RemoveFileOnDrop(PathBuf);

    impl Drop for RemoveFileOnDrop {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn build_test_app(dir: &std::path::Path) -> tauri::App<tauri::test::MockRuntime> {
        let state = AppState::new(AppPaths::new(dir.to_owned())).unwrap();
        tauri::test::mock_builder()
            .manage(state)
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap()
    }

    fn register_instance(app: &tauri::AppHandle<tauri::test::MockRuntime>, id: &str) {
        let mut record = instance();
        record.id = id.to_owned();
        crate::app::state(app)
            .instances
            .update(|records| {
                records.push(record);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn missing_instance_returns_not_found_without_creating_orphan_directory() {
        let dir = tempfile::tempdir().unwrap();
        let app = build_test_app(dir.path());
        let missing_id = format!("sync-missing-{}", uuid::Uuid::new_v4());

        for error in [
            export_instance_category_to_shared(app.handle(), &missing_id, "all").unwrap_err(),
            import_shared_to_instance(app.handle(), &missing_id).unwrap_err(),
            check_sync_conflict(app.handle(), &missing_id).unwrap_err(),
        ] {
            assert!(matches!(
                &error,
                SyncError::InstanceNotFound(id) if id == &missing_id
            ));
            assert!(matches!(
                crate::ipc::ApiError::from(error),
                crate::ipc::ApiError::NotFound { .. }
            ));
        }

        let orphan_dir = app
            .path()
            .app_data_dir()
            .unwrap()
            .join("instances")
            .join(&missing_id);
        assert!(!orphan_dir.exists());
    }

    #[test]
    fn corrupt_and_recovery_blocked_metadata_are_preserved() {
        use ingot_core::metadata::MetadataError;

        let corrupt_dir = tempfile::tempdir().unwrap();
        let instances_file = corrupt_dir.path().join("instances.json");
        let corrupt = b"{corrupt-instances";
        std::fs::write(&instances_file, corrupt).unwrap();
        let corrupt_app = build_test_app(corrupt_dir.path());

        for error in [
            export_instance_category_to_shared(corrupt_app.handle(), "any-instance", "all")
                .unwrap_err(),
            import_shared_to_instance(corrupt_app.handle(), "any-instance").unwrap_err(),
            check_sync_conflict(corrupt_app.handle(), "any-instance").unwrap_err(),
        ] {
            assert!(matches!(
                &error,
                SyncError::Instances(InstanceError::Metadata(MetadataError::Decode { path, .. }))
                    if path == &instances_file
            ));
            assert!(matches!(
                crate::ipc::ApiError::from(error),
                crate::ipc::ApiError::StorageFailure { .. }
            ));
        }
        assert_eq!(std::fs::read(&instances_file).unwrap(), corrupt);

        let recovery_dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(recovery_dir.path().join("instances.deletion.json")).unwrap();
        let recovery_app = build_test_app(recovery_dir.path());

        for error in [
            export_instance_category_to_shared(recovery_app.handle(), "any-instance", "all")
                .unwrap_err(),
            import_shared_to_instance(recovery_app.handle(), "any-instance").unwrap_err(),
            check_sync_conflict(recovery_app.handle(), "any-instance").unwrap_err(),
        ] {
            assert!(matches!(
                &error,
                SyncError::Instances(InstanceError::Metadata(MetadataError::RecoveryBlocked(_)))
            ));
            assert!(matches!(
                crate::ipc::ApiError::from(error),
                crate::ipc::ApiError::IncompleteRecovery { .. }
            ));
        }
    }

    #[test]
    fn registered_io_and_invalid_id_errors_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let app = build_test_app(dir.path());
        let io_id = format!("sync-io-blocker-{}", uuid::Uuid::new_v4());
        let invalid_id = "../sync-invalid";
        register_instance(app.handle(), &io_id);
        register_instance(app.handle(), invalid_id);

        let instances_root = app.path().app_data_dir().unwrap().join("instances");
        std::fs::create_dir_all(&instances_root).unwrap();
        let blocker = instances_root.join(&io_id);
        std::fs::write(&blocker, b"file-blocking-dir").unwrap();
        let _cleanup = RemoveFileOnDrop(blocker);

        let io_err = check_sync_conflict(app.handle(), &io_id).unwrap_err();
        assert!(matches!(
            &io_err,
            SyncError::Instances(InstanceError::Io(_))
        ));
        assert!(matches!(
            crate::ipc::ApiError::from(io_err),
            crate::ipc::ApiError::StorageFailure { .. }
        ));

        let invalid_err = check_sync_conflict(app.handle(), invalid_id).unwrap_err();
        assert!(matches!(
            &invalid_err,
            SyncError::Instances(InstanceError::InvalidId)
        ));
        assert!(matches!(
            crate::ipc::ApiError::from(invalid_err),
            crate::ipc::ApiError::InvalidInput { .. }
        ));
    }
}
