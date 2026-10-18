use super::{MetadataError, MetadataRepository};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PublicationReceipt {
    pub(crate) version: u32,
    pub(crate) id: String,
    pub(crate) staged: String,
}

impl<T: Serialize + DeserializeOwned> MetadataRepository<T> {
    pub(super) fn publication_receipt_path(&self) -> PathBuf {
        self.path.with_extension("publication.json")
    }

    pub(super) fn write_publication_receipt(
        &self,
        receipt: &PublicationReceipt,
    ) -> Result<(), MetadataError> {
        let path = self.publication_receipt_path();
        let parent = path
            .parent()
            .ok_or(MetadataError::RecoveryBlocked("Missing receipt parent"))?;
        fs::create_dir_all(parent)
            .map_err(|source| path_io("create publication receipt directory", parent, source))?;
        let mut stage = tempfile::NamedTempFile::new_in(parent)
            .map_err(|source| path_io("stage publication receipt", &path, source))?;
        stage
            .write_all(&serde_json::to_vec(receipt)?)
            .map_err(|source| path_io("write publication receipt", &path, source))?;
        stage
            .as_file()
            .sync_all()
            .map_err(|source| path_io("sync publication receipt", &path, source))?;
        stage
            .persist(&path)
            .map_err(|error| path_io("publish publication receipt", &path, error.error))?;
        sync_parent(&path)
    }

    pub(super) fn recover_publication(&self, records: &[T]) -> Result<(), MetadataError> {
        self.recover_publication_with(records, |path| fs::remove_dir_all(path))
    }

    pub(super) fn recover_publication_with(
        &self,
        records: &[T],
        cleanup: impl FnOnce(&Path) -> io::Result<()>,
    ) -> Result<(), MetadataError> {
        let path = self.publication_receipt_path();
        match fs::symlink_metadata(&path) {
            Ok(metadata) if !metadata.is_file() || metadata.file_type().is_symlink() => {
                return Err(MetadataError::RecoveryBlocked(
                    "Publication receipt is not a plain file",
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(source) => return Err(path_io("inspect publication receipt", &path, source)),
        }
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(source) => return Err(path_io("read publication receipt", &path, source)),
        };
        let receipt: PublicationReceipt =
            serde_json::from_slice(&bytes).map_err(|source| MetadataError::Decode {
                path: path.clone(),
                source,
            })?;
        let uuid = receipt
            .staged
            .strip_prefix(".publishing-")
            .and_then(|value| uuid::Uuid::parse_str(value).ok());
        let valid_staged =
            uuid.is_some_and(|value| receipt.staged == format!(".publishing-{value}"));
        if receipt.version != 1 || !valid_staged || receipt.id == receipt.staged {
            return Err(MetadataError::RecoveryBlocked(
                "Invalid publication receipt",
            ));
        }
        let policy = self.policy.as_ref().ok_or(MetadataError::RecoveryBlocked(
            "Publication recovery requires an identity policy",
        ))?;
        let original = checked_path(&policy.directory, &receipt.id)?;
        let staged = checked_path(&policy.directory, &receipt.staged)?;
        let count = records
            .iter()
            .filter(|record| (policy.identify)(record) == receipt.id)
            .count();
        if count > 1 {
            return Err(MetadataError::RecoveryBlocked("Duplicate record identity"));
        }
        if count == 0 {
            sync_parent(&self.path)?;
        }
        let original_exists = directory_exists(&original)?;
        let staged_exists = directory_exists(&staged)?;
        match (count == 1, original_exists, staged_exists) {
            (true, true, false) => {}
            (true, false, true) => {
                fs::rename(&staged, &original)
                    .map_err(|source| path_io("restore published directory", &staged, source))?;
            }
            (false, false, true) => {
                cleanup(&staged).map_err(|source| MetadataError::RecoveryCleanup {
                    path: staged.clone(),
                    source,
                })?;
            }
            (false, true, false) => {
                cleanup(&original).map_err(|source| MetadataError::RecoveryCleanup {
                    path: original.clone(),
                    source,
                })?;
            }
            (false, false, false) => {}
            _ => {
                return Err(MetadataError::RecoveryBlocked(
                    "Ambiguous publication directories",
                ));
            }
        }
        sync_parent(&original)?;
        fs::remove_file(&path)
            .map_err(|source| path_io("acknowledge publication receipt", &path, source))?;
        sync_parent(&path)?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn publish_with_effects(
        &self,
        staging: &Path,
        destination: &Path,
        record: T,
        validate: impl FnOnce(&[T], &T) -> Result<(), MetadataError>,
        write: impl FnOnce(&[T]) -> Result<(), MetadataError>,
        rename: impl Fn(&Path, &Path) -> io::Result<()>,
        cleanup: impl Fn(&Path) -> io::Result<()>,
        acknowledge: impl FnOnce(&Path) -> io::Result<()>,
    ) -> Result<(), MetadataError> {
        let _guard = self.lock.lock().map_err(|_| MetadataError::LockPoisoned)?;
        let mut records = self.read()?;
        self.recover(&records)?;
        validate(&records, &record)?;
        if destination
            .try_exists()
            .map_err(|source| self.io("inspect destination", source))?
        {
            return Err(MetadataError::Conflict(
                "Publication destination already exists".into(),
            ));
        }
        let policy = self.policy.as_ref().ok_or(MetadataError::RecoveryBlocked(
            "Publication requires an identity policy",
        ))?;
        let policy_dir = &policy.directory;
        let id_str = (policy.identify)(&record);
        if records
            .iter()
            .any(|existing| (policy.identify)(existing) == id_str)
        {
            return Err(MetadataError::Conflict(
                "Publication identity already exists".into(),
            ));
        }
        let checked_dest = checked_path(policy_dir, id_str)?;
        if checked_dest != destination {
            return Err(MetadataError::RecoveryBlocked(
                "Publication destination does not match identity",
            ));
        }
        let staged_name = format!(".publishing-{}", uuid::Uuid::new_v4());
        let durable_stage = checked_path(policy_dir, &staged_name)?;

        let receipt = PublicationReceipt {
            version: 1,
            id: id_str.to_string(),
            staged: staged_name,
        };
        self.write_publication_receipt(&receipt)?;
        if let Err(source) = rename(staging, &durable_stage) {
            return self.failed_publish(self.io("stage directory", source), &records, &cleanup);
        }
        sync_parent(staging)?;
        sync_parent(&durable_stage)?;

        if let Err(source) = rename(&durable_stage, &checked_dest) {
            let primary = self.io("publish directory", source);
            if rename(&durable_stage, staging).is_ok() {
                sync_parent(staging)?;
                let receipt_path = self.publication_receipt_path();
                if let Err(source) = fs::remove_file(&receipt_path) {
                    return Err(MetadataError::Rollback {
                        primary: Box::new(primary),
                        rollback: Box::new(path_io("acknowledge rollback", &receipt_path, source)),
                    });
                }
                sync_parent(&receipt_path)?;
                return Err(primary);
            }
            return self.failed_publish(primary, &records, &cleanup);
        }
        sync_parent(&checked_dest)?;

        records.push(record);
        if let Err(primary) = write(&records) {
            let current = match self.read() {
                Ok(current) => current,
                Err(read_error) => {
                    return Err(MetadataError::Rollback {
                        primary: Box::new(primary),
                        rollback: Box::new(read_error),
                    })
                }
            };
            let committed = current
                .iter()
                .any(|record| (policy.identify)(record) == receipt.id);
            if committed {
                return Err(primary);
            }
            if rename(&checked_dest, staging).is_ok() {
                sync_parent(staging)?;
                let receipt_path = self.publication_receipt_path();
                if let Err(source) = fs::remove_file(&receipt_path) {
                    return Err(MetadataError::Rollback {
                        primary: Box::new(primary),
                        rollback: Box::new(path_io("acknowledge rollback", &receipt_path, source)),
                    });
                }
                sync_parent(&receipt_path)?;
                return Err(primary);
            }
            records.pop();
            return self.failed_publish(primary, &records, &cleanup);
        }

        sync_parent(&self.path)?;
        let receipt_path = self.publication_receipt_path();
        acknowledge(&receipt_path)
            .map_err(|source| path_io("acknowledge publication receipt", &receipt_path, source))?;
        sync_parent(&receipt_path)?;
        Ok(())
    }

    pub(super) fn failed_publish(
        &self,
        primary: MetadataError,
        previous: &[T],
        cleanup: impl Fn(&Path) -> Result<(), io::Error>,
    ) -> Result<(), MetadataError> {
        match self.recover_publication_with(previous, cleanup) {
            Ok(()) => Err(primary),
            Err(rollback) => Err(MetadataError::Rollback {
                primary: Box::new(primary),
                rollback: Box::new(rollback),
            }),
        }
    }
}

fn checked_path(parent: &Path, name: &str) -> Result<PathBuf, MetadataError> {
    crate::app::safe_enclosed_filename(parent, name)
        .ok_or(MetadataError::RecoveryBlocked("Unsafe publication path"))
}

fn directory_exists(path: &Path) -> Result<bool, MetadataError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(true),
        Ok(_) => Err(MetadataError::RecoveryBlocked(
            "Publication path is not a plain directory",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(path_io("inspect publication directory", path, source)),
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
            .map_err(|source| path_io("sync publication directory", parent, source))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(test)]
pub(super) mod tests;
