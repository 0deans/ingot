use super::{InstanceConfig, InstanceError};
use crate::minecraft::metadata::{DeletionFiles, MetadataError, MetadataRepository};
use std::path::Path;

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

pub(super) fn delete_instance_record(
    repository: &MetadataRepository<InstanceConfig>,
    parent: &Path,
    id: &str,
) -> Result<(), InstanceError> {
    crate::app::safe_enclosed_filename(parent, id).ok_or(InstanceError::InvalidId)?;
    repository.delete_record(id, DeletionFiles::Remove)?;
    Ok(())
}

#[cfg(test)]
pub(super) fn delete_with_rename(
    repository: &MetadataRepository<InstanceConfig>,
    parent: &Path,
    id: &str,
    rename: impl Fn(&Path, &Path) -> std::io::Result<()>,
) -> Result<(), InstanceError> {
    crate::app::safe_enclosed_filename(parent, id).ok_or(InstanceError::InvalidId)?;
    repository.delete_with_rename(id, DeletionFiles::Remove, rename)?;
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
