use super::models::{
    backup_dir, dir_size, TransactionPhase, VersionBackup, VersionChangeError,
    VersionTransactionStatus, JOURNAL,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Everything `apply` changed, in order, so it can be put back
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Journal {
    pub created_at: u64,
    pub from_game_version: String,
    pub from_loader: String,
    pub from_loader_version: Option<String>,
    pub to_game_version: String,
    pub to_loader: String,
    #[serde(default)]
    pub phase: TransactionPhase,
    /// Files moved into the backup: (path in the target, path in the backup)
    pub moved: Vec<(String, String)>,
    /// Files renamed to <name>.disabled
    pub disabled: Vec<String>,
    /// Files put in place
    pub added: Vec<String>,
    /// Configs copied into the backup
    pub configs: Vec<String>,
    pub worlds: Vec<String>,
    /// Set after the first successful start; until then a crash is reported
    pub first_start_checked: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum JournalState {
    Missing,
    Present(Box<Journal>),
    Corrupt(String),
}

pub fn read_journal_state(root: &Path) -> JournalState {
    let path = backup_dir(root).join(JOURNAL);
    if !path.exists() {
        return JournalState::Missing;
    }
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(e) => return JournalState::Corrupt(format!("Unreadable journal at {}: {e}", path.display())),
    };
    match serde_json::from_str::<Journal>(&raw) {
        Ok(journal) => JournalState::Present(Box::new(journal)),
        Err(e) => JournalState::Corrupt(format!("Invalid journal format at {}: {e}", path.display())),
    }
}

pub fn read_journal(root: &Path) -> Option<Journal> {
    match read_journal_state(root) {
        JournalState::Present(j) => Some(*j),
        _ => None,
    }
}

pub fn detect_incomplete_transaction(root: &Path) -> Result<Option<Journal>, VersionChangeError> {
    match read_journal_state(root) {
        JournalState::Missing => Ok(None),
        JournalState::Present(journal) => {
            if matches!(
                journal.phase,
                TransactionPhase::Swapping | TransactionPhase::RollbackFailed
            ) {
                Ok(Some(*journal))
            } else {
                Ok(None)
            }
        }
        JournalState::Corrupt(err) => Err(VersionChangeError::RestoreError(format!(
            "Corrupt or unreadable recovery journal found: {err}. Manual recovery or inspection required before proceeding."
        ))),
    }
}

pub fn write_journal_typed(root: &Path, journal: &Journal) -> Result<(), VersionChangeError> {
    use std::io::Write;
    let backup = backup_dir(root);
    fs::create_dir_all(&backup)
        .map_err(|e| VersionChangeError::BackupError(format!("Failed to create backup directory: {e}")))?;
    let raw = serde_json::to_string_pretty(journal)
        .map_err(|e| VersionChangeError::Custom(e.to_string()))?;

    let mut staged = tempfile::NamedTempFile::new_in(&backup)
        .map_err(|e| VersionChangeError::BackupError(format!("Failed to stage journal: {e}")))?;
    staged
        .write_all(raw.as_bytes())
        .map_err(|e| VersionChangeError::BackupError(format!("Failed to write staged journal: {e}")))?;
    staged
        .as_file()
        .sync_all()
        .map_err(|e| VersionChangeError::BackupError(format!("Failed to sync staged journal: {e}")))?;
    staged
        .persist(backup.join(JOURNAL))
        .map_err(|e| VersionChangeError::BackupError(format!("Failed to publish journal: {}", e.error)))?;
    Ok(())
}

pub fn write_journal(root: &Path, journal: &Journal) -> Result<(), String> {
    write_journal_typed(root, journal).map_err(|e| e.to_string())
}

/// The last version change, if it can be undone
pub fn last_backup(root: &Path) -> Option<VersionBackup> {
    let journal = read_journal(root)?;
    if journal.phase == TransactionPhase::RollbackFailed {
        return None;
    }
    Some(VersionBackup {
        from_game_version: journal.from_game_version,
        from_loader: journal.from_loader,
        to_game_version: journal.to_game_version,
        to_loader: journal.to_loader,
        created_at: journal.created_at,
        worlds_backed_up: !journal.worlds.is_empty(),
        bytes: dir_size(&backup_dir(root)) as f64,
    })
}

/// Deletes the backup of the last change (frees the space; it can't be undone after)
pub fn discard_backup_typed(root: &Path) -> Result<(), VersionChangeError> {
    let backup = backup_dir(root);
    if backup.exists() {
        fs::remove_dir_all(&backup)
            .map_err(|e| VersionChangeError::FileOp(format!("Failed to delete the backup: {e}")))?;
    }
    Ok(())
}

pub fn discard_backup(root: &Path) -> Result<(), String> {
    discard_backup_typed(root).map_err(|e| e.to_string())
}

/// Reports the current transaction and recovery status for a target root.
pub fn transaction_status(root: &Path) -> VersionTransactionStatus {
    match read_journal_state(root) {
        JournalState::Missing => VersionTransactionStatus {
            phase: None,
            is_incomplete: false,
            can_recover: false,
            backup: None,
            error: None,
        },
        JournalState::Present(j) => {
            let is_incomplete = matches!(
                j.phase,
                TransactionPhase::Swapping | TransactionPhase::RollbackFailed
            );
            let can_recover = is_incomplete;
            let backup = Some(VersionBackup {
                from_game_version: j.from_game_version.clone(),
                from_loader: j.from_loader.clone(),
                to_game_version: j.to_game_version.clone(),
                to_loader: j.to_loader.clone(),
                created_at: j.created_at,
                worlds_backed_up: !j.worlds.is_empty(),
                bytes: dir_size(&backup_dir(root)) as f64,
            });
            VersionTransactionStatus {
                phase: Some(j.phase),
                is_incomplete,
                can_recover,
                backup,
                error: if j.phase == TransactionPhase::RollbackFailed {
                    Some("A previous version change rollback failed. Recovery is required to restore files and metadata.".into())
                } else if j.phase == TransactionPhase::Swapping {
                    Some("An interrupted version change was detected. Recovery is required before launching or updating.".into())
                } else {
                    None
                },
            }
        }
        JournalState::Corrupt(err) => VersionTransactionStatus {
            phase: None,
            is_incomplete: true,
            can_recover: false,
            backup: None,
            error: Some(format!("Recovery journal is corrupt: {err}")),
        },
    }
}
