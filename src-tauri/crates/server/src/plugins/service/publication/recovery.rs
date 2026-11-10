use super::super::super::{
    models::PluginError,
    tracking::{read_tracking_raw, write_tracking_typed},
};
use super::journal::{validate_journal, PluginJournal, PluginJournalPhase};
use super::{journal_path, sync_parent_dir};
use std::{fs, io, path::Path};

pub(super) fn recover_plugins_locked(server_dir: &Path) -> Result<(), PluginError> {
    let path = journal_path(server_dir);
    match fs::symlink_metadata(&path) {
        Ok(m) if !m.is_file() || m.file_type().is_symlink() => {
            return Err(PluginError::RecoveryBlocked(
                "Plugin journal is not a plain file",
            ));
        }
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(PluginError::Io(e)),
    }
    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(PluginError::Io(e)),
    };
    let journal: PluginJournal = serde_json::from_slice(&bytes).map_err(PluginError::Json)?;
    validate_journal(server_dir, &journal)?;
    let current_tracking = read_tracking_raw(server_dir)?;
    let is_committed = journal.phase == PluginJournalPhase::Committed;

    if is_committed {
        if current_tracking != journal.target_tracking {
            return Err(PluginError::RecoveryBlocked(
                "Committed plugin tracking changed",
            ));
        }
        for b in &journal.backups {
            remove_plain_file(&server_dir.join(&b.backup))?;
        }
        fs::remove_file(&path).map_err(PluginError::Io)?;
        sync_parent_dir(&path)?;
        Ok(())
    } else {
        for replacement in &journal.replacements {
            let original_existed = journal
                .backups
                .iter()
                .any(|backup| backup.original == replacement.target);
            if !original_existed {
                remove_plain_file(&server_dir.join(&replacement.target))?;
            }
            if !replacement.stage.is_empty() {
                remove_plain_file(&server_dir.join(&replacement.stage))?;
            }
        }
        for backup in &journal.backups {
            let original = server_dir.join(&backup.original);
            let preserved = server_dir.join(&backup.backup);
            if plain_file(&preserved)? {
                let parent = original
                    .parent()
                    .ok_or(PluginError::InvalidPayload("plugin parent"))?;
                let mut staging = tempfile::NamedTempFile::new_in(parent)?;
                io::copy(&mut fs::File::open(&preserved)?, &mut staging)?;
                staging.as_file().sync_all()?;
                staging
                    .persist(&original)
                    .map_err(|error| PluginError::Io(error.error))?;
                sync_parent_dir(&original)?;
            } else if !plain_file(&original)? {
                return Err(PluginError::RecoveryBlocked(
                    "Missing plugin original and backup",
                ));
            }
        }
        if current_tracking != journal.previous_tracking {
            write_tracking_typed(server_dir, &journal.previous_tracking)
                .map_err(PluginError::Io)?;
        }
        for backup in &journal.backups {
            remove_plain_file(&server_dir.join(&backup.backup))?;
        }
        fs::remove_file(&path).map_err(PluginError::Io)?;
        sync_parent_dir(&path)?;
        Ok(())
    }
}

pub(super) fn plain_file(path: &Path) -> Result<bool, PluginError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => Ok(true),
        Ok(_) => Err(PluginError::RecoveryBlocked(
            "Plugin transaction path is not a plain file",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn remove_plain_file(path: &Path) -> Result<(), PluginError> {
    if plain_file(path)? {
        fs::remove_file(path)?;
        sync_parent_dir(path)?;
    }
    Ok(())
}
