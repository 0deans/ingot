use super::{MetadataError, MetadataRepository};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

pub(super) struct DeletionPolicy<T> {
    pub(super) directory: PathBuf,
    pub(super) identify: fn(&T) -> &str,
}

pub(crate) enum DeletionFiles {
    Retain,
    Remove,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    version: u32,
    id: String,
    backup: String,
}

impl<T: Serialize + DeserializeOwned> MetadataRepository<T> {
    pub(crate) fn delete_record(
        &self,
        id: &str,
        files: DeletionFiles,
    ) -> Result<(), MetadataError> {
        self.delete_with_rename(id, files, |source, destination| {
            fs::rename(source, destination)
        })
    }

    pub(crate) fn delete_with_rename(
        &self,
        id: &str,
        files: DeletionFiles,
        rename: impl Fn(&Path, &Path) -> io::Result<()>,
    ) -> Result<(), MetadataError> {
        let _guard = self.lock.lock().map_err(|_| MetadataError::LockPoisoned)?;
        let mut records = self.read()?;
        self.recover_deletion(&records)?;
        let policy = self
            .deletion
            .as_ref()
            .ok_or(MetadataError::RecoveryBlocked(
                "Deletion directory is not configured",
            ))?;
        let directory = checked_path(&policy.directory, id)?;
        let matches: Vec<_> = records
            .iter()
            .enumerate()
            .filter(|(_, record)| (policy.identify)(record) == id)
            .map(|(index, _)| index)
            .collect();
        let index = match matches.as_slice() {
            [index] => *index,
            [] => return Err(MetadataError::Conflict("Record not found".into())),
            _ => return Err(MetadataError::RecoveryBlocked("Duplicate record identity")),
        };
        let receipt = if matches!(files, DeletionFiles::Remove) && directory_exists(&directory)? {
            let receipt = Receipt {
                version: 1,
                id: id.into(),
                backup: format!(".deleted-{}", uuid::Uuid::new_v4()),
            };
            self.write_receipt(&receipt)?;
            let backup = checked_path(&policy.directory, &receipt.backup)?;
            if let Err(source) = rename(&directory, &backup) {
                let primary = path_io("retain deleted directory", &directory, source);
                return self.failed_delete(primary, &records);
            }
            if let Err(primary) = sync_parent(&directory) {
                return self.failed_delete(primary, &records);
            }
            Some(receipt)
        } else {
            None
        };
        let removed = records.remove(index);
        if let Err(primary) = self.write(&records) {
            // The failed replacement did not commit: restore from the intent using
            // the pre-commit record set, even if the store itself is now unreadable.
            records.insert(index, removed);
            return self.failed_delete(primary, &records);
        }
        if receipt.is_some() {
            self.recover_deletion(&records)?;
        }
        Ok(())
    }

    fn failed_delete(&self, primary: MetadataError, previous: &[T]) -> Result<(), MetadataError> {
        match self.recover_deletion(previous) {
            Ok(()) => Err(primary),
            Err(rollback) => Err(MetadataError::Rollback {
                primary: Box::new(primary),
                rollback: Box::new(rollback),
            }),
        }
    }

    pub(super) fn recover_deletion(&self, records: &[T]) -> Result<(), MetadataError> {
        self.recover_deletion_with(records, |path| fs::remove_dir_all(path))
    }

    fn recover_deletion_with(
        &self,
        records: &[T],
        cleanup: impl FnOnce(&Path) -> io::Result<()>,
    ) -> Result<(), MetadataError> {
        let path = self.receipt_path();
        match fs::symlink_metadata(&path) {
            Ok(metadata) if !metadata.is_file() || metadata.file_type().is_symlink() => {
                return Err(MetadataError::RecoveryBlocked(
                    "Deletion receipt is not a plain file",
                ))
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(source) => return Err(path_io("inspect deletion receipt", &path, source)),
        }
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(source) => return Err(path_io("read deletion receipt", &path, source)),
        };
        let policy = self
            .deletion
            .as_ref()
            .ok_or(MetadataError::RecoveryBlocked(
                "Deletion directory is not configured",
            ))?;
        let receipt: Receipt =
            serde_json::from_slice(&bytes).map_err(|source| MetadataError::Decode {
                path: path.clone(),
                source,
            })?;
        let uuid = receipt
            .backup
            .strip_prefix(".deleted-")
            .and_then(|value| uuid::Uuid::parse_str(value).ok());
        let valid_backup = uuid.is_some_and(|value| receipt.backup == format!(".deleted-{value}"));
        if receipt.version != 1 || !valid_backup || receipt.id == receipt.backup {
            return Err(MetadataError::RecoveryBlocked("Invalid deletion receipt"));
        }
        let original = checked_path(&policy.directory, &receipt.id)?;
        let backup = checked_path(&policy.directory, &receipt.backup)?;
        let count = records
            .iter()
            .filter(|record| (policy.identify)(record) == receipt.id)
            .count();
        if count > 1 {
            return Err(MetadataError::RecoveryBlocked("Duplicate record identity"));
        }
        // A committed metadata replacement is the acknowledgement that allows
        // destruction of retained files. Flush its directory before cleanup on Unix.
        if count == 0 {
            sync_parent(&self.path)?;
        }
        let original_exists = directory_exists(&original)?;
        let backup_exists = directory_exists(&backup)?;
        match (count == 1, original_exists, backup_exists) {
            (true, false, true) => fs::rename(&backup, &original)
                .map_err(|source| path_io("restore deleted directory", &backup, source))?,
            (true, true, false) => {}
            (false, false, true) => {
                cleanup(&backup).map_err(|source| MetadataError::RecoveryCleanup {
                    path: backup,
                    source,
                })?
            }
            (false, false, false) => {}
            _ => {
                return Err(MetadataError::RecoveryBlocked(
                    "Ambiguous deletion directories",
                ))
            }
        }
        sync_parent(&original)?;
        fs::remove_file(&path)
            .map_err(|source| path_io("acknowledge deletion receipt", &path, source))?;
        sync_parent(&path)?;
        Ok(())
    }

    fn receipt_path(&self) -> PathBuf {
        self.path.with_extension("deletion.json")
    }

    fn write_receipt(&self, receipt: &Receipt) -> Result<(), MetadataError> {
        let path = self.receipt_path();
        let parent = path
            .parent()
            .ok_or(MetadataError::RecoveryBlocked("Missing receipt parent"))?;
        fs::create_dir_all(parent)
            .map_err(|source| path_io("create receipt directory", parent, source))?;
        let mut stage = tempfile::NamedTempFile::new_in(parent)
            .map_err(|source| path_io("stage deletion receipt", &path, source))?;
        stage
            .write_all(&serde_json::to_vec(receipt)?)
            .map_err(|source| path_io("write deletion receipt", &path, source))?;
        stage
            .as_file()
            .sync_all()
            .map_err(|source| path_io("sync deletion receipt", &path, source))?;
        stage
            .persist(&path)
            .map_err(|error| path_io("publish deletion receipt", &path, error.error))?;
        sync_parent(&path)
    }
}

fn checked_path(parent: &Path, name: &str) -> Result<PathBuf, MetadataError> {
    crate::app::safe_enclosed_filename(parent, name)
        .ok_or(MetadataError::RecoveryBlocked("Unsafe deletion path"))
}

fn directory_exists(path: &Path) -> Result<bool, MetadataError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(true),
        Ok(_) => Err(MetadataError::RecoveryBlocked(
            "Deletion path is not a plain directory",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(path_io("inspect deletion directory", path, source)),
    }
}

fn path_io(operation: &'static str, path: &Path, source: io::Error) -> MetadataError {
    MetadataError::Io {
        operation,
        path: path.to_owned(),
        source,
    }
}

fn sync_parent(path: &Path) -> Result<(), MetadataError> {
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|source| path_io("sync deletion directory", parent, source))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests;
