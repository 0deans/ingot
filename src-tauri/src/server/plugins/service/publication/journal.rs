use super::super::super::{
    metadata::safe_jar_name,
    models::{PluginError, TrackedPlugin},
};
use super::{journal_path, plain_file, sync_parent_dir};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PluginJournal {
    pub(crate) version: u32,
    pub(crate) phase: PluginJournalPhase,
    pub(crate) replacements: Vec<PluginFileReplacement>,
    pub(crate) backups: Vec<PluginFileBackup>,
    pub(crate) previous_tracking: Vec<TrackedPlugin>,
    pub(crate) target_tracking: Vec<TrackedPlugin>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum PluginJournalPhase {
    Prepared,
    Committed,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PluginFileReplacement {
    pub(crate) stage: String,
    pub(crate) target: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PluginFileBackup {
    pub(crate) original: String,
    pub(crate) backup: String,
}

pub(super) fn validate_journal(
    server_dir: &Path,
    journal: &PluginJournal,
) -> Result<(), PluginError> {
    if journal.version != 2 {
        return Err(PluginError::RecoveryBlocked(
            "Invalid plugin journal version",
        ));
    }

    validate_tracking(server_dir, &journal.previous_tracking)?;
    validate_tracking(server_dir, &journal.target_tracking)?;
    for replacement in &journal.replacements {
        let folder = plugin_file(server_dir, &replacement.target)?;
        transaction_file(server_dir, &replacement.stage, folder, ".plugin.stage.")?;
    }
    for backup in &journal.backups {
        let folder = plugin_file(server_dir, &backup.original)?;
        transaction_file(server_dir, &backup.backup, folder, ".plugin.backup.")?;
    }

    let mut destinations = std::collections::HashSet::new();
    let mut stages = std::collections::HashSet::new();
    let mut originals = std::collections::HashSet::new();
    let mut backups = std::collections::HashSet::new();
    for replacement in &journal.replacements {
        if !destinations.insert(&replacement.target)
            || (!replacement.stage.is_empty() && !stages.insert(&replacement.stage))
        {
            return Err(PluginError::RecoveryBlocked(
                "Overlapping plugin transaction paths",
            ));
        }
    }
    for backup in &journal.backups {
        if !originals.insert(&backup.original) || !backups.insert(&backup.backup) {
            return Err(PluginError::RecoveryBlocked(
                "Overlapping plugin transaction paths",
            ));
        }
    }
    if destinations
        .iter()
        .any(|path| stages.contains(path) || backups.contains(path))
        || originals
            .iter()
            .any(|path| stages.contains(path) || backups.contains(path))
        || stages.iter().any(|path| backups.contains(path))
    {
        return Err(PluginError::RecoveryBlocked(
            "Overlapping plugin transaction paths",
        ));
    }
    for replacement in &journal.replacements {
        let target_exists = plain_file(&server_dir.join(&replacement.target))?;
        if journal.phase == PluginJournalPhase::Committed && !target_exists {
            return Err(PluginError::RecoveryBlocked(
                "Committed plugin file is missing",
            ));
        }
        if !replacement.stage.is_empty() {
            plain_file(&server_dir.join(&replacement.stage))?;
        }
    }
    for backup in &journal.backups {
        let original_exists = plain_file(&server_dir.join(&backup.original))?;
        let backup_exists = plain_file(&server_dir.join(&backup.backup))?;
        if journal.phase == PluginJournalPhase::Prepared && !original_exists && !backup_exists {
            return Err(PluginError::RecoveryBlocked(
                "Missing plugin original and backup",
            ));
        }
    }
    Ok(())
}

fn validate_tracking(server: &Path, items: &[TrackedPlugin]) -> Result<(), PluginError> {
    let mut names = std::collections::HashSet::new();
    for item in items {
        if safe_jar_name(&item.file_name).is_err()
            || crate::app::safe_enclosed_filename(server, &item.file_name).is_none()
            || item.project_id.trim().is_empty()
            || item.version_id.trim().is_empty()
            || !names.insert(&item.file_name)
        {
            return Err(PluginError::RecoveryBlocked(
                "Invalid plugin journal tracking",
            ));
        }
    }
    Ok(())
}

fn plugin_file<'a>(server: &Path, relative: &'a str) -> Result<&'a str, PluginError> {
    let (folder, name) = split_path(server, relative)?;
    safe_jar_name(name).map_err(|_| PluginError::RecoveryBlocked("Unsafe plugin recovery path"))?;
    Ok(folder)
}

fn split_path<'a>(server: &Path, relative: &'a str) -> Result<(&'a str, &'a str), PluginError> {
    let (folder, name) = relative
        .split_once('/')
        .ok_or(PluginError::RecoveryBlocked("Unsafe plugin recovery path"))?;
    if !matches!(folder, "plugins" | "mods")
        || name.contains(['/', '\\'])
        || crate::app::safe_enclosed_path(server, relative).is_none()
    {
        return Err(PluginError::RecoveryBlocked("Unsafe plugin recovery path"));
    }
    Ok((folder, name))
}

fn transaction_file(
    server: &Path,
    relative: &str,
    expected_folder: &str,
    prefix: &str,
) -> Result<(), PluginError> {
    let (folder, name) = split_path(server, relative)?;
    let identifier = name
        .strip_prefix(prefix)
        .and_then(|value| uuid::Uuid::parse_str(value).ok());
    let valid_name = identifier.is_some_and(|identifier| name == format!("{prefix}{identifier}"));
    if folder != expected_folder || !valid_name {
        return Err(PluginError::RecoveryBlocked(
            "Invalid plugin transaction filename",
        ));
    }
    Ok(())
}

pub(super) fn write_journal(server_dir: &Path, journal: &PluginJournal) -> Result<(), PluginError> {
    let path = journal_path(server_dir);
    let parent = path
        .parent()
        .ok_or(PluginError::InvalidPayload("journal parent"))?;
    fs::create_dir_all(parent).map_err(PluginError::Io)?;
    sync_parent_dir(parent)?;
    let mut staged = tempfile::NamedTempFile::new_in(parent).map_err(PluginError::Io)?;
    staged
        .write_all(&serde_json::to_vec(journal).map_err(PluginError::Json)?)
        .map_err(PluginError::Io)?;
    staged.as_file().sync_all().map_err(PluginError::Io)?;
    staged
        .persist(&path)
        .map_err(|e| PluginError::Io(e.error))?;
    sync_parent_dir(&path)?;
    Ok(())
}
