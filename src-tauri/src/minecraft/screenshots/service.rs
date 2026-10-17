//! Path-based screenshot indexing and deletion; native adapters provide sources.
use super::discovery::{get_companion_capture_time, scan_screenshot_files};
use super::models::{ScreenshotError, ScreenshotInfo};
use super::thumbnails::{cached_thumbnail_exists, get_thumbnail_file_path};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScreenshotKind {
    Java,
    Bedrock,
}

pub(crate) struct ScreenshotSource {
    pub(crate) instance_id: String,
    pub(crate) instance_name: String,
    pub(crate) directory: PathBuf,
    pub(crate) kind: ScreenshotKind,
}

pub(crate) fn get_all_screenshots(
    sources: &[ScreenshotSource],
    cache: &Path,
) -> Result<Vec<ScreenshotInfo>, ScreenshotError> {
    let mut screenshots = Vec::new();
    for source in sources {
        let depth = if source.kind == ScreenshotKind::Bedrock {
            3
        } else {
            1
        };
        for path in scan_screenshot_files(&source.directory, depth).map_err(ScreenshotError::Io)? {
            let metadata = fs::metadata(&path).map_err(ScreenshotError::Io)?;
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| ScreenshotError::InvalidFilename(path.display().to_string()))?
                .to_string();
            let modified = metadata.modified().map_err(ScreenshotError::Io)?;
            let modified_at = modified
                .duration_since(UNIX_EPOCH)
                .map_err(|error| {
                    ScreenshotError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, error))
                })?
                .as_millis() as u64;
            // Creation time is not supported by every filesystem; modification is
            // the deliberate fallback, not a substitute for a failed required read.
            let created_at = metadata
                .created()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis() as u64)
                .unwrap_or(modified_at);
            let capture_time = if source.kind == ScreenshotKind::Bedrock {
                get_companion_capture_time(&path)
            } else {
                None
            };
            let modified_at = capture_time.unwrap_or(modified_at);
            let created_at = capture_time.unwrap_or(created_at);
            let thumbnail =
                get_thumbnail_file_path(cache, &source.instance_id, &file_name, modified_at);
            let thumbnail_path = cached_thumbnail_exists(&thumbnail)?
                .then(|| thumbnail.to_string_lossy().into_owned());
            screenshots.push(ScreenshotInfo {
                id: format!("{}_{}", source.instance_id, file_name),
                instance_id: source.instance_id.clone(),
                instance_name: source.instance_name.clone(),
                file_name,
                file_path: path.to_string_lossy().into_owned(),
                thumbnail_path,
                file_size_bytes: metadata.len(),
                created_at,
                modified_at,
            });
        }
    }
    screenshots.sort_by_key(|item| std::cmp::Reverse(item.modified_at));
    Ok(screenshots)
}

pub(crate) fn resolve_screenshot(
    sources: &[ScreenshotSource],
    cache: &Path,
    instance_id: &str,
    file_name: &str,
) -> Result<ScreenshotInfo, ScreenshotError> {
    if crate::app::safe_enclosed_filename(cache, file_name).is_none() {
        return Err(ScreenshotError::InvalidFilename(file_name.to_string()));
    }
    let items = get_all_screenshots(sources, cache)?;
    let mut matching = items
        .into_iter()
        .filter(|item| item.instance_id == instance_id && item.file_name == file_name);
    let item = matching
        .next()
        .ok_or_else(|| ScreenshotError::NotFound(file_name.to_string()))?;
    if matching.next().is_some() {
        return Err(ScreenshotError::AmbiguousFilename(file_name.to_string()));
    }
    Ok(item)
}

pub(crate) fn delete_screenshot(
    sources: &[ScreenshotSource],
    cache: &Path,
    instance_id: &str,
    file_name: &str,
) -> Result<(), ScreenshotError> {
    let item = resolve_screenshot(sources, cache, instance_id, file_name)?;
    let path = Path::new(&item.file_path);
    fs::remove_file(path).map_err(ScreenshotError::Io)?;
    if instance_id == "bedrock" {
        for extension in ["json", "mc"] {
            remove_if_present(&path.with_extension(extension))?;
        }
    }
    remove_if_present(&get_thumbnail_file_path(
        cache,
        instance_id,
        file_name,
        item.modified_at,
    ))?;
    Ok(())
}

pub(crate) fn validate_thumbnail_request(
    item: &ScreenshotInfo,
    requested_path: &Path,
    modified_at: u64,
) -> Result<(), ScreenshotError> {
    if Path::new(&item.file_path) != requested_path || item.modified_at != modified_at {
        return Err(ScreenshotError::StaleMetadata);
    }
    Ok(())
}

fn remove_if_present(path: &Path) -> Result<(), ScreenshotError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(ScreenshotError::Io(error)),
    }
}
