//! Instance and server metadata share one synchronous publication protocol.
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs, io,
    io::Write,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Mutex,
};

#[derive(Debug, thiserror::Error)]
pub enum MetadataError {
    #[error("Metadata lock poisoned")]
    LockPoisoned,
    #[error("Failed to {operation} metadata {path}: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Invalid metadata {path}: {source}")]
    Decode {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("Failed to encode metadata: {0}")]
    Encode(#[from] serde_json::Error),
    #[error("{0}")]
    Conflict(String),
    #[error("{primary}; publication cleanup also failed: {cleanup}")]
    Cleanup {
        primary: Box<MetadataError>,
        cleanup: io::Error,
    },
    #[error("{primary}; compensating rollback also failed: {rollback}")]
    Rollback {
        #[source]
        primary: Box<MetadataError>,
        rollback: Box<MetadataError>,
    },
}

pub(crate) struct MetadataRepository<T> {
    path: PathBuf,
    lock: Mutex<()>,
    kind: PhantomData<T>,
}

impl<T: Serialize + DeserializeOwned> MetadataRepository<T> {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Mutex::new(()),
            kind: PhantomData,
        }
    }

    pub(crate) fn load(&self) -> Result<Vec<T>, MetadataError> {
        let _guard = self.lock.lock().map_err(|_| MetadataError::LockPoisoned)?;
        self.read()
    }

    pub(crate) fn update<R>(
        &self,
        change: impl FnOnce(&mut Vec<T>) -> Result<R, MetadataError>,
    ) -> Result<R, MetadataError> {
        self.update_with(change, |records| self.write(records))
    }

    pub(crate) fn with_serialized<R>(
        &self,
        action: impl FnOnce() -> R,
    ) -> Result<R, MetadataError> {
        let _guard = self.lock.lock().map_err(|_| MetadataError::LockPoisoned)?;
        Ok(action())
    }

    pub(crate) fn update_compensated<R>(
        &self,
        change: impl FnOnce(&mut Vec<T>) -> Result<R, MetadataError>,
        rollback: impl FnOnce(&R) -> Result<(), MetadataError>,
    ) -> Result<R, MetadataError> {
        let _guard = self.lock.lock().map_err(|_| MetadataError::LockPoisoned)?;
        let mut records = self.read()?;
        let result = change(&mut records)?;
        if let Err(primary) = self.write(&records) {
            if let Err(rollback) = rollback(&result) {
                return Err(MetadataError::Rollback {
                    primary: Box::new(primary),
                    rollback: Box::new(rollback),
                });
            }
            return Err(primary);
        }
        Ok(result)
    }

    fn update_with<R>(
        &self,
        change: impl FnOnce(&mut Vec<T>) -> Result<R, MetadataError>,
        write: impl FnOnce(&[T]) -> Result<(), MetadataError>,
    ) -> Result<R, MetadataError> {
        let _guard = self.lock.lock().map_err(|_| MetadataError::LockPoisoned)?;
        let mut records = self.read()?;
        let result = change(&mut records)?;
        write(&records)?;
        Ok(result)
    }

    pub(crate) fn publish(
        &self,
        staging: &Path,
        destination: &Path,
        record: T,
        validate: impl FnOnce(&[T], &T) -> Result<(), MetadataError>,
    ) -> Result<(), MetadataError> {
        self.publish_with(staging, destination, record, validate, |records| {
            self.write(records)
        })
    }

    fn publish_with(
        &self,
        staging: &Path,
        destination: &Path,
        record: T,
        validate: impl FnOnce(&[T], &T) -> Result<(), MetadataError>,
        write: impl FnOnce(&[T]) -> Result<(), MetadataError>,
    ) -> Result<(), MetadataError> {
        let _guard = self.lock.lock().map_err(|_| MetadataError::LockPoisoned)?;
        let mut records = self.read()?;
        validate(&records, &record)?;
        if destination
            .try_exists()
            .map_err(|source| self.io("inspect destination", source))?
        {
            return Err(MetadataError::Conflict(
                "Publication destination already exists".into(),
            ));
        }
        fs::rename(staging, destination).map_err(|source| self.io("publish directory", source))?;
        records.push(record);
        if let Err(primary) = write(&records) {
            if let Err(cleanup) = fs::rename(destination, staging) {
                return Err(MetadataError::Cleanup {
                    primary: Box::new(primary),
                    cleanup,
                });
            }
            return Err(primary);
        }
        Ok(())
    }

    fn read(&self) -> Result<Vec<T>, MetadataError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => return Err(self.io("read", source)),
        };
        serde_json::from_slice(&bytes).map_err(|source| MetadataError::Decode {
            path: self.path.clone(),
            source,
        })
    }

    fn write(&self, records: &[T]) -> Result<(), MetadataError> {
        let bytes = serde_json::to_vec_pretty(records)?;
        let directory = self.path.parent().unwrap_or(Path::new("."));
        fs::create_dir_all(directory).map_err(|source| self.io("create directory", source))?;
        let mut staged = tempfile::NamedTempFile::new_in(directory)
            .map_err(|source| self.io("stage", source))?;
        staged
            .write_all(&bytes)
            .map_err(|source| self.io("write", source))?;
        staged
            .as_file()
            .sync_all()
            .map_err(|source| self.io("flush", source))?;
        staged
            .persist(&self.path)
            .map_err(|error| self.io("replace", error.error))?;
        Ok(())
    }

    fn io(&self, operation: &'static str, source: io::Error) -> MetadataError {
        MetadataError::Io {
            operation,
            path: self.path.clone(),
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    #[test]
    fn concurrent_updates_preserve_every_record() {
        let root = tempfile::tempdir().unwrap();
        let repository = Arc::new(MetadataRepository::<u32>::new(
            root.path().join("store.json"),
        ));
        let workers: Vec<_> = (0..16)
            .map(|id| {
                let repository = repository.clone();
                std::thread::spawn(move || {
                    repository
                        .update(|records| {
                            records.push(id);
                            Ok(())
                        })
                        .unwrap()
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
        let mut result = repository.load().unwrap();
        result.sort();
        assert_eq!(result, (0..16).collect::<Vec<_>>());
    }
    #[test]
    fn invalid_and_empty_storage_is_preserved() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("store.json");
        let repository = MetadataRepository::<u32>::new(path.clone());
        assert!(repository.load().unwrap().is_empty());
        for bytes in [b"".as_slice(), b"{invalid".as_slice()] {
            fs::write(&path, bytes).unwrap();
            assert!(matches!(
                repository.update(|records| {
                    records.push(1);
                    Ok(())
                }),
                Err(MetadataError::Decode { .. })
            ));
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
    }
    #[test]
    fn rejected_change_preserves_stored_bytes() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("store.json");
        fs::write(&path, b"[1]").unwrap();
        let repository = MetadataRepository::<u32>::new(path.clone());
        let result: Result<(), _> = repository.update(|records| {
            records.push(2);
            Err(MetadataError::Conflict("no".into()))
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"[1]");
    }

    #[test]
    fn failed_metadata_publication_returns_directory_to_staging() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store.json");
        fs::write(&path, b"[1]").unwrap();
        let repository = MetadataRepository::<u32>::new(path.clone());
        let staging = directory.path().join("staging");
        let destination = directory.path().join("published");
        fs::create_dir_all(&staging).unwrap();
        fs::write(staging.join("content"), b"complete").unwrap();
        let result = repository.publish_with(
            &staging,
            &destination,
            2,
            |_, _| Ok(()),
            |_| {
                Err(repository.io(
                    "replace",
                    io::Error::new(io::ErrorKind::PermissionDenied, "injected"),
                ))
            },
        );
        assert!(result.is_err());
        assert!(!destination.exists());
        assert_eq!(fs::read(staging.join("content")).unwrap(), b"complete");
        assert_eq!(fs::read(&path).unwrap(), b"[1]");
    }

    #[test]
    fn failed_metadata_write_preserves_previous_store() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store.json");
        fs::write(&path, b"[1]").unwrap();
        let repository = MetadataRepository::<u32>::new(path.clone());
        let result = repository.update_with(
            |records| {
                records.push(2);
                Ok(())
            },
            |_| {
                Err(repository.io(
                    "replace",
                    io::Error::new(io::ErrorKind::PermissionDenied, "injected"),
                ))
            },
        );
        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"[1]");
    }

    #[test]
    fn publication_preserves_existing_destination_and_staged_data() {
        let directory = tempfile::tempdir().unwrap();
        let repository = MetadataRepository::<u32>::new(directory.path().join("store.json"));
        let staging = directory.path().join("staging");
        let destination = directory.path().join("published");
        fs::create_dir_all(&staging).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(destination.join("content"), b"old").unwrap();
        fs::write(staging.join("content"), b"new").unwrap();
        assert!(repository
            .publish(&staging, &destination, 1, |_, _| Ok(()))
            .is_err());
        assert_eq!(fs::read(destination.join("content")).unwrap(), b"old");
        assert_eq!(fs::read(staging.join("content")).unwrap(), b"new");
        assert!(repository.load().unwrap().is_empty());
    }
}
