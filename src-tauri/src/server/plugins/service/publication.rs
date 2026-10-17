use super::super::models::PluginError;
use std::path::{Path, PathBuf};
/// Atomically publishes a staged plugin file to `target_dest`, preserving any existing file
/// under a backup path during tracking update, and rolling back cleanly if tracking fails.
/// Retains and reports both primary and rollback failures if rollback encounters errors.
pub fn publish_installed_plugin<F>(
    staging_file: &Path,
    target_dest: &Path,
    title: &str,
    commit_tracking: F,
    old_files: &[PathBuf],
) -> Result<(), PluginError>
where
    F: FnOnce() -> Result<(), PluginError>,
{
    let _ = title;
    publish_plugin_batch(
        &[(staging_file.to_owned(), target_dest.to_owned())],
        commit_tracking,
        old_files,
    )
}

pub(super) fn publish_plugin_batch<F>(
    replacements: &[(PathBuf, PathBuf)],
    commit_tracking: F,
    old_files: &[PathBuf],
) -> Result<(), PluginError>
where
    F: FnOnce() -> Result<(), PluginError>,
{
    let mut preserved = Vec::new();
    let mut originals = old_files.to_vec();
    for (_, destination) in replacements {
        if destination.exists() {
            originals.push(destination.clone());
        }
    }
    originals.sort();
    originals.dedup();
    for original in originals {
        let dir = original
            .parent()
            .ok_or(PluginError::InvalidPayload("plugin destination parent"))?;
        let backup = dir.join(format!(".plugin.backup.{}", uuid::Uuid::new_v4()));
        if let Err(error) = std::fs::rename(&original, &backup) {
            return Err(publication_error(PluginError::Io(error), &preserved));
        }
        preserved.push((original, backup));
    }
    let mut published = Vec::new();
    for (stage, destination) in replacements {
        if let Err(error) = std::fs::rename(stage, destination) {
            return Err(rollback_publication(
                PluginError::Io(error),
                &published,
                &preserved,
            ));
        }
        published.push(destination.clone());
    }
    if let Err(primary) = commit_tracking() {
        return Err(rollback_publication(primary, &published, &preserved));
    }
    for (_, backup) in preserved {
        std::fs::remove_file(&backup).map_err(|source| PluginError::Cleanup {
            path: backup,
            source,
        })?;
    }
    Ok(())
}

fn rollback_publication(
    primary: PluginError,
    published: &[PathBuf],
    preserved: &[(PathBuf, PathBuf)],
) -> PluginError {
    let mut rollback = Vec::new();
    for path in published {
        if let Err(error) = std::fs::remove_file(path) {
            rollback.push(error);
        }
    }
    rollback.extend(restore_preserved(preserved));
    if rollback.is_empty() {
        return PluginError::PublicationFailed {
            primary: Box::new(primary),
        };
    }
    PluginError::PublicationRollbackFailed {
        primary: Box::new(primary),
        rollback,
        backup: preserved.first().map(|(_, backup)| backup.clone()),
    }
}

fn restore_preserved(preserved: &[(PathBuf, PathBuf)]) -> Vec<std::io::Error> {
    let mut failures = Vec::new();
    for (original, backup) in preserved.iter().rev() {
        if let Err(error) = std::fs::rename(backup, original) {
            failures.push(error);
        }
    }
    failures
}

pub(super) fn publication_error(
    primary: PluginError,
    preserved: &[(PathBuf, PathBuf)],
) -> PluginError {
    let rollback = restore_preserved(preserved);
    if rollback.is_empty() {
        return PluginError::PublicationFailed {
            primary: Box::new(primary),
        };
    }
    PluginError::PublicationRollbackFailed {
        primary: Box::new(primary),
        rollback,
        backup: preserved.first().map(|(_, backup)| backup.clone()),
    }
}
