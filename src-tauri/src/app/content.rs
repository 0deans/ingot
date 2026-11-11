use ingot_content::content::{self, ContentError};
use ingot_minecraft::instance::InstanceConfig;
use ingot_minecraft::launcher::progress::make_progress_reporter;
use tauri::{AppHandle, Runtime};

pub(crate) fn service<R: Runtime>(app: &AppHandle<R>) -> content::service::ContentService {
    content::service::ContentService::new(super::state(app).http.clone())
}

pub(crate) async fn install_content_file<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: &str,
    project_type: &str,
    download_url: &str,
    filename: &str,
) -> Result<String, ContentError> {
    let handle = app.clone();
    let id = instance_id.to_owned();
    let crate::app::instances::ResolvedInstance {
        config: instance,
        dir: directory,
    } = tokio::task::spawn_blocking(move || {
        crate::app::instances::resolve_registered_instance(&handle, &id)
    })
    .await
    .map_err(ContentError::Task)?
    .map_err(ContentError::Instance)?;

    if project_type.eq_ignore_ascii_case("mod") && !instance.loader.supports_mods() {
        return Err(ContentError::UnsupportedLoader {
            instance: instance.name,
            loader: instance.loader,
            category: "mods",
        });
    }

    content::install::install_content_file(
        &super::state(app).http,
        &directory,
        project_type,
        download_url,
        filename,
    )
    .await
}

pub(crate) async fn install_modpack_instance<R: Runtime>(
    app: &AppHandle<R>,
    name: &str,
    source: &str,
    download_url: &str,
    filename: &str,
) -> Result<InstanceConfig, ContentError> {
    let handle = app.clone();
    let root =
        tokio::task::spawn_blocking(move || crate::app::instances::ensure_instances_dir(&handle))
            .await
            .map_err(ContentError::Task)?
            .map_err(ContentError::Instance)?;
    let state = super::state(app);
    let tracking_id = format!("installing:{}", name.trim());
    let app_prog = app.clone();
    let on_progress = make_progress_reporter(tracking_id, move |ev| {
        crate::app::events::launch_progress(&app_prog, ev);
    });
    let installer = content::install::ModpackInstaller {
        client: state.http.clone(),
        instances: std::sync::Arc::clone(&state.instances),
        root,
        curseforge_base: content::curseforge::DEFAULT_CURSEFORGE_BASE.into(),
        on_progress: Some(on_progress),
    };
    installer
        .install(name, source, download_url, filename)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppPaths, AppState};
    use crate::ipc::ApiError;
    use ingot_core::metadata::MetadataError;
    use ingot_minecraft::instance::{prepare_instance, InstanceError, ModLoaderType};
    use std::path::PathBuf;
    use tauri::Manager;

    struct RemoveFileOnDrop(PathBuf);

    impl Drop for RemoveFileOnDrop {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    struct RemoveDirOnDrop(PathBuf);

    impl Drop for RemoveDirOnDrop {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn build_test_app(dir: &std::path::Path) -> tauri::App<tauri::test::MockRuntime> {
        let state = AppState::new(AppPaths::new(dir.to_owned())).unwrap();
        tauri::test::mock_builder()
            .manage(state)
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap()
    }

    fn register_instance(app: &AppHandle<tauri::test::MockRuntime>, id: &str) {
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

    #[tokio::test]
    async fn install_content_file_rejects_missing_instance_without_orphan_dir() {
        let dir = tempfile::tempdir().unwrap();
        let app = build_test_app(dir.path());
        let missing_id = format!("content-missing-{}", uuid::Uuid::new_v4());

        let err = install_content_file(
            app.handle(),
            &missing_id,
            "mod",
            "https://example.invalid/mod.jar",
            "mod.jar",
        )
        .await
        .unwrap_err();
        assert!(matches!(
            &err,
            ContentError::Instance(InstanceError::NotFound(id)) if id == &missing_id
        ));
        assert!(matches!(ApiError::from(err), ApiError::NotFound { .. }));

        let orphan_dir = app
            .path()
            .app_data_dir()
            .unwrap()
            .join("instances")
            .join(&missing_id);
        assert!(!orphan_dir.exists());
    }

    #[tokio::test]
    async fn install_content_file_preserves_metadata_io_and_invalid_id_errors() {
        let corrupt_dir = tempfile::tempdir().unwrap();
        let instances_file = corrupt_dir.path().join("instances.json");
        let corrupt = b"{corrupt-instances";
        std::fs::write(&instances_file, corrupt).unwrap();
        let corrupt_app = build_test_app(corrupt_dir.path());

        let decode_err = install_content_file(
            corrupt_app.handle(),
            "any-instance",
            "mod",
            "https://example.invalid/mod.jar",
            "mod.jar",
        )
        .await
        .unwrap_err();
        assert!(matches!(
            &decode_err,
            ContentError::Instance(InstanceError::Metadata(MetadataError::Decode { path, .. }))
                if path == &instances_file
        ));
        assert!(matches!(
            ApiError::from(decode_err),
            ApiError::StorageFailure { .. }
        ));
        assert_eq!(std::fs::read(&instances_file).unwrap(), corrupt);

        let recovery_dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(recovery_dir.path().join("instances.deletion.json")).unwrap();
        let recovery_app = build_test_app(recovery_dir.path());

        let recovery_err = install_content_file(
            recovery_app.handle(),
            "any-instance",
            "mod",
            "https://example.invalid/mod.jar",
            "mod.jar",
        )
        .await
        .unwrap_err();
        assert!(matches!(
            &recovery_err,
            ContentError::Instance(InstanceError::Metadata(MetadataError::RecoveryBlocked(_)))
        ));
        assert!(matches!(
            ApiError::from(recovery_err),
            ApiError::IncompleteRecovery { .. }
        ));

        let io_dir = tempfile::tempdir().unwrap();
        let app = build_test_app(io_dir.path());
        let io_id = format!("content-io-blocker-{}", uuid::Uuid::new_v4());
        let invalid_id = "../content-invalid";
        let vanilla_id = format!("content-vanilla-{}", uuid::Uuid::new_v4());
        register_instance(app.handle(), &io_id);
        register_instance(app.handle(), invalid_id);
        register_instance(app.handle(), &vanilla_id);

        let instances_root = app.path().app_data_dir().unwrap().join("instances");
        std::fs::create_dir_all(&instances_root).unwrap();
        let blocker = instances_root.join(&io_id);
        std::fs::write(&blocker, b"file-blocking-dir").unwrap();
        let _blocker_cleanup = RemoveFileOnDrop(blocker);
        let _vanilla_cleanup = RemoveDirOnDrop(instances_root.join(&vanilla_id));

        let io_err = install_content_file(
            app.handle(),
            &io_id,
            "mod",
            "https://example.invalid/mod.jar",
            "mod.jar",
        )
        .await
        .unwrap_err();
        assert!(matches!(
            &io_err,
            ContentError::Instance(InstanceError::Io(_))
        ));
        assert!(matches!(
            ApiError::from(io_err),
            ApiError::StorageFailure { .. }
        ));

        let invalid_err = install_content_file(
            app.handle(),
            invalid_id,
            "mod",
            "https://example.invalid/mod.jar",
            "mod.jar",
        )
        .await
        .unwrap_err();
        assert!(matches!(
            &invalid_err,
            ContentError::Instance(InstanceError::InvalidId)
        ));
        assert!(matches!(
            ApiError::from(invalid_err),
            ApiError::InvalidInput { .. }
        ));

        let unsupported_err = install_content_file(
            app.handle(),
            &vanilla_id,
            "mod",
            "https://example.invalid/mod.jar",
            "mod.jar",
        )
        .await
        .unwrap_err();
        assert!(matches!(
            unsupported_err,
            ContentError::UnsupportedLoader { .. }
        ));
    }
}
