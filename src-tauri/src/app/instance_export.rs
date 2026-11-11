use std::fs;
use std::path::PathBuf;

use tauri::{Manager, Runtime};

use ingot_artifacts::sources::SourceClient;
use ingot_content::instance_export::{
    export_instance, preview_export, ExportOptions, ExportPreview, ExportReport,
    InstanceExportError, SourceClientResolver,
};

pub(crate) async fn preview_instance_export<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: String,
) -> Result<ExportPreview, InstanceExportError> {
    let crate::app::instances::ResolvedInstance { config, dir } =
        crate::app::instances::resolve_registered_instance(app, &instance_id)?;

    tokio::task::spawn_blocking(move || preview_export(&dir, &config)).await?
}

pub(crate) async fn export_instance_archive<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: String,
    options: ExportOptions,
    dest: Option<String>,
) -> Result<ExportReport, InstanceExportError> {
    let guard = crate::app::state(app)
        .games
        .reserve_maintenance(&instance_id)
        .map_err(|_| InstanceExportError::InstanceRunning)?;

    let crate::app::instances::ResolvedInstance { config, dir } =
        crate::app::instances::resolve_registered_instance(app, &instance_id)?;

    let dest_path = match dest {
        Some(p) => PathBuf::from(p),
        None => {
            let cache_dir = app
                .path()
                .app_cache_dir()
                .map_err(|e| InstanceExportError::io(&dir, std::io::Error::other(e.to_string())))?
                .join("exports");
            fs::create_dir_all(&cache_dir).map_err(|e| InstanceExportError::io(&cache_dir, e))?;

            let sanitized_name: String = options
                .name
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '-' || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            let ext = options.format.file_extension();
            cache_dir.join(format!("{sanitized_name}-{}.{ext}", options.version.trim()))
        }
    };

    let state = crate::app::state(app);
    let client = SourceClient::new(&state.http);
    let resolver = SourceClientResolver { client };

    let report = export_instance(&dir, &config, options, &dest_path, &resolver).await?;

    drop(guard);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppPaths, AppState};
    use crate::ipc::ApiError;
    use ingot_content::instance_export::ExportFormat;
    use ingot_core::metadata::MetadataError;
    use ingot_minecraft::instance::{prepare_instance, InstanceError, ModLoaderType};
    use tauri::Manager;

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
            "Test Instance".to_owned(),
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

    fn test_export_options() -> ExportOptions {
        ExportOptions {
            format: ExportFormat::Modrinth,
            name: "Export Pack".to_owned(),
            version: "1.0.0".to_owned(),
            author: None,
            summary: None,
            included_paths: Vec::new(),
            file_sides: Vec::new(),
        }
    }

    #[tokio::test]
    async fn missing_instance_returns_not_found_without_creating_orphan_directory() {
        let dir = tempfile::tempdir().unwrap();
        let app = build_test_app(dir.path());
        let missing_id = format!("export-missing-{}", uuid::Uuid::new_v4());

        for error in [
            preview_instance_export(app.handle(), missing_id.clone())
                .await
                .unwrap_err(),
            export_instance_archive(
                app.handle(),
                missing_id.clone(),
                test_export_options(),
                None,
            )
            .await
            .unwrap_err(),
        ] {
            assert!(matches!(
                &error,
                InstanceExportError::Instance(InstanceError::NotFound(id)) if id == &missing_id
            ));
            assert!(matches!(ApiError::from(error), ApiError::NotFound { .. }));
        }

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
            preview_instance_export(app.handle(), "any-instance".to_owned())
                .await
                .unwrap_err(),
            export_instance_archive(
                app.handle(),
                "any-instance".to_owned(),
                test_export_options(),
                None,
            )
            .await
            .unwrap_err(),
        ] {
            assert!(matches!(
                &error,
                InstanceExportError::Instance(InstanceError::Metadata(MetadataError::Decode {
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

        let recovery_err = preview_instance_export(recovery_app.handle(), "any".to_owned())
            .await
            .unwrap_err();
        assert!(matches!(
            &recovery_err,
            InstanceExportError::Instance(InstanceError::Metadata(MetadataError::RecoveryBlocked(
                _
            )))
        ));
        assert!(matches!(
            ApiError::from(recovery_err),
            ApiError::IncompleteRecovery { .. }
        ));
    }

    #[tokio::test]
    async fn registered_io_invalid_id_and_running_checks_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let app = build_test_app(dir.path());
        let io_id = format!("export-io-blocker-{}", uuid::Uuid::new_v4());
        let invalid_id = "../export-invalid";
        register_instance(app.handle(), &io_id);
        register_instance(app.handle(), invalid_id);

        let instances_root = app.path().app_data_dir().unwrap().join("instances");
        std::fs::create_dir_all(&instances_root).unwrap();
        let blocker = instances_root.join(&io_id);
        std::fs::write(&blocker, b"file-blocking-dir").unwrap();
        let _cleanup = RemoveFileOnDrop(blocker);

        let io_err = preview_instance_export(app.handle(), io_id.clone())
            .await
            .unwrap_err();
        assert!(matches!(
            &io_err,
            InstanceExportError::Instance(InstanceError::Io(_))
        ));
        assert!(matches!(
            ApiError::from(io_err),
            ApiError::StorageFailure { .. }
        ));

        let invalid_err = preview_instance_export(app.handle(), invalid_id.to_owned())
            .await
            .unwrap_err();
        assert!(matches!(
            &invalid_err,
            InstanceExportError::Instance(InstanceError::InvalidId)
        ));
        assert!(matches!(
            ApiError::from(invalid_err),
            ApiError::InvalidInput { .. }
        ));

        let _guard = crate::app::state(app.handle())
            .games
            .reserve_maintenance("running-inst")
            .unwrap();
        let running_err = export_instance_archive(
            app.handle(),
            "running-inst".to_owned(),
            test_export_options(),
            None,
        )
        .await
        .unwrap_err();
        assert!(matches!(running_err, InstanceExportError::InstanceRunning));
        assert!(matches!(
            ApiError::from(running_err),
            ApiError::Conflict { .. }
        ));
    }
}
