use super::{InstanceConfig, InstanceError};
use crate::minecraft::metadata::MetadataRepository;
use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

const NOT_COPIED: &[&str] = &["logs", "crash-reports", "screenshots", ".ingot"];

fn linked_entry(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    false
}

pub(super) fn copy_instance_tree(
    from: &Path,
    to: &Path,
    top: bool,
    include_worlds: bool,
) -> Result<(), InstanceError> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_str().ok_or(InstanceError::InvalidId)?;
        let metadata = fs::symlink_metadata(entry.path())?;
        // Quick Play links intentionally point back into the original instance.
        if linked_entry(&metadata) || name == "session.lock" || (top && NOT_COPIED.contains(&name))
        {
            continue;
        }
        crate::app::safe_enclosed_filename(from, name).ok_or(InstanceError::InvalidId)?;
        let destination =
            crate::app::safe_enclosed_filename(to, name).ok_or(InstanceError::InvalidId)?;
        if metadata.is_dir() {
            if top && name == "saves" && !include_worlds {
                fs::create_dir_all(destination)?;
                continue;
            }
            copy_instance_tree(&entry.path(), &destination, false, include_worlds)?;
        } else if metadata.is_file() {
            fs::copy(entry.path(), destination)?;
        } else {
            return Err(InstanceError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unsupported instance entry",
            )));
        }
    }
    Ok(())
}

pub(super) fn duplicate_instance_record(
    repository: &MetadataRepository<InstanceConfig>,
    parent: &Path,
    source_id: &str,
    name: String,
    include_worlds: bool,
) -> Result<InstanceConfig, InstanceError> {
    let name = name.trim().to_owned();
    if name.is_empty() {
        return Err(InstanceError::EmptyName);
    }
    let source_path =
        crate::app::safe_enclosed_filename(parent, source_id).ok_or(InstanceError::InvalidId)?;
    let source = repository
        .load()?
        .into_iter()
        .find(|record| record.id == source_id)
        .ok_or_else(|| InstanceError::NotFound(source_id.to_owned()))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| InstanceError::Io(std::io::Error::other(error)))?
        .as_secs();
    let copy = InstanceConfig {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        created_at: now,
        last_played: None,
        total_play_time_seconds: 0,
        last_synced_at: None,
        ..source
    };
    let staging = tempfile::Builder::new()
        .prefix(".instance-copy-")
        .tempdir_in(parent)?;
    let result = (|| {
        copy_instance_tree(&source_path, staging.path(), true, include_worlds)?;
        super::storage::publish_instance(repository, parent, copy, staging.path())
    })();
    super::storage::finish_publication(staging, result)
}
