use super::{InstanceConfig, InstanceError};
use crate::minecraft::metadata::{MetadataError, MetadataRepository};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn publish_instance(
    repository: &MetadataRepository<InstanceConfig>,
    parent: &Path,
    config: InstanceConfig,
    staging: &Path,
) -> Result<InstanceConfig, InstanceError> {
    let destination =
        crate::app::safe_enclosed_filename(parent, &config.id).ok_or(InstanceError::InvalidId)?;
    repository.publish(
        staging,
        &destination,
        config.clone(),
        |records, candidate| {
            if records.iter().any(|record| record.id == candidate.id) {
                return Err(MetadataError::Conflict("Instance already exists".into()));
            }
            Ok(())
        },
    )?;
    Ok(config)
}

pub(super) fn finish_publication(
    staging: tempfile::TempDir,
    result: Result<InstanceConfig, InstanceError>,
) -> Result<InstanceConfig, InstanceError> {
    match result {
        Ok(config) => {
            let _ = staging.keep();
            Ok(config)
        }
        Err(primary) => {
            if let Err(cleanup) = staging.close() {
                return Err(InstanceError::Cleanup {
                    primary: Box::new(primary),
                    cleanup,
                });
            }
            Err(primary)
        }
    }
}

fn metadata_io(operation: &'static str, path: &Path, source: std::io::Error) -> MetadataError {
    MetadataError::Io {
        operation,
        path: path.to_owned(),
        source,
    }
}

struct RetainedDirectory {
    original: PathBuf,
    backup: PathBuf,
}

pub(super) fn delete_instance_record(
    repository: &MetadataRepository<InstanceConfig>,
    parent: &Path,
    id: &str,
) -> Result<(), InstanceError> {
    delete_with_rename(repository, parent, id, |source, destination| {
        fs::rename(source, destination)
    })
}

pub(super) fn delete_with_rename(
    repository: &MetadataRepository<InstanceConfig>,
    parent: &Path,
    id: &str,
    rename: impl Fn(&Path, &Path) -> std::io::Result<()>,
) -> Result<(), InstanceError> {
    let directory =
        crate::app::safe_enclosed_filename(parent, id).ok_or(InstanceError::InvalidId)?;
    let retained = repository.update_compensated(
        |records| {
            let index = records
                .iter()
                .position(|record| record.id == id)
                .ok_or_else(|| MetadataError::Conflict("Instance not found".into()))?;
            let retained = match fs::symlink_metadata(&directory) {
                Ok(metadata) if metadata.is_dir() => {
                    let backup = parent.join(format!(".deleted-{id}-{}", uuid::Uuid::new_v4()));
                    rename(&directory, &backup).map_err(|error| {
                        metadata_io("preserve instance directory", &directory, error)
                    })?;
                    Some(RetainedDirectory {
                        original: directory.clone(),
                        backup,
                    })
                }
                Ok(_) => {
                    return Err(metadata_io(
                        "inspect instance directory",
                        &directory,
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Instance path is not a directory",
                        ),
                    ))
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => {
                    return Err(metadata_io("inspect instance directory", &directory, error))
                }
            };
            records.remove(index);
            Ok(retained)
        },
        |retained| {
            if let Some(retained) = retained {
                rename(&retained.backup, &retained.original).map_err(|error| {
                    metadata_io(
                        "restore deleted instance directory",
                        &retained.backup,
                        error,
                    )
                })?;
            }
            Ok(())
        },
    )?;
    if let Some(retained) = retained {
        fs::remove_dir_all(&retained.backup).map_err(|source| {
            InstanceError::FilesCleanupCommitted {
                path: retained.backup,
                source,
            }
        })?;
    }
    Ok(())
}

pub(super) fn update_instance_record(
    repository: &MetadataRepository<InstanceConfig>,
    parent: &Path,
    config: InstanceConfig,
) -> Result<(), InstanceError> {
    crate::app::safe_enclosed_filename(parent, &config.id).ok_or(InstanceError::InvalidId)?;
    repository.update(|records| {
        let existing = records
            .iter_mut()
            .find(|record| record.id == config.id)
            .ok_or_else(|| MetadataError::Conflict("Instance not found".into()))?;
        *existing = config;
        Ok(())
    })?;
    Ok(())
}
