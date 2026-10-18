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
                errors.push(super::super::error::file_io(
                    "restore file",
                    &root.join(rel),
                    e,
                ));
            }
        }
    }
    for rel in journal.disabled.iter().rev() {
        if let Err(error) = restore_disabled(root, backup, rel) {
            errors.push(error);
        }
    }
    for (rel, stored) in journal.moved.iter().rev() {
        let src = checked_path(root, &backup.join(stored))?;
        let dest = rooted_path(root, rel)?;
        if src.exists() {
            if let Err(error) = remove_path(&dest) {
                errors.push(super::super::error::file_io(
                    "remove restoration destination",
                    &root.join(rel),
                    error,
                ));
                continue;
            }
            if let Err(e) = copy_path(&src, &dest) {
                errors.push(super::super::error::file_io(
                    "restore file",
                    &root.join(rel),
                    e,
                ));
            }
        } else if !dest.exists() {
            errors.push(VersionChangeError::MissingRecoveryFile(src));
        }
    }
    for (dir, list) in [("configs", &journal.configs), ("worlds", &journal.worlds)] {
        for rel in list {
            let src = checked_path(root, &backup.join(dir).join(rel))?;
            let dest = rooted_path(root, rel)?;
            if src.exists() {
                if let Err(error) = remove_path(&dest) {
                    errors.push(super::super::error::file_io(
                        "remove restoration destination",
                        &root.join(rel),
                        error,
                    ));
                    continue;
                }
                if let Err(e) = copy_path(&src, &dest) {
                    errors.push(super::super::error::file_io(
                        "restore file",
                        &root.join(rel),
                        e,
                    ));
                }
            } else if !dest.exists() {
                errors.push(VersionChangeError::MissingRecoveryFile(src));
            }
        }
    }
    for rel in &journal.absent_configs {
        if let Err(error) = remove_path(&rooted_path(root, rel)?) {
            errors.push(super::super::error::file_io(
                "remove restoration destination",
                &root.join(rel),
                error,
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(VersionChangeError::RestoreFailures(
            super::super::error::RestorationFailures(errors),
        ))
    }
}

fn restore_disabled(root: &Path, backup: &Path, relative: &str) -> Result<(), VersionChangeError> {
    let disabled = rooted_path(root, format!("{relative}.disabled"))?;
    let target = rooted_path(root, relative)?;
    let file_error =
        |source| super::super::error::file_io("restore disabled file", &target, source);
    let snapshot = checked_path(root, &backup.join("restored-disabled").join(relative))?;
    if !snapshot.try_exists().map_err(file_error)? {
        let source = if disabled.try_exists().map_err(file_error)? {
            &disabled
        } else if target.try_exists().map_err(file_error)? {
            &target
        } else {
            return Err(VersionChangeError::MissingRecoveryFile(target));
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_missing_recovery_sources_are_retained_as_typed_failures() {
        let root = tempfile::tempdir().unwrap();
        let journal = Journal {
            moved: vec![
                ("mods/a.jar".into(), "files/mods/a.jar".into()),
                ("mods/b.jar".into(), "files/mods/b.jar".into()),
            ],
            ..Default::default()
        };
        let error = restore(root.path(), &root.path().join("backup"), &journal).unwrap_err();
        let VersionChangeError::RestoreFailures(failures) = error else {
            panic!("expected aggregate failure")
        };
        assert_eq!(failures.0.len(), 2);
        assert!(failures
            .0
            .iter()
            .all(|failure| matches!(failure, VersionChangeError::MissingRecoveryFile(_))));
    }
}
