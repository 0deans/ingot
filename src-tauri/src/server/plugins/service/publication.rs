mod journal;
mod recovery;

use journal::{
    validate_journal, write_journal, PluginFileBackup, PluginFileReplacement, PluginJournal,
    PluginJournalPhase,
};
use recovery::{plain_file, recover_plugins_locked};

use super::super::{
    models::{PluginError, TrackedPlugin},
    tracking::{read_tracking_raw, write_tracking_typed},
};
use crate::server::config::ServerCoreType;
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock, Weak},
};

pub(crate) fn journal_path(server_dir: &Path) -> PathBuf {
    server_dir.join(".ingot").join("plugins.journal.json")
}

fn server_lock(server_dir: &Path) -> Result<Arc<Mutex<()>>, PluginError> {
    static REGISTRY: OnceLock<Mutex<HashMap<PathBuf, Weak<Mutex<()>>>>> = OnceLock::new();
    let registry = REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    let canonical = server_dir.canonicalize()?;
    let mut map = registry
        .lock()
        .map_err(|_| PluginError::RecoveryBlocked("Plugin lock poisoned"))?;
    map.retain(|_, weak| weak.strong_count() > 0);
    if let Some(existing) = map.get(&canonical).and_then(Weak::upgrade) {
        Ok(existing)
    } else {
        let lock = Arc::new(Mutex::new(()));
        map.insert(canonical, Arc::downgrade(&lock));
        Ok(lock)
    }
}

pub(crate) fn with_plugin_lock<R>(
    server_dir: &Path,
    action: impl FnOnce() -> Result<R, PluginError>,
) -> Result<R, PluginError> {
    let lock = server_lock(server_dir)?;
    let _guard = lock
        .lock()
        .map_err(|_| PluginError::RecoveryBlocked("Plugin lock poisoned"))?;
    for relative in [".ingot/plugins.journal.json", ".ingot/plugins.json"] {
        if crate::app::safe_enclosed_path(server_dir, relative).is_none() {
            return Err(PluginError::RecoveryBlocked("Unsafe plugin metadata path"));
        }
    }
    recover_plugins_locked(server_dir)?;
    action()
}

pub fn recover_plugins(server_dir: &Path) -> Result<(), PluginError> {
    with_plugin_lock(server_dir, || Ok(()))
}

pub fn publish_plugin_transaction(
    server_dir: &Path,
    _core: &ServerCoreType,
    replacements: &[(PathBuf, PathBuf)],
    old_files: &[PathBuf],
    target_tracking: &[TrackedPlugin],
) -> Result<(), PluginError> {
    with_plugin_lock(server_dir, || {
        publish_plugin_transaction_locked(
            server_dir,
            _core,
            replacements,
            old_files,
            target_tracking,
        )
    })
}

pub(super) fn publish_plugin_transaction_locked(
    server_dir: &Path,
    _core: &ServerCoreType,
    replacements: &[(PathBuf, PathBuf)],
    old_files: &[PathBuf],
    target_tracking: &[TrackedPlugin],
) -> Result<(), PluginError> {
    let previous_tracking = read_tracking_raw(server_dir)?;

    let mut actual_replacements = Vec::new();
    for (stage, destination) in replacements {
        to_relative(server_dir, destination)?;
        let parent = destination
            .parent()
            .ok_or(PluginError::InvalidPayload("destination parent"))?;
        let local_stage = parent.join(format!(".plugin.stage.{}", uuid::Uuid::new_v4()));
        fs::copy(stage, &local_stage)?;
        fs::OpenOptions::new()
            .write(true)
            .open(&local_stage)?
            .sync_all()?;
        sync_parent_dir(&local_stage)?;
        actual_replacements.push((local_stage, destination.clone()));
    }

    let mut originals = old_files.to_vec();
    for (_, destination) in &actual_replacements {
        if plain_file(destination)? {
            originals.push(destination.clone());
        }
    }
    originals.sort();
    originals.dedup();

    let mut backups_rel = Vec::new();
    let mut backups_paths = Vec::new();
    for original in &originals {
        let dir = original
            .parent()
            .ok_or(PluginError::InvalidPayload("plugin destination parent"))?;
        let backup_name = format!(".plugin.backup.{}", uuid::Uuid::new_v4());
        let backup = dir.join(&backup_name);
        let orig_rel = to_relative(server_dir, original)?;
        let backup_rel = to_relative(server_dir, &backup)?;
        backups_rel.push(PluginFileBackup {
            original: orig_rel,
            backup: backup_rel,
        });
        backups_paths.push((original.clone(), backup));
    }

    let mut replacements_rel = Vec::new();
    for (stage, target) in &actual_replacements {
        replacements_rel.push(PluginFileReplacement {
            stage: to_relative(server_dir, stage)?,
            target: to_relative(server_dir, target)?,
        });
    }

    let mut journal = PluginJournal {
        version: 2,
        phase: PluginJournalPhase::Prepared,
        replacements: replacements_rel,
        backups: backups_rel,
        previous_tracking,
        target_tracking: target_tracking.to_vec(),
    };

    validate_journal(server_dir, &journal)?;
    write_journal(server_dir, &journal)?;

    for (original, backup) in &backups_paths {
        if let Err(error) = fs::rename(original, backup) {
            return Err(rollback_journal(
                server_dir,
                PluginError::Io(error),
                &journal,
            ));
        }
        sync_parent_dir(backup)?;
    }

    for (stage, destination) in &actual_replacements {
        if let Err(error) = fs::rename(stage, destination) {
            return Err(rollback_journal(
                server_dir,
                PluginError::Io(error),
                &journal,
            ));
        }
        sync_parent_dir(destination)?;
    }

    if let Err(source) = write_tracking_typed(server_dir, target_tracking) {
        return Err(rollback_journal(
            server_dir,
            PluginError::Io(source),
            &journal,
        ));
    }

    journal.phase = PluginJournalPhase::Committed;
    write_journal(server_dir, &journal)?;

    recover_plugins_locked(server_dir)
}

fn rollback_journal(
    server_dir: &Path,
    primary: PluginError,
    journal: &PluginJournal,
) -> PluginError {
    match recover_plugins_locked(server_dir) {
        Ok(()) => PluginError::PublicationFailed {
            primary: Box::new(primary),
        },
        Err(err) => {
            let backup_path = journal.backups.first().map(|b| server_dir.join(&b.backup));
            PluginError::PublicationRollbackFailed {
                primary: Box::new(primary),
                rollback: vec![io::Error::other(err)],
                backup: backup_path,
            }
        }
    }
}

fn to_relative(server_dir: &Path, path: &Path) -> Result<String, PluginError> {
    let rel = path
        .strip_prefix(server_dir)
        .map_err(|_| PluginError::InvalidPayload("path outside server directory"))?;
    let rel_str = rel.to_string_lossy().replace('\\', "/");
    if crate::app::safe_enclosed_path(server_dir, &rel_str).is_none() {
        return Err(PluginError::InvalidPayload("unsafe relative path"));
    }
    Ok(rel_str)
}

fn sync_parent_dir(path: &Path) -> Result<(), PluginError> {
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(PluginError::Io)?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(test)]
pub(super) mod tests;
