use std::path::PathBuf;

use ingot_minecraft::instance::{InstanceConfig, InstanceError};
use ingot_minecraft::instance_content::{
    listing, mutation, ContentCategory, ContentListResult, InstanceContentError,
};
use tauri::{AppHandle, Runtime};

#[derive(Debug, thiserror::Error)]
pub(crate) enum InstanceContentActionError {
    #[error(transparent)]
    Content(#[from] InstanceContentError),
    #[error(transparent)]
    Instance(#[from] InstanceError),
    #[error(transparent)]
    Task(#[from] tokio::task::JoinError),
    #[error(transparent)]
    FileManager(#[from] super::platform::OpenDirectoryError),
}

struct ResolvedInstance {
    config: InstanceConfig,
    dir: PathBuf,
}

async fn ensure_instance_not_running<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: &str,
) -> Result<(), InstanceContentActionError> {
    super::lifecycle::adopt_running_games(app).await;
    if super::state(app).games.is_running(instance_id).await {
        return Err(InstanceContentError::InstanceRunning(instance_id.to_string()).into());
    }
    Ok(())
}

fn resolve_instance<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: &str,
) -> Result<ResolvedInstance, InstanceContentActionError> {
    let config = super::instances::load_instances(app)?
        .into_iter()
        .find(|instance| instance.id == instance_id)
        .ok_or_else(|| InstanceContentError::InstanceNotFound(instance_id.to_string()))?;
    let dir = super::instances::ensure_instance_dir(app, &config.id)?;
    Ok(ResolvedInstance { config, dir })
}

async fn run_blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, InstanceContentError> + Send + 'static,
) -> Result<T, InstanceContentActionError> {
    Ok(tokio::task::spawn_blocking(work).await??)
}

pub(crate) async fn list_content<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: &str,
    category: ContentCategory,
) -> Result<ContentListResult, InstanceContentActionError> {
    let instance_dir = resolve_instance(app, instance_id)?.dir;
    run_blocking(move || listing::list_content(&instance_dir, category)).await
}

pub(crate) async fn set_content_enabled<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: &str,
    category: ContentCategory,
    file_name: String,
    enabled: bool,
) -> Result<String, InstanceContentActionError> {
    ensure_instance_not_running(app, instance_id).await?;
    let instance_dir = resolve_instance(app, instance_id)?.dir;
    run_blocking(move || mutation::set_enabled(&instance_dir, category, &file_name, enabled)).await
}

pub(crate) async fn set_content_enabled_bulk<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: &str,
    category: ContentCategory,
    file_names: Vec<String>,
    enabled: bool,
) -> Result<Vec<String>, InstanceContentActionError> {
    ensure_instance_not_running(app, instance_id).await?;
    let instance_dir = resolve_instance(app, instance_id)?.dir;
    run_blocking(move || mutation::set_enabled_bulk(&instance_dir, category, &file_names, enabled))
        .await
}

pub(crate) async fn delete_content<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: &str,
    category: ContentCategory,
    file_names: Vec<String>,
) -> Result<Vec<String>, InstanceContentActionError> {
    ensure_instance_not_running(app, instance_id).await?;
    let instance_dir = resolve_instance(app, instance_id)?.dir;
    run_blocking(move || mutation::delete_content(&instance_dir, category, &file_names)).await
}

pub(crate) async fn open_content_folder<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: &str,
    category: ContentCategory,
    world: Option<String>,
) -> Result<(), InstanceContentActionError> {
    let instance_dir = resolve_instance(app, instance_id)?.dir;

    let path = match category {
        ContentCategory::DataPack => {
            if let Some(w) = world {
                instance_dir.join("saves").join(w).join("datapacks")
            } else {
                instance_dir.join("saves")
            }
        }
        _ => instance_dir.join(category.default_subfolder()),
    };

    std::fs::create_dir_all(&path).map_err(|e| InstanceContentError::io(&path, e))?;
    super::platform::open_directory(&path)?;

    Ok(())
}

pub(crate) async fn identify_content<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: &str,
    category: ContentCategory,
) -> Result<ingot_minecraft::instance_content::IdentifyResult, InstanceContentActionError> {
    let resolved = resolve_instance(app, instance_id)?;
    let game_version = resolved.config.game_version;
    let loader_id = ingot_version_change::targets::loader_id(&resolved.config.loader);
    let http = super::state(app).http.clone();

    Ok(ingot_minecraft::instance_content::identify_content(
        &resolved.dir,
        category,
        &game_version,
        &loader_id,
        &http,
    )
    .await?)
}

pub(crate) async fn check_content_updates<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: &str,
    category: ContentCategory,
) -> Result<ingot_minecraft::instance_content::UpdateCheckResult, InstanceContentActionError> {
    let resolved = resolve_instance(app, instance_id)?;
    let game_version = resolved.config.game_version;
    let loader_id = ingot_version_change::targets::loader_id(&resolved.config.loader);
    let http = super::state(app).http.clone();

    Ok(ingot_minecraft::instance_content::check_updates(
        &resolved.dir,
        category,
        &game_version,
        &loader_id,
        &http,
    )
    .await?)
}

pub(crate) async fn apply_content_updates<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: &str,
    category: ContentCategory,
    selections: Vec<ingot_minecraft::instance_content::ContentUpdateCandidate>,
) -> Result<ingot_minecraft::instance_content::ContentApplyReport, InstanceContentActionError> {
    ensure_instance_not_running(app, instance_id).await?;
    let instance_dir = resolve_instance(app, instance_id)?.dir;
    let http = super::state(app).http.clone();

    Ok(
        ingot_minecraft::instance_content::apply_updates(
            &instance_dir,
            category,
            selections,
            &http,
        )
        .await?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppPaths, AppState};
    use crate::ipc::ApiError;
    use ingot_core::metadata::MetadataError;
    use ingot_minecraft::instance::{prepare_instance, ModLoaderType};
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

    fn assert_instance_not_found(error: InstanceContentActionError, expected_id: &str) {
        assert!(matches!(
            &error,
            InstanceContentActionError::Content(InstanceContentError::InstanceNotFound(id))
                if id == expected_id
        ));
        assert!(matches!(ApiError::from(error), ApiError::NotFound { .. }));
    }

    #[tokio::test]
    async fn missing_instance_returns_not_found_without_creating_orphan_directory() {
        let dir = tempfile::tempdir().unwrap();
        let app = build_test_app(dir.path());
        let missing_id = format!("p07-missing-{}", uuid::Uuid::new_v4());

        assert_instance_not_found(
            list_content(app.handle(), &missing_id, ContentCategory::Mod)
                .await
                .unwrap_err(),
            &missing_id,
        );
        assert_instance_not_found(
            set_content_enabled(
                app.handle(),
                &missing_id,
                ContentCategory::Mod,
                "mod.jar".to_owned(),
                true,
            )
            .await
            .unwrap_err(),
            &missing_id,
        );
        assert_instance_not_found(
            set_content_enabled_bulk(
                app.handle(),
                &missing_id,
                ContentCategory::Mod,
                vec!["mod.jar".to_owned()],
                false,
            )
            .await
            .unwrap_err(),
            &missing_id,
        );
        assert_instance_not_found(
            delete_content(
                app.handle(),
                &missing_id,
                ContentCategory::Mod,
                vec!["mod.jar".to_owned()],
            )
            .await
            .unwrap_err(),
            &missing_id,
        );
        assert_instance_not_found(
            open_content_folder(app.handle(), &missing_id, ContentCategory::Mod, None)
                .await
                .unwrap_err(),
            &missing_id,
        );
        assert_instance_not_found(
            identify_content(app.handle(), &missing_id, ContentCategory::Mod)
                .await
                .unwrap_err(),
            &missing_id,
        );
        assert_instance_not_found(
            check_content_updates(app.handle(), &missing_id, ContentCategory::Mod)
                .await
                .unwrap_err(),
            &missing_id,
        );
        assert_instance_not_found(
            apply_content_updates(app.handle(), &missing_id, ContentCategory::Mod, Vec::new())
                .await
                .unwrap_err(),
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
    async fn corrupt_and_recovery_blocked_metadata_are_not_masked_as_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let instances_file = dir.path().join("instances.json");
        let corrupt = b"{corrupt-instances";
        std::fs::write(&instances_file, corrupt).unwrap();
        let app = build_test_app(dir.path());

        for error in [
            list_content(app.handle(), "any-instance", ContentCategory::Mod)
                .await
                .unwrap_err(),
            set_content_enabled(
                app.handle(),
                "any-instance",
                ContentCategory::Mod,
                "mod.jar".to_owned(),
                true,
            )
            .await
            .unwrap_err(),
            identify_content(app.handle(), "any-instance", ContentCategory::Mod)
                .await
                .unwrap_err(),
            check_content_updates(app.handle(), "any-instance", ContentCategory::Mod)
                .await
                .unwrap_err(),
        ] {
            assert!(matches!(
                &error,
                InstanceContentActionError::Instance(InstanceError::Metadata(
                    MetadataError::Decode { path, .. }
                )) if path == &instances_file
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
            list_content(recovery_app.handle(), "any-instance", ContentCategory::Mod)
                .await
                .unwrap_err(),
            check_content_updates(recovery_app.handle(), "any-instance", ContentCategory::Mod)
                .await
                .unwrap_err(),
        ] {
            assert!(matches!(
                &error,
                InstanceContentActionError::Instance(InstanceError::Metadata(
                    MetadataError::RecoveryBlocked(_)
                ))
            ));
            assert!(matches!(
                ApiError::from(error),
                ApiError::IncompleteRecovery { .. }
            ));
        }
    }

    #[tokio::test]
    async fn ensure_instance_dir_io_and_invalid_id_failures_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let app = build_test_app(dir.path());
        let io_id = format!("p07-io-blocker-{}", uuid::Uuid::new_v4());
        let invalid_id = "../p07-invalid";
        register_instance(app.handle(), &io_id);
        register_instance(app.handle(), invalid_id);

        let instances_root = app.path().app_data_dir().unwrap().join("instances");
        std::fs::create_dir_all(&instances_root).unwrap();
        let blocker = instances_root.join(&io_id);
        std::fs::write(&blocker, b"file-blocking-dir").unwrap();
        let _cleanup = RemoveFileOnDrop(blocker);

        let io_error = list_content(app.handle(), &io_id, ContentCategory::Mod)
            .await
            .unwrap_err();
        assert!(matches!(
            &io_error,
            InstanceContentActionError::Instance(InstanceError::Io(_))
        ));
        assert!(matches!(
            ApiError::from(io_error),
            ApiError::StorageFailure { .. }
        ));

        let invalid_error = list_content(app.handle(), invalid_id, ContentCategory::Mod)
            .await
            .unwrap_err();
        assert!(matches!(
            &invalid_error,
            InstanceContentActionError::Instance(InstanceError::InvalidId)
        ));
        assert!(matches!(
            ApiError::from(invalid_error),
            ApiError::InvalidInput { .. }
        ));
    }

    #[tokio::test]
    async fn running_instance_short_circuits_before_instance_resolution() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("instances.json"), b"{corrupt").unwrap();
        let app = build_test_app(dir.path());
        let running_id = "running-inst";
        let _guard = crate::app::state(app.handle())
            .games
            .reserve_maintenance(running_id)
            .unwrap();

        for error in [
            set_content_enabled(
                app.handle(),
                running_id,
                ContentCategory::Mod,
                "mod.jar".to_owned(),
                true,
            )
            .await
            .unwrap_err(),
            set_content_enabled_bulk(
                app.handle(),
                running_id,
                ContentCategory::Mod,
                vec!["mod.jar".to_owned()],
                false,
            )
            .await
            .unwrap_err(),
            delete_content(
                app.handle(),
                running_id,
                ContentCategory::Mod,
                vec!["mod.jar".to_owned()],
            )
            .await
            .unwrap_err(),
            apply_content_updates(app.handle(), running_id, ContentCategory::Mod, Vec::new())
                .await
                .unwrap_err(),
        ] {
            assert!(matches!(
                &error,
                InstanceContentActionError::Content(InstanceContentError::InstanceRunning(id))
                    if id == running_id
            ));
            assert!(matches!(ApiError::from(error), ApiError::Conflict { .. }));
        }
    }

    #[tokio::test]
    async fn blocking_task_panic_maps_to_task_and_internal_api_error() {
        let panic_error = run_blocking::<()>(|| panic!("simulated blocking worker panic"))
            .await
            .unwrap_err();
        assert!(matches!(panic_error, InstanceContentActionError::Task(_)));
        assert!(matches!(
            ApiError::from(panic_error),
            ApiError::Internal { .. }
        ));

        let domain_error =
            run_blocking::<()>(|| Err(InstanceContentError::NotFound("mod.jar".to_owned())))
                .await
                .unwrap_err();
        assert!(matches!(
            domain_error,
            InstanceContentActionError::Content(InstanceContentError::NotFound(_))
        ));
    }
}
