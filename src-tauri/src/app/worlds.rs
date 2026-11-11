use ingot_minecraft::instance::{InstanceConfig, InstanceError};
use ingot_minecraft::worlds::models::{
    validate_world_folder_name, WorldBackupInfo, WorldDimension, WorldResetResult, WorldSummary,
};
use ingot_minecraft::worlds::{dimensions, discovery, transfer, WorldError};
use std::path::PathBuf;
use tauri::{Manager, Runtime};

fn resolve_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<crate::app::instances::ResolvedInstance, WorldError> {
    crate::app::instances::resolve_registered_instance(app, instance_id).map_err(
        |error| match error {
            InstanceError::NotFound(id) => WorldError::InstanceNotFound(id),
            other => WorldError::InstanceDir(other),
        },
    )
}

fn instance_dir<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<PathBuf, WorldError> {
    resolve_instance(app, instance_id).map(|resolved| resolved.dir)
}

fn load_instances<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<InstanceConfig>, WorldError> {
    crate::app::instances::load_instances(app).map_err(WorldError::LoadInstances)
}

pub(crate) async fn get_all_worlds<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<WorldSummary>, WorldError> {
    let app = app.clone();
    tokio::task::spawn_blocking(move || {
        let instances = load_instances(&app)?;
        let mut all_worlds = Vec::new();
        for instance in instances {
            if let Ok(dir) = crate::app::instances::ensure_instance_dir(&app, &instance.id) {
                if let Ok(worlds) =
                    discovery::get_instance_worlds(&dir, &instance.id, &instance.name)
                {
                    all_worlds.extend(worlds);
                }
            }
        }
        all_worlds.sort_by_key(|w| std::cmp::Reverse(w.last_played));
        Ok(all_worlds)
    })
    .await?
}

pub(crate) async fn get_instance_worlds_full<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: String,
) -> Result<Vec<WorldSummary>, WorldError> {
    let app = app.clone();
    tokio::task::spawn_blocking(move || {
        let crate::app::instances::ResolvedInstance {
            config: instance,
            dir,
        } = resolve_instance(&app, &instance_id)?;
        discovery::get_instance_worlds(&dir, &instance_id, &instance.name)
    })
    .await?
}

pub(crate) async fn reset_world_dimension<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: String,
    folder_name: String,
    dimension: WorldDimension,
) -> Result<WorldResetResult, WorldError> {
    let guard = crate::app::state(app)
        .games
        .reserve_maintenance(&instance_id)
        .map_err(|_| WorldError::InstanceRunning)?;

    validate_world_folder_name(&folder_name)?;
    let app = app.clone();

    tokio::task::spawn_blocking(move || {
        let _guard = guard;
        let dir = instance_dir(&app, &instance_id)?;
        let world_dir = dir.join("saves").join(&folder_name);
        dimensions::reset_dimension(&world_dir, dimension)
    })
    .await?
}

pub(crate) async fn export_world<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: String,
    folder_name: String,
    dest: Option<String>,
) -> Result<String, WorldError> {
    let guard = crate::app::state(app)
        .games
        .reserve_maintenance(&instance_id)
        .map_err(|_| WorldError::InstanceRunning)?;

    validate_world_folder_name(&folder_name)?;
    let app = app.clone();

    tokio::task::spawn_blocking(move || {
        let _guard = guard;
        let dir = instance_dir(&app, &instance_id)?;
        let world_dir = dir.join("saves").join(&folder_name);

        let dest_path = match dest {
            Some(p) => PathBuf::from(p),
            None => {
                let cache_dir = app
                    .path()
                    .app_cache_dir()
                    .map_err(|e| WorldError::Archive(e.to_string()))?
                    .join("exports");
                std::fs::create_dir_all(&cache_dir)?;
                cache_dir.join(format!("{folder_name}.zip"))
            }
        };

        transfer::export_world(&world_dir, &dest_path)?;
        Ok(dest_path.to_string_lossy().to_string())
    })
    .await?
}

pub(crate) async fn backup_world<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: String,
    folder_name: String,
) -> Result<WorldBackupInfo, WorldError> {
    let guard = crate::app::state(app)
        .games
        .reserve_maintenance(&instance_id)
        .map_err(|_| WorldError::InstanceRunning)?;

    validate_world_folder_name(&folder_name)?;
    let app = app.clone();

    tokio::task::spawn_blocking(move || {
        let _guard = guard;
        let dir = instance_dir(&app, &instance_id)?;
        transfer::backup_world(&dir, &folder_name)
    })
    .await?
}

pub(crate) async fn list_world_backups<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: String,
    folder_name: Option<String>,
) -> Result<Vec<WorldBackupInfo>, WorldError> {
    let app = app.clone();
    tokio::task::spawn_blocking(move || {
        let dir = instance_dir(&app, &instance_id)?;
        transfer::list_world_backups(&dir, folder_name.as_deref())
    })
    .await?
}

pub(crate) async fn delete_world_backup<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: String,
    file_name: String,
) -> Result<(), WorldError> {
    let app = app.clone();
    tokio::task::spawn_blocking(move || {
        let dir = instance_dir(&app, &instance_id)?;
        transfer::delete_world_backup(&dir, &file_name)
    })
    .await?
}

pub(crate) async fn import_world<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: String,
    zip_path: String,
) -> Result<WorldSummary, WorldError> {
    let guard = crate::app::state(app)
        .games
        .reserve_maintenance(&instance_id)
        .map_err(|_| WorldError::InstanceRunning)?;

    let app = app.clone();
    tokio::task::spawn_blocking(move || {
        let _guard = guard;
        let crate::app::instances::ResolvedInstance {
            config: instance,
            dir,
        } = resolve_instance(&app, &instance_id)?;
        let saves_dir = dir.join("saves");
        let source_path = PathBuf::from(zip_path);

        let final_folder_name = transfer::import_world(&saves_dir, &source_path)?;
        let worlds = discovery::get_instance_worlds(&dir, &instance_id, &instance.name)?;
        worlds
            .into_iter()
            .find(|w| w.folder_name == final_folder_name)
            .ok_or_else(|| WorldError::NotFound(final_folder_name))
    })
    .await?
}

pub(crate) fn resolve_world_dir<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    folder_name: &str,
) -> Result<PathBuf, WorldError> {
    validate_world_folder_name(folder_name)?;
    let dir = instance_dir(app, instance_id)?;
    let world_dir = dir.join("saves").join(folder_name);
    if !world_dir.is_dir() {
        return Err(WorldError::NotFound(folder_name.to_string()));
    }
    Ok(world_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppPaths, AppState};
    use crate::ipc::ApiError;
    use ingot_core::metadata::MetadataError;
    use ingot_minecraft::instance::{prepare_instance, ModLoaderType};

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
        let mut instance = prepare_instance(
            "Worlds Test".to_owned(),
            "1.21.4".to_owned(),
            ModLoaderType::Vanilla,
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

    fn assert_instance_not_found(error: WorldError, expected_id: &str) {
        assert!(matches!(
            &error,
            WorldError::InstanceNotFound(id) if id == expected_id
        ));
        assert!(matches!(ApiError::from(error), ApiError::NotFound { .. }));
    }

    #[tokio::test]
    async fn missing_instance_returns_not_found_without_creating_orphan_directory() {
        let dir = tempfile::tempdir().unwrap();
        let app = build_test_app(dir.path());
        let missing_id = format!("worlds-missing-{}", uuid::Uuid::new_v4());

        assert_instance_not_found(
            get_instance_worlds_full(app.handle(), missing_id.clone())
                .await
                .unwrap_err(),
            &missing_id,
        );
        assert_instance_not_found(
            reset_world_dimension(
                app.handle(),
                missing_id.clone(),
                "world".to_owned(),
                WorldDimension::Nether,
            )
            .await
            .unwrap_err(),
            &missing_id,
        );
        assert_instance_not_found(
            export_world(app.handle(), missing_id.clone(), "world".to_owned(), None)
                .await
                .unwrap_err(),
            &missing_id,
        );
        assert_instance_not_found(
            backup_world(app.handle(), missing_id.clone(), "world".to_owned())
                .await
                .unwrap_err(),
            &missing_id,
        );
        assert_instance_not_found(
            list_world_backups(app.handle(), missing_id.clone(), None)
                .await
                .unwrap_err(),
            &missing_id,
        );
        assert_instance_not_found(
            delete_world_backup(app.handle(), missing_id.clone(), "backup.zip".to_owned())
                .await
                .unwrap_err(),
            &missing_id,
        );
        assert_instance_not_found(
            import_world(app.handle(), missing_id.clone(), "world.zip".to_owned())
                .await
                .unwrap_err(),
            &missing_id,
        );
        assert_instance_not_found(
            resolve_world_dir(app.handle(), &missing_id, "world").unwrap_err(),
            &missing_id,
        );

        let orphan_dir = app
            .path()
            .app_data_dir()
            .unwrap()
            .join("instances")
            .join(&missing_id);
        assert!(!orphan_dir.exists());
    }

    #[tokio::test]
    async fn corrupt_and_recovery_blocked_metadata_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let instances_file = dir.path().join("instances.json");
        let corrupt = b"{corrupt-instances";
        std::fs::write(&instances_file, corrupt).unwrap();
        let app = build_test_app(dir.path());

        for error in [
            get_instance_worlds_full(app.handle(), "any-instance".to_owned())
                .await
                .unwrap_err(),
            list_world_backups(app.handle(), "any-instance".to_owned(), None)
                .await
                .unwrap_err(),
            backup_world(app.handle(), "any-instance".to_owned(), "world".to_owned())
                .await
                .unwrap_err(),
            resolve_world_dir(app.handle(), "any-instance", "world").unwrap_err(),
        ] {
            assert!(matches!(
                &error,
                WorldError::InstanceDir(InstanceError::Metadata(MetadataError::Decode {
                    path,
                    ..
                })) if path == &instances_file
            ));
            assert!(matches!(
                ApiError::from(error),
                ApiError::StorageFailure { .. }
            ));
        }
        assert_eq!(std::fs::read(&instances_file).unwrap(), corrupt);

        let recovery_dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(recovery_dir.path().join("instances.deletion.json")).unwrap();
        let recovery_app = build_test_app(recovery_dir.path());

        for error in [
            get_instance_worlds_full(recovery_app.handle(), "any-instance".to_owned())
                .await
                .unwrap_err(),
            list_world_backups(recovery_app.handle(), "any-instance".to_owned(), None)
                .await
                .unwrap_err(),
        ] {
            assert!(matches!(
                &error,
                WorldError::InstanceDir(InstanceError::Metadata(MetadataError::RecoveryBlocked(_)))
            ));
            assert!(matches!(
                ApiError::from(error),
                ApiError::IncompleteRecovery { .. }
            ));
        }
    }

    #[tokio::test]
    async fn registered_io_invalid_id_and_running_checks_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let app = build_test_app(dir.path());
        let io_id = format!("worlds-io-blocker-{}", uuid::Uuid::new_v4());
        let invalid_id = "../worlds-invalid";
        register_instance(app.handle(), &io_id);
        register_instance(app.handle(), invalid_id);

        let instances_root = app.path().app_data_dir().unwrap().join("instances");
        std::fs::create_dir_all(&instances_root).unwrap();
        let blocker = instances_root.join(&io_id);
        std::fs::write(&blocker, b"file-blocking-dir").unwrap();
        let _cleanup = RemoveFileOnDrop(blocker);

        let io_err = list_world_backups(app.handle(), io_id, None)
            .await
            .unwrap_err();
        assert!(matches!(
            &io_err,
            WorldError::InstanceDir(InstanceError::Io(_))
        ));
        assert!(matches!(
            ApiError::from(io_err),
            ApiError::StorageFailure { .. }
        ));

        let invalid_err = list_world_backups(app.handle(), invalid_id.to_owned(), None)
            .await
            .unwrap_err();
        assert!(matches!(
            &invalid_err,
            WorldError::InstanceDir(InstanceError::InvalidId)
        ));
        assert!(matches!(
            ApiError::from(invalid_err),
            ApiError::InvalidInput { .. }
        ));

        let _guard = crate::app::state(app.handle())
            .games
            .reserve_maintenance("running-inst")
            .unwrap();
        for error in [
            reset_world_dimension(
                app.handle(),
                "running-inst".to_owned(),
                "world".to_owned(),
                WorldDimension::TheEnd,
            )
            .await
            .unwrap_err(),
            export_world(
                app.handle(),
                "running-inst".to_owned(),
                "world".to_owned(),
                None,
            )
            .await
            .unwrap_err(),
            backup_world(app.handle(), "running-inst".to_owned(), "world".to_owned())
                .await
                .unwrap_err(),
            import_world(
                app.handle(),
                "running-inst".to_owned(),
                "world.zip".to_owned(),
            )
            .await
            .unwrap_err(),
        ] {
            assert!(matches!(error, WorldError::InstanceRunning));
            assert!(matches!(ApiError::from(error), ApiError::Conflict { .. }));
        }
    }
}
