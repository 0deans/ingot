use super::super::{
    journal::{read_journal_state, write_journal_typed, Journal, JournalState},
    models::{backup_dir, loader_label, Restored, TransactionPhase, VersionChangeError},
};
use super::{paths::remove_path, restoration_files::restore};
use std::path::Path;

/// Restores everything as it was before the last change, atomically updating metadata.
pub fn undo_typed_with_commit<F, E>(
    root: &Path,
    game_version: &str,
    loader: &str,
    commit_metadata: F,
) -> Result<Restored, VersionChangeError>
where
    F: FnOnce(&Restored) -> Result<(), E>,
    E: std::fmt::Display,
{
    let journal = match read_journal_state(root) {
        JournalState::Present(j) => *j,
        JournalState::Missing => {
            return Err(VersionChangeError::UndoError(
                "There's no version change to undo".into(),
            ));
        }
        JournalState::Corrupt(err) => {
            return Err(VersionChangeError::UndoError(format!(
                "Cannot undo: recovery journal is corrupt: {err}"
            )));
        }
    };
    if journal.phase == TransactionPhase::RollbackFailed {
        return Err(VersionChangeError::UndoError(
            "The previous transaction suffered a failed rollback and cannot be cleanly undone. Check the recovery backup.".into(),
        ));
    }
    let matches_target = game_version == journal.to_game_version && loader == journal.to_loader;
    let retrying_restoration = matches!(
        journal.phase,
        TransactionPhase::Restoring | TransactionPhase::Restored
    ) && game_version == journal.from_game_version
        && loader == journal.from_loader;
    if !matches_target && !retrying_restoration {
        return Err(VersionChangeError::UndoError(format!(
            "It's on {} {game_version} now, not {} {}, so this backup no longer matches",
            loader_label(loader),
            loader_label(&journal.to_loader),
            journal.to_game_version
        )));
    }
    finish_restoration(root, journal, commit_metadata, |backup| {
        cleanup_backup(root, backup)
    })
}

/// Restores everything as it was before the last change. `game_version` and `loader` are
/// the current ones: the backup only applies to the version it changed to.
pub fn undo_typed(
    root: &Path,
    game_version: &str,
    loader: &str,
) -> Result<Restored, VersionChangeError> {
    undo_typed_with_commit(root, game_version, loader, |_| {
        Ok::<(), VersionChangeError>(())
    })
}

pub fn undo_with_commit<F, E>(
    root: &Path,
    game_version: &str,
    loader: &str,
    commit_metadata: F,
) -> Result<Restored, VersionChangeError>
where
    F: FnOnce(&Restored) -> Result<(), E>,
    E: std::fmt::Display,
{
    undo_typed_with_commit(root, game_version, loader, commit_metadata)
}

pub fn undo(root: &Path, game_version: &str, loader: &str) -> Result<Restored, VersionChangeError> {
    undo_typed(root, game_version, loader)
}

/// Recovers an interrupted version change transaction left in Swapping or RollbackFailed state,
/// restoring files and updating configuration metadata.
pub fn recover_interrupted_transaction_with_commit<F, E>(
    root: &Path,
    commit_metadata: F,
) -> Result<Restored, VersionChangeError>
where
    F: FnOnce(&Restored) -> Result<(), E>,
    E: std::fmt::Display,
{
    let journal = match read_journal_state(root) {
        JournalState::Present(j) => *j,
        JournalState::Missing => {
            return Err(VersionChangeError::RestoreError(
                "No recovery journal found".into(),
            ));
        }
        JournalState::Corrupt(err) => {
            return Err(VersionChangeError::RestoreError(format!(
                "Cannot recover: recovery journal is corrupt: {err}"
            )));
        }
    };
    if matches!(
        journal.phase,
        TransactionPhase::Committed | TransactionPhase::Prepared
    ) {
        return Err(VersionChangeError::RestoreError(
            "Transaction was already committed. Use undo instead.".into(),
        ));
    }
    finish_restoration(root, journal, commit_metadata, |backup| {
        cleanup_backup(root, backup)
    })
}

/// Recovers an interrupted version change transaction left in Swapping or RollbackFailed state.
pub fn recover_interrupted_transaction(root: &Path) -> Result<(), VersionChangeError> {
    recover_interrupted_transaction_with_commit(root, |_| Ok::<(), VersionChangeError>(()))
        .map(|_| ())
}

fn finish_restoration<F, E>(
    root: &Path,
    mut journal: Journal,
    commit_metadata: F,
    cleanup: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<Restored, VersionChangeError>
where
    F: FnOnce(&Restored) -> Result<(), E>,
    E: std::fmt::Display,
{
    let backup = backup_dir(root);
    let restored = Restored {
        game_version: journal.from_game_version.clone(),
        loader: journal.from_loader.clone(),
        loader_version: journal.from_loader_version.clone(),
    };
    if journal.phase != TransactionPhase::Restored {
        journal.phase = TransactionPhase::Restoring;
        write_journal_typed(root, &journal)?;
        if let Err(primary) = restore(root, &backup, &journal) {
            journal.phase = TransactionPhase::RollbackFailed;
            let diagnostic = match write_journal_typed(root, &journal) {
                Ok(()) => primary.to_string(),
                Err(checkpoint) => {
                    format!("{primary}; failure checkpoint also failed: {checkpoint}")
                }
            };
            return Err(VersionChangeError::RollbackFailed {
                primary: "Version restoration failed".into(),
                restore: diagnostic,
                backup_path: backup,
            });
        }
        commit_metadata(&restored).map_err(|error| VersionChangeError::MetadataAfterRestore {
            message: error.to_string(),
            backup_path: backup.clone(),
        })?;
        journal.phase = TransactionPhase::Restored;
        write_journal_typed(root, &journal).map_err(|source| {
            VersionChangeError::CommitJournal {
                backup_path: backup.clone(),
                source: Box::new(source),
            }
        })?;
    }
    cleanup(&backup).map_err(|source| VersionChangeError::Finalization {
        operation: "Restoration cleanup",
        backup_path: backup,
        source,
    })?;
    Ok(restored)
}

fn cleanup_backup(root: &Path, backup: &Path) -> std::io::Result<()> {
    use super::super::models::JOURNAL;
    let journal = match read_journal_state(root) {
        JournalState::Present(journal) => journal,
        _ => {
            return Err(std::io::Error::other(
                "Restoration cleanup checkpoint is unavailable",
            ))
        }
    };
    for entry in std::fs::read_dir(backup)? {
        let entry = entry?;
        if entry.file_name() != JOURNAL {
            remove_path(&entry.path())?;
        }
    }
    std::fs::remove_file(backup.join(JOURNAL))?;
    if let Err(primary) = std::fs::remove_dir(backup) {
        // An empty directory can fail deletion after its contents were acknowledged.
        // Restore the checkpoint so cleanup remains explicitly retryable.
        if let Err(checkpoint) = write_journal_typed(root, &journal) {
            return Err(std::io::Error::other(format!(
                "{primary}; cleanup checkpoint failed: {checkpoint}"
            )));
        }
        return Err(primary);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn disabled_fixture(root: &Path) -> Journal {
        let backup = backup_dir(root);
        fs::create_dir_all(root.join("mods")).unwrap();
        fs::create_dir_all(backup.join("files/mods")).unwrap();
        fs::write(root.join("mods/a.jar.disabled"), b"original").unwrap();
        fs::write(
            backup.join("files/mods/a.jar.disabled"),
            b"previous disabled",
        )
        .unwrap();
        let journal = Journal {
            from_game_version: "1".into(),
            to_game_version: "2".into(),
            from_loader: "fabric".into(),
            to_loader: "fabric".into(),
            disabled: vec!["mods/a.jar".into()],
            moved: vec![(
                "mods/a.jar.disabled".into(),
                "files/mods/a.jar.disabled".into(),
            )],
            ..Default::default()
        };
        write_journal_typed(root, &journal).unwrap();
        journal
    }

    fn assert_originals(root: &Path) {
        assert_eq!(fs::read(root.join("mods/a.jar")).unwrap(), b"original");
        assert_eq!(
            fs::read(root.join("mods/a.jar.disabled")).unwrap(),
            b"previous disabled"
        );
    }

    #[test]
    fn repeated_restore_preserves_original_and_previous_disabled_file() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let journal = disabled_fixture(root);
        for _ in 0..2 {
            restore(root, &backup_dir(root), &journal).unwrap();
            assert_originals(root);
        }
    }

    #[test]
    fn failed_metadata_commit_is_recoverable_without_replacing_original() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let journal = disabled_fixture(root);
        let result = finish_restoration(
            root,
            journal,
            |_| Err("injected metadata failure"),
            |_| Ok(()),
        );
        assert!(matches!(
            result,
            Err(VersionChangeError::MetadataAfterRestore { .. })
        ));
        assert!(super::super::super::journal::transaction_status(root).can_recover);
        recover_interrupted_transaction_with_commit(root, |_| Ok::<(), String>(())).unwrap();
        assert_originals(root);
        assert!(!backup_dir(root).exists());
    }

    #[test]
    fn failed_cleanup_keeps_a_retryable_checkpoint_after_metadata_commit() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let journal = disabled_fixture(root);
        let result = finish_restoration(
            root,
            journal,
            |_| Ok::<(), String>(()),
            |_| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "injected cleanup failure",
                ))
            },
        );
        assert!(matches!(
            result,
            Err(VersionChangeError::Finalization { .. })
        ));
        assert_originals(root);
        assert!(super::super::super::journal::transaction_status(root).can_recover);
        undo_typed(root, "1", "fabric").unwrap();
        assert_originals(root);
        assert!(!backup_dir(root).exists());
    }
}
