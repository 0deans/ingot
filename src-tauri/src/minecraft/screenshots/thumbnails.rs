//! Thumbnail generation and caching for screenshots.
use super::models::ScreenshotError;
use md5::Digest;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{Manager, Runtime};

pub const SUPPORTED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];

/// Resolves and ensures the cache directory for screenshot thumbnails.
pub fn get_thumbnail_cache_dir<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<PathBuf, ScreenshotError> {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| ScreenshotError::CacheDir(e.to_string()))?
        .join("screenshot_thumbnails");

    if !cache_dir.exists() {
        let _ = fs::create_dir_all(&cache_dir);
    }
    Ok(cache_dir)
}

/// Generates a deterministic thumbnail path based on an MD5 hash of instance, name, and modification timestamp.
pub fn get_thumbnail_file_path(
    cache_dir: &Path,
    instance_id: &str,
    file_name: &str,
    modified_at: u64,
) -> PathBuf {
    let key = format!("{instance_id}:{file_name}:{modified_at}");
    let hash = format!("{:x}", md5::Md5::digest(key.as_bytes()));
    cache_dir.join(format!("{hash}.jpg"))
}

/// Generates a 480x270 JPEG thumbnail for the given source image.
pub fn create_thumbnail(source_path: &Path, dest_path: &Path) -> Result<(), ScreenshotError> {
    if dest_path.exists() {
        return Ok(());
    }

    let img = image::open(source_path).map_err(|e| ScreenshotError::Image {
        path: source_path.display().to_string(),
        source: e,
    })?;

    let thumb = img.thumbnail(480, 270);

    if let Some(parent) = dest_path.parent() {
        if !parent.exists() {
            let _ = fs::create_dir_all(parent);
        }
    }

    thumb
        .save_with_format(dest_path, image::ImageFormat::Jpeg)
        .map_err(|e| ScreenshotError::Image {
            path: dest_path.display().to_string(),
            source: e,
        })?;

    Ok(())
}

/// Retrieves an existing thumbnail or generates one immediately if missing.
pub fn get_or_create_thumbnail<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    file_name: &str,
    file_path: &str,
    modified_at: u64,
) -> Result<String, ScreenshotError> {
    let cache_dir = get_thumbnail_cache_dir(app)?;
    let thumb_path = get_thumbnail_file_path(&cache_dir, instance_id, file_name, modified_at);

    if thumb_path.exists() {
        return Ok(thumb_path.to_string_lossy().to_string());
    }

    create_thumbnail(Path::new(file_path), &thumb_path)?;
    Ok(thumb_path.to_string_lossy().to_string())
}
