use crate::minecraft::metadata::MetadataRepository;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

mod duplicate;
mod error;
mod models;
mod storage;
#[cfg(test)]
mod tests;
pub use error::InstanceError;
mod worlds;
pub use models::{InstanceConfig, ModLoaderType};
pub use worlds::{get_instance_worlds, InstanceWorldSummary};

pub(crate) fn ensure_instances_directory(
    repository: &MetadataRepository<InstanceConfig>,
    parent: &Path,
) -> Result<PathBuf, InstanceError> {
    repository.with_serialized(|| fs::create_dir_all(parent))??;
    Ok(parent.to_owned())
}

pub(crate) fn load_instances(
    repository: &MetadataRepository<InstanceConfig>,
) -> Result<Vec<InstanceConfig>, InstanceError> {
    repository.load().map_err(InstanceError::Metadata)
}

pub(crate) fn patch_instance(
    repository: &MetadataRepository<InstanceConfig>,
    id: &str,
    change: impl FnOnce(&mut InstanceConfig),
) -> Result<(), InstanceError> {
    repository
        .update(|records| {
            let record = records
                .iter_mut()
                .find(|record| record.id == id)
                .ok_or_else(|| {
                    crate::minecraft::metadata::MetadataError::Conflict("Instance not found".into())
                })?;
            change(record);
            Ok(())
        })
        .map_err(InstanceError::Metadata)
}

pub(crate) fn create_instance(
    repository: &MetadataRepository<InstanceConfig>,
    parent: &Path,
    name: String,
    game_version: String,
    loader: ModLoaderType,
    loader_version: Option<String>,
) -> Result<InstanceConfig, InstanceError> {
    let instance = prepare_instance(name, game_version, loader, loader_version)?;
    ensure_instances_directory(repository, parent)?;
    let staging = tempfile::tempdir_in(parent)?;
    let result = (|| {
        for directory in ["mods", "config", "saves", "resourcepacks"] {
            fs::create_dir_all(staging.path().join(directory))?;
        }
        storage::publish_instance(repository, parent, instance, staging.path())
    })();
    storage::finish_publication(staging, result)
}

pub(crate) fn prepare_instance(
    name: String,
    game_version: String,
    loader: ModLoaderType,
    loader_version: Option<String>,
) -> Result<InstanceConfig, InstanceError> {
    if name.trim().is_empty() {
        return Err(InstanceError::EmptyName);
    }

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let id = uuid::Uuid::new_v4().to_string();
    let instance = InstanceConfig {
        id,
        name,
        game_version,
        loader,
        loader_version,
        java_path: None,
        memory_min_mb: None,
        memory_max_mb: None,
        icon: None,
        created_at: now,
        last_played: None,
        total_play_time_seconds: 0,
        jvm_args: None,
        fullscreen: None,
        window_width: None,
        window_height: None,
        sync_options: None,
        sync_servers: None,
        sync_resource_packs: None,
        sync_command_history: None,
        sync_creative_hotbars: None,
        last_synced_at: None,
    };

    Ok(instance)
}

pub(crate) fn update_last_played(
    repository: &MetadataRepository<InstanceConfig>,
    instance_id: &str,
    played_seconds: u64,
) -> Result<(), InstanceError> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    patch_instance(repository, instance_id, |existing| {
        existing.last_played = Some(now);
        existing.total_play_time_seconds = existing
            .total_play_time_seconds
            .saturating_add(played_seconds);
    })
}

pub(crate) fn ensure_instance_directory(
    repository: &crate::minecraft::metadata::MetadataRepository<InstanceConfig>,
    parent: &Path,
    id: &str,
) -> Result<PathBuf, InstanceError> {
    repository.with_serialized(|| create_instance_directory(parent, id))?
}

fn create_instance_directory(parent: &Path, id: &str) -> Result<PathBuf, InstanceError> {
    let directory =
        crate::app::safe_enclosed_filename(parent, id).ok_or(InstanceError::InvalidId)?;
    fs::create_dir_all(&directory)?;
    for name in ["mods", "config", "saves", "resourcepacks"] {
        let child =
            crate::app::safe_enclosed_filename(&directory, name).ok_or(InstanceError::InvalidId)?;
        fs::create_dir_all(child)?;
    }
    Ok(directory)
}

pub(crate) fn duplicate_instance(
    repository: &MetadataRepository<InstanceConfig>,
    parent: &Path,
    source_id: &str,
    name: String,
    include_worlds: bool,
) -> Result<InstanceConfig, InstanceError> {
    ensure_instances_directory(repository, parent)?;
    duplicate::duplicate_instance_record(repository, parent, source_id, name, include_worlds)
}

pub(crate) fn delete_instance(
    repository: &MetadataRepository<InstanceConfig>,
    parent: &Path,
    id: &str,
) -> Result<(), InstanceError> {
    ensure_instances_directory(repository, parent)?;
    storage::delete_instance_record(repository, parent, id)
}

pub(crate) fn update_instance(
    repository: &MetadataRepository<InstanceConfig>,
    parent: &Path,
    instance: InstanceConfig,
) -> Result<(), InstanceError> {
    ensure_instances_directory(repository, parent)?;
    storage::update_instance_record(repository, parent, instance)
}

#[cfg(test)]
mod service_tests;
