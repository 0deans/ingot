//! Native path and repository integration for screenshot operations.
use ingot_minecraft::screenshots::service::{self, ScreenshotKind, ScreenshotSource};
use ingot_minecraft::screenshots::{self, ScreenshotError, ScreenshotInfo};
use std::path::{Path, PathBuf};
use tauri::{Manager, Runtime};

fn sources<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<ScreenshotSource>, ScreenshotError> {
    let mut sources = Vec::new();
    for instance in
        crate::app::instances::load_instances(app).map_err(ScreenshotError::LoadInstances)?
    {
        let directory = crate::app::instances::ensure_instance_dir(app, &instance.id)
            .map_err(ScreenshotError::InstanceDir)?
            .join("screenshots");
        sources.push(ScreenshotSource {
            instance_id: instance.id,
            instance_name: instance.name,
            directory,
            kind: ScreenshotKind::Java,
        });
    }
    for directory in screenshots::get_bedrock_screenshot_dirs() {
        sources.push(ScreenshotSource {
            instance_id: "bedrock".into(),
            instance_name: "Bedrock Edition".into(),
            directory,
            kind: ScreenshotKind::Bedrock,
        });
    }
    Ok(sources)
}

fn cache_dir<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, ScreenshotError> {
    let root = app
        .path()
        .app_cache_dir()
        .map_err(|source| ScreenshotError::CacheDir(std::io::Error::other(source)))?;
    screenshots::get_thumbnail_cache_dir(&root)
}

pub(crate) async fn get_all_screenshots<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<ScreenshotInfo>, ScreenshotError> {
    let app = app.clone();
    tokio::task::spawn_blocking(move || {
        let sources = sources(&app)?;
        let cache = cache_dir(&app)?;
        service::get_all_screenshots(&sources, &cache)
    })
    .await?
}

pub(crate) async fn get_or_create_thumbnail<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: String,
    file_name: String,
    file_path: String,
    modified_at: u64,
) -> Result<String, ScreenshotError> {
    let app = app.clone();
    tokio::task::spawn_blocking(move || {
        let sources = sources(&app)?
            .into_iter()
            .filter(|source| source.instance_id == instance_id)
            .collect::<Vec<_>>();
        let cache = cache_dir(&app)?;
        let item = service::resolve_screenshot(&sources, &cache, &instance_id, &file_name)?;
        service::validate_thumbnail_request(&item, Path::new(&file_path), modified_at)?;
        screenshots::thumbnails::get_or_create_thumbnail(
            &cache,
            &instance_id,
            &file_name,
            &item.file_path,
            item.modified_at,
        )
    })
    .await?
}

pub(crate) async fn delete_screenshot<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    file_name: &str,
) -> Result<(), ScreenshotError> {
    let app = app.clone();
    let instance_id = instance_id.to_string();
    let file_name = file_name.to_string();
    tokio::task::spawn_blocking(move || {
        let sources = sources(&app)?
            .into_iter()
            .filter(|source| source.instance_id == instance_id)
            .collect::<Vec<_>>();
        let cache = cache_dir(&app)?;
        service::delete_screenshot(&sources, &cache, &instance_id, &file_name)
    })
    .await?
}

pub(crate) fn open_screenshots_folder<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: Option<&str>,
) -> Result<(), ScreenshotActionError> {
    let directory = match instance_id {
        Some("bedrock") => screenshots::get_primary_bedrock_screenshots_folder(),
        Some(id) if !id.is_empty() => crate::app::instances::resolve_registered_instance(app, id)
            .map_err(ScreenshotError::InstanceDir)?
            .dir
            .join("screenshots"),
        _ => crate::app::instances::ensure_instances_dir(app)
            .map_err(ScreenshotError::InstanceDir)?,
    };
    std::fs::create_dir_all(&directory).map_err(ScreenshotError::Io)?;
    super::platform::open_directory(&directory).map_err(ScreenshotActionError::FileManager)
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ScreenshotActionError {
    #[error(transparent)]
    Screenshot(#[from] ScreenshotError),
    #[error(transparent)]
    FileManager(#[from] super::platform::OpenDirectoryError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppPaths, AppState};
    use crate::ipc::ApiError;
    use ingot_core::metadata::MetadataError;
    use ingot_minecraft::instance::{prepare_instance, InstanceError, ModLoaderType};

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
            "Screenshots Test".to_owned(),
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

    #[test]
    fn open_screenshots_folder_rejects_missing_instance_without_orphan_dir() {
        let dir = tempfile::tempdir().unwrap();
        let app = build_test_app(dir.path());
        let missing_id = format!("screenshots-missing-{}", uuid::Uuid::new_v4());

        let err = open_screenshots_folder(app.handle(), Some(&missing_id)).unwrap_err();
        assert!(matches!(
            &err,
            ScreenshotActionError::Screenshot(ScreenshotError::InstanceDir(
                InstanceError::NotFound(id)
            )) if id == &missing_id
        ));
        let api_err = match err {
            ScreenshotActionError::Screenshot(e) => ApiError::from(e),
            ScreenshotActionError::FileManager(e) => ApiError::from(e),
        };
        assert!(matches!(api_err, ApiError::NotFound { .. }));

        let orphan_dir = app
            .path()
            .app_data_dir()
            .unwrap()
            .join("instances")
            .join(&missing_id);
        assert!(!orphan_dir.exists());
    }

    #[test]
    fn open_screenshots_folder_preserves_metadata_io_and_invalid_id_errors() {
        let corrupt_dir = tempfile::tempdir().unwrap();
        let instances_file = corrupt_dir.path().join("instances.json");
        let corrupt = b"{corrupt-instances";
        std::fs::write(&instances_file, corrupt).unwrap();
        let corrupt_app = build_test_app(corrupt_dir.path());

        let decode_err = open_screenshots_folder(corrupt_app.handle(), Some("any")).unwrap_err();
        assert!(matches!(
            &decode_err,
            ScreenshotActionError::Screenshot(ScreenshotError::InstanceDir(
                InstanceError::Metadata(MetadataError::Decode { path, .. })
            )) if path == &instances_file
        ));
        assert_eq!(std::fs::read(&instances_file).unwrap(), corrupt);

        let recovery_dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(recovery_dir.path().join("instances.deletion.json")).unwrap();
        let recovery_app = build_test_app(recovery_dir.path());

        let recovery_err = open_screenshots_folder(recovery_app.handle(), Some("any")).unwrap_err();
        assert!(matches!(
            &recovery_err,
            ScreenshotActionError::Screenshot(ScreenshotError::InstanceDir(
                InstanceError::Metadata(MetadataError::RecoveryBlocked(_))
            ))
        ));

        let io_dir = tempfile::tempdir().unwrap();
        let app = build_test_app(io_dir.path());
        let io_id = format!("screenshots-io-blocker-{}", uuid::Uuid::new_v4());
        let invalid_id = "../screenshots-invalid";
        register_instance(app.handle(), &io_id);
        register_instance(app.handle(), invalid_id);

        let instances_root = app.path().app_data_dir().unwrap().join("instances");
        std::fs::create_dir_all(&instances_root).unwrap();
        let blocker = instances_root.join(&io_id);
        std::fs::write(&blocker, b"file-blocking-dir").unwrap();
        let _cleanup = RemoveFileOnDrop(blocker);

        let io_err = open_screenshots_folder(app.handle(), Some(&io_id)).unwrap_err();
        assert!(matches!(
            &io_err,
            ScreenshotActionError::Screenshot(ScreenshotError::InstanceDir(InstanceError::Io(_)))
        ));

        let invalid_err = open_screenshots_folder(app.handle(), Some(invalid_id)).unwrap_err();
        assert!(matches!(
            &invalid_err,
            ScreenshotActionError::Screenshot(ScreenshotError::InstanceDir(
                InstanceError::InvalidId
            ))
        ));
    }
}
