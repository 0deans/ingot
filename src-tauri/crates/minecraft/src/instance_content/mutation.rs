use std::fs;
use std::path::Path;

use super::error::InstanceContentError;
use super::file_name::ContentFileName;
use super::models::ContentCategory;
use super::tracking::{read_tracking, write_tracking};

pub fn set_enabled(
    instance_dir: &Path,
    category: ContentCategory,
    file_name: &str,
    enabled: bool,
) -> Result<String, InstanceContentError> {
    let parsed = ContentFileName::parse(file_name, category)?;
    if parsed.is_disabled() == !enabled {
        // Already in desired state
        return Ok(parsed.to_string());
    }

    let folder = instance_dir.join(category.default_subfolder());
    let src_path = folder.join(parsed.as_str());
    if !src_path.exists() {
        return Err(InstanceContentError::NotFound(parsed.to_string()));
    }

    let target = parsed.with_enabled_state(enabled);
    let target_path = folder.join(target.as_str());

    if target_path.exists() {
        return Err(InstanceContentError::NameConflict(target.to_string()));
    }

    fs::rename(&src_path, &target_path).map_err(|e| InstanceContentError::io(target_path, e))?;

    Ok(target.to_string())
}

pub fn set_enabled_bulk(
    instance_dir: &Path,
    category: ContentCategory,
    file_names: &[String],
    enabled: bool,
) -> Result<Vec<String>, InstanceContentError> {
    let mut updated = Vec::with_capacity(file_names.len());
    for file_name in file_names {
        let res = set_enabled(instance_dir, category, file_name, enabled)?;
        updated.push(res);
    }
    Ok(updated)
}

pub fn delete_content(
    instance_dir: &Path,
    category: ContentCategory,
    file_names: &[String],
) -> Result<Vec<String>, InstanceContentError> {
    let folder = instance_dir.join(category.default_subfolder());
    let mut tracking = read_tracking(instance_dir).unwrap_or_default();
    let mut tracking_changed = false;
    let mut deleted = Vec::new();

    for raw in file_names {
        let parsed = ContentFileName::parse(raw, category)?;
        let path = folder.join(parsed.as_str());

        if path.is_file() {
            fs::remove_file(&path).map_err(|e| InstanceContentError::io(&path, e))?;
            deleted.push(parsed.to_string());
        } else if path.is_dir() {
            fs::remove_dir_all(&path).map_err(|e| InstanceContentError::io(&path, e))?;
            deleted.push(parsed.to_string());
        }

        if tracking.remove(parsed.base_name()).is_some() {
            tracking_changed = true;
        }
    }

    if tracking_changed {
        let _ = write_tracking(instance_dir, &tracking);
    }

    Ok(deleted)
}
