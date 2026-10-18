use super::models::{
    backup_dir, dir_size_typed, TransactionPhase, VersionBackup, VersionChangeError,
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
    #[serde(default)]
    pub absent_configs: Vec<String>,
    pub worlds: Vec<String>,
    /// Set after the first successful start; until then a crash is reported
    pub first_start_checked: bool,
}

#[derive(Debug)]
pub enum JournalState {
    Missing,
    Present(Box<Journal>),
    Corrupt(Box<VersionChangeError>),
}

pub fn read_journal_state(root: &Path) -> JournalState {
    let path = backup_dir(root).join(JOURNAL);
    if let Err(error) = super::transaction::checked_path(root, &path) {
        return JournalState::Corrupt(Box::new(error));
    }
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return JournalState::Missing,
        Err(e) => return JournalState::Corrupt(Box::new(journal_io("read", &path, e))),
    };
    match serde_json::from_str::<Journal>(&raw) {
        Ok(journal) => match validate_journal_paths(&journal) {
            Ok(()) => JournalState::Present(Box::new(journal)),
            Err(error) => JournalState::Corrupt(Box::new(error)),
        },
        Err(e) => JournalState::Corrupt(Box::new(VersionChangeError::JournalDecode(e))),
    }
}

pub(super) fn validate_journal_paths(journal: &Journal) -> Result<(), VersionChangeError> {
    let paths = journal
        .added
        .iter()
        .chain(&journal.disabled)
        .chain(&journal.configs)
        .chain(&journal.absent_configs)
        .chain(&journal.worlds);
    for path in paths {
        super::models::safe_rel_typed(path)?;
    }
    for (original, stored) in &journal.moved {
        super::models::safe_rel_typed(original)?;
        super::models::safe_rel_typed(stored)?;
        if !stored.starts_with("files/") {
            return Err(VersionChangeError::InvalidPath(stored.clone()));
        }
    }
    Ok(())
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
                TransactionPhase::Swapping
                    | TransactionPhase::RollbackFailed
                    | TransactionPhase::Restoring
                    | TransactionPhase::Restored
            ) {
                Ok(Some(*journal))
            } else {
                Ok(None)
            }
        }
        JournalState::Corrupt(err) => Err(*err),
    }
}

fn journal_io(operation: &'static str, path: &Path, source: std::io::Error) -> VersionChangeError {
    VersionChangeError::JournalIo {
        operation,
        path: path.to_owned(),
        source,
    }
}

pub fn write_journal_typed(root: &Path, journal: &Journal) -> Result<(), VersionChangeError> {
    use std::io::Write;
    let backup = backup_dir(root);
    let path = backup.join(JOURNAL);
    super::transaction::checked_path(root, &path)?;
    fs::create_dir_all(&backup)
        .map_err(|source| journal_io("create directory for", &path, source))?;
    let raw = serde_json::to_vec_pretty(journal).map_err(VersionChangeError::JournalEncode)?;
    let mut staged = tempfile::NamedTempFile::new_in(&backup)
        .map_err(|source| journal_io("stage", &path, source))?;
    staged
        .write_all(&raw)
        .map_err(|source| journal_io("write", &path, source))?;
    staged
        .as_file()
        .sync_all()
        .map_err(|source| journal_io("sync", &path, source))?;
    staged
        .persist(&path)
        .map_err(|error| journal_io("publish", &path, error.error))?;
    Ok(())
}

pub fn write_journal(root: &Path, journal: &Journal) -> Result<(), VersionChangeError> {
    write_journal_typed(root, journal)
}

/// The last version change, if it can be undone
pub fn last_backup(root: &Path) -> Result<Option<VersionBackup>, VersionChangeError> {
    let journal = match read_journal_state(root) {
        JournalState::Missing => return Ok(None),
        JournalState::Present(journal) => journal,
        JournalState::Corrupt(error) => return Err(*error),
    };
    if !matches!(
        journal.phase,
        TransactionPhase::Committed | TransactionPhase::Prepared
    ) {
        return Ok(None);
    }
    backup_summary(root, &journal).map(Some)
}

fn backup_summary(root: &Path, journal: &Journal) -> Result<VersionBackup, VersionChangeError> {
    backup_summary_with(root, journal, dir_size_typed)
}

fn backup_summary_with(
    root: &Path,
    journal: &Journal,
    measure: impl FnOnce(&Path) -> std::io::Result<u64>,
) -> Result<VersionBackup, VersionChangeError> {
    let path = backup_dir(root);
    let bytes = measure(&path).map_err(|source| journal_io("measure backup for", &path, source))?;
    Ok(VersionBackup {
        from_game_version: journal.from_game_version.clone(),
        from_loader: journal.from_loader.clone(),
        to_game_version: journal.to_game_version.clone(),
        to_loader: journal.to_loader.clone(),
        created_at: journal.created_at,
        worlds_backed_up: !journal.worlds.is_empty(),
        bytes: bytes as f64,
    })
}

/// Deletes the backup of the last change (frees the space; it can't be undone after)
pub fn discard_backup_typed(root: &Path) -> Result<(), VersionChangeError> {
    let backup = backup_dir(root);
    if backup.exists() {
        fs::remove_dir_all(&backup)
            .map_err(|source| super::error::file_io("delete backup", &backup, source))?;
    }
    Ok(())
}

pub fn discard_backup(root: &Path) -> Result<(), VersionChangeError> {
    discard_backup_typed(root)
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
                TransactionPhase::Swapping
                    | TransactionPhase::RollbackFailed
                    | TransactionPhase::Restoring
                    | TransactionPhase::Restored
            );
            let can_recover = is_incomplete;
            let backup = match backup_summary(root, &j) {
                Ok(backup) => Some(backup),
                Err(error) => {
                    return VersionTransactionStatus {
                        phase: Some(j.phase),
                        is_incomplete: true,
                        can_recover: false,
                        backup: None,
                        error: Some(error.to_string()),
                    }
                }
            };
            VersionTransactionStatus {
                phase: Some(j.phase),
                is_incomplete,
                can_recover,
                backup,
                error: if j.phase == TransactionPhase::RollbackFailed {
                    Some("A previous version change rollback failed. Recovery is required to restore files and metadata.".into())
                } else if j.phase == TransactionPhase::Swapping {
                    Some("An interrupted version change was detected. Recovery is required before launching or updating.".into())
                } else if matches!(
                    j.phase,
                    TransactionPhase::Restoring | TransactionPhase::Restored
                ) {
                    Some("Version restoration needs finalization. Retry recovery before launching or updating.".into())
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

#[cfg(test)]
mod tests;
