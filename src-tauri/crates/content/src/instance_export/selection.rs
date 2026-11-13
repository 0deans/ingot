use std::path::{Path, PathBuf};

use super::error::InstanceExportError;
use super::models::ExportPreviewEntry;

/// Check if a relative path is permanently excluded from export.
pub fn is_permanently_excluded(rel_path: &str) -> bool {
    let normalized = rel_path.replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').filter(|p| !p.is_empty()).collect();

    if parts.is_empty() {
        return true;
    }

    for part in &parts {
        let lower = part.to_ascii_lowercase();
        if lower == ".ingot"
            || lower == ".ingot-trash"
            || lower == "logs"
            || lower == "crash-reports"
            || lower == "natives"
            || lower == ".fabric"
            || lower == ".mixin.out"
            || lower == ".cache"
            || lower == "downloads"
            || lower == "backups"
            || lower.starts_with(".staging_")
            || lower.starts_with(".qp_")
        {
            return true;
        }
    }

    if let Some(file_name) = parts.last() {
        let lower = file_name.to_ascii_lowercase();
        if lower == "session.lock"
            || lower == "usercache.json"
            || lower.ends_with(".tmp")
            || lower == ".ds_store"
            || lower == "thumbs.db"
        {
            return true;
        }
    }

    false
}

/// Check if an eligible relative path should be included by default in preview.
pub fn is_default_included(rel_path: &str) -> bool {
    if is_permanently_excluded(rel_path) {
        return false;
    }

    let normalized = rel_path.replace('\\', "/");
    let trimmed = normalized.trim_matches('/');

    let excluded_by_default = trimmed == "saves"
        || trimmed.starts_with("saves/")
        || trimmed == "screenshots"
        || trimmed.starts_with("screenshots/")
        || trimmed.eq_ignore_ascii_case("options.txt")
        || trimmed.eq_ignore_ascii_case("servers.dat");

    !excluded_by_default
}

/// Recursively scan an instance directory and collect exportable file entries.
pub fn collect_preview_entries(
    instance_dir: &Path,
) -> Result<(Vec<ExportPreviewEntry>, Vec<String>), InstanceExportError> {
    let mut entries = Vec::new();
    let mut warnings = Vec::new();

    walk_dir_entries(instance_dir, instance_dir, &mut entries, &mut warnings)?;
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    warnings.sort();

    Ok((entries, warnings))
}

fn walk_dir_entries(
    instance_dir: &Path,
    current_dir: &Path,
    entries: &mut Vec<ExportPreviewEntry>,
    warnings: &mut Vec<String>,
) -> Result<(), InstanceExportError> {
    let read_dir = match std::fs::read_dir(current_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(InstanceExportError::io(current_dir, e)),
    };

    for entry_res in read_dir {
        let entry = entry_res.map_err(|e| InstanceExportError::io(current_dir, e))?;
        let path = entry.path();
        let symlink_meta = match std::fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(InstanceExportError::io(&path, e)),
        };

        let file_type = symlink_meta.file_type();
        let rel = match path.strip_prefix(instance_dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let rel_str = rel.to_string_lossy().replace('\\', "/");

        if file_type.is_symlink() {
            warnings.push(format!("Skipped symlink or junction: {rel_str}"));
            continue;
        }

        if file_type.is_dir() {
            if is_permanently_excluded(&rel_str) {
                continue;
            }
            walk_dir_entries(instance_dir, &path, entries, warnings)?;
        } else if file_type.is_file() {
            if is_permanently_excluded(&rel_str) {
                continue;
            }
            if ingot_core::safe_enclosed_path(instance_dir, &rel_str).is_none() {
                return Err(InstanceExportError::UnsafePath(rel_str));
            }
            entries.push(ExportPreviewEntry {
                default_included: is_default_included(&rel_str),
                size_bytes: symlink_meta.len(),
                path: rel_str,
            });
        }
    }

    Ok(())
}

/// Validate user-provided list of relative paths for export.
pub fn validate_included_paths(
    instance_dir: &Path,
    included_paths: &[String],
) -> Result<Vec<(String, PathBuf)>, InstanceExportError> {
    let mut validated = Vec::with_capacity(included_paths.len());

    for raw_path in included_paths {
        let trimmed = raw_path.trim();
        if trimmed.is_empty() {
            return Err(InstanceExportError::UnsafePath(raw_path.clone()));
        }

        let normalized = trimmed.replace('\\', "/");
        if is_permanently_excluded(&normalized) {
            return Err(InstanceExportError::UnsafePath(normalized));
        }

        let full_path = ingot_core::safe_enclosed_path(instance_dir, &normalized)
            .ok_or_else(|| InstanceExportError::UnsafePath(normalized.clone()))?;

        let meta = std::fs::symlink_metadata(&full_path)
            .map_err(|e| InstanceExportError::io(&full_path, e))?;

        if meta.file_type().is_symlink() || !meta.file_type().is_file() {
            return Err(InstanceExportError::UnsafePath(normalized));
        }

        validated.push((normalized, full_path));
    }

    Ok(validated)
}
