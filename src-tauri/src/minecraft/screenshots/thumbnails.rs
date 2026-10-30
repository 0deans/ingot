//! Thumbnail generation and caching for screenshots.
use super::models::ScreenshotError;
use md5::Digest;
use std::fs;
use std::path::{Path, PathBuf};

pub const SUPPORTED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];

/// Resolves and ensures the cache directory for screenshot thumbnails.
pub fn get_thumbnail_cache_dir(root: &Path) -> Result<PathBuf, ScreenshotError> {
    let cache = root.join("screenshot_thumbnails");
    fs::create_dir_all(&cache).map_err(ScreenshotError::Io)?;
    Ok(cache)
}
/// Generates a deterministic thumbnail path based on an MD5 hash of instance, name, and modification timestamp.
pub fn get_thumbnail_file_path(
    cache_dir: &Path,
    instance_id: &str,
    file_name: &str,
    modified_at: u64,
) -> PathBuf {
    let key = format!("{instance_id}:{file_name}:{modified_at}");
    let hash: String = md5::Md5::digest(key.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    cache_dir.join(format!("{hash}.jpg"))
}

/// Generates a 480x270 JPEG thumbnail for the given source image.
pub fn create_thumbnail(source_path: &Path, dest_path: &Path) -> Result<(), ScreenshotError> {
    if cached_thumbnail_exists(dest_path)? {
        return Ok(());
    }

    let img = image::open(source_path).map_err(|e| ScreenshotError::Image {
        path: source_path.display().to_string(),
        source: e,
    })?;

    let thumb = img.thumbnail(480, 270);

    let parent = dest_path
        .parent()
        .ok_or_else(|| ScreenshotError::MissingCacheParent(dest_path.to_path_buf()))?;
    fs::create_dir_all(parent).map_err(ScreenshotError::Io)?;
    let mut staged = tempfile::NamedTempFile::new_in(parent).map_err(ScreenshotError::Io)?;
    thumb
        .write_to(&mut staged, image::ImageFormat::Jpeg)
        .map_err(|source| ScreenshotError::Image {
            path: dest_path.display().to_string(),
            source,
        })?;
    staged.as_file().sync_all().map_err(ScreenshotError::Io)?;
    staged
        .persist(dest_path)
        .map_err(|error| ScreenshotError::Io(error.error))?;
    Ok(())
}

/// Retrieves an existing thumbnail or generates one immediately if missing.
pub fn get_or_create_thumbnail(
    cache_dir: &Path,
    instance_id: &str,
    file_name: &str,
    file_path: &str,
    modified_at: u64,
) -> Result<String, ScreenshotError> {
    let thumb_path = get_thumbnail_file_path(cache_dir, instance_id, file_name, modified_at);

    if cached_thumbnail_exists(&thumb_path)? {
        return Ok(thumb_path.to_string_lossy().to_string());
    }

    create_thumbnail(Path::new(file_path), &thumb_path)?;
    Ok(thumb_path.to_string_lossy().to_string())
}

pub(super) fn cached_thumbnail_exists(path: &Path) -> Result<bool, ScreenshotError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(true),
        Ok(_) => Err(ScreenshotError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Thumbnail cache entry must be a regular file",
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(ScreenshotError::Io(error)),
    }
}
