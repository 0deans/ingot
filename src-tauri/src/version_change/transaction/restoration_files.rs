use super::super::{journal::Journal, models::VersionChangeError};
use super::paths::{checked_path, copy_path, remove_path, rooted_path};
use std::{fs, path::Path};

/// Puts back everything the journal lists, newest change first.
/// Idempotent and non-destructive to backup sources so undo and recovery can be retried.
pub fn restore(root: &Path, backup: &Path, journal: &Journal) -> Result<(), VersionChangeError> {
    super::super::journal::validate_journal_paths(journal)?;
    checked_path(root, &backup.join("journal.json"))?;
    let mut errors = Vec::new();
    for rel in journal.added.iter().rev() {
        let target = rooted_path(root, rel)?;
        if target.exists() {
            if let Err(e) = remove_path(&target) {
                errors.push(format!("{rel}: {e}"));
            }
        }
    }
    for rel in journal.disabled.iter().rev() {
        if let Err(error) = restore_disabled(root, backup, rel) {
            errors.push(format!("{rel}: {error}"));
        }
    }
    for (rel, stored) in journal.moved.iter().rev() {
        let src = checked_path(root, &backup.join(stored))?;
        let dest = rooted_path(root, rel)?;
        if src.exists() {
            if let Err(error) = remove_path(&dest) {
                errors.push(format!("{rel}: {error}"));
                continue;
            }
            if let Err(e) = copy_path(&src, &dest) {
                errors.push(format!("{rel}: {e}"));
            }
        } else if !dest.exists() {
            errors.push(format!(
                "Backup source {stored} is missing and {rel} is absent from root"
            ));
        }
    }
    for (dir, list) in [("configs", &journal.configs), ("worlds", &journal.worlds)] {
        for rel in list {
            let src = checked_path(root, &backup.join(dir).join(rel))?;
            let dest = rooted_path(root, rel)?;
            if src.exists() {
                if let Err(error) = remove_path(&dest) {
                    errors.push(format!("{rel}: {error}"));
                    continue;
                }
                if let Err(e) = copy_path(&src, &dest) {
                    errors.push(format!("{rel}: {e}"));
                }
            } else if !dest.exists() {
                errors.push(format!(
                    "Backup for {dir}/{rel} is missing and {rel} is absent from root"
                ));
            }
        }
    }
    for rel in &journal.absent_configs {
        if let Err(error) = remove_path(&rooted_path(root, rel)?) {
            errors.push(format!("{rel}: {error}"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(VersionChangeError::RestoreError(errors.join("; ")))
    }
}

fn restore_disabled(root: &Path, backup: &Path, relative: &str) -> Result<(), VersionChangeError> {
    let disabled = rooted_path(root, format!("{relative}.disabled"))?;
    let target = rooted_path(root, relative)?;
    let snapshot = checked_path(root, &backup.join("restored-disabled").join(relative))?;
    if !snapshot.try_exists().map_err(file_error)? {
        let source = if disabled.try_exists().map_err(file_error)? {
            &disabled
        } else if target.try_exists().map_err(file_error)? {
            &target
        } else {
            return Err(VersionChangeError::RestoreError(format!(
                "Missing disabled file {relative}"
            )));
        };
        // Keep an immutable original before restoring a previous .disabled file.
        // Otherwise a metadata retry could mistake that older file for the original.
        let directory = snapshot
            .parent()
            .ok_or_else(|| VersionChangeError::InvalidPath(relative.into()))?;
        fs::create_dir_all(directory).map_err(file_error)?;
        let stage = tempfile::tempdir_in(directory).map_err(file_error)?;
        let prepared = stage.path().join("original");
        copy_path(source, &prepared).map_err(file_error)?;
        fs::rename(&prepared, &snapshot).map_err(file_error)?;
    }
    remove_path(&target).map_err(file_error)?;
    copy_path(&snapshot, &target).map_err(file_error)?;
    remove_path(&disabled).map_err(file_error)
}

fn file_error(error: std::io::Error) -> VersionChangeError {
    VersionChangeError::RestoreError(error.to_string())
}
