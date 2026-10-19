//! Native path and repository integration for screenshot operations.
use crate::minecraft::screenshots::service::{self, ScreenshotKind, ScreenshotSource};
use crate::minecraft::screenshots::{self, ScreenshotError, ScreenshotInfo};
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
        Some(id) if !id.is_empty() => crate::app::instances::ensure_instance_dir(app, id)
            .map_err(ScreenshotError::InstanceDir)?
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
