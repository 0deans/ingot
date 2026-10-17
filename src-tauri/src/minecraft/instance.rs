use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Manager, Runtime};

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

pub fn get_instances_dir<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, InstanceError> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| InstanceError::Directory(format!("Failed to get app data directory: {e}")))?;

    let instances_dir = data_dir.join("instances");
    crate::app::state(app)
        .instances
        .with_serialized(|| fs::create_dir_all(&instances_dir))??;
    Ok(instances_dir)
}

pub fn get_instance_dir<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<PathBuf, InstanceError> {
    let instances_dir = get_instances_dir(app)?;
    ensure_instance_directory(
        &crate::app::state(app).instances,
        &instances_dir,
        instance_id,
    )
}

pub fn load_instances<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<InstanceConfig>, InstanceError> {
    crate::app::state(app)
        .instances
        .load()
        .map_err(InstanceError::Metadata)
}

pub fn get_instances<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<InstanceConfig>, InstanceError> {
    load_instances(app)
}

pub(crate) fn patch_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    id: &str,
    change: impl FnOnce(&mut InstanceConfig),
) -> Result<(), InstanceError> {
    crate::app::state(app)
        .instances
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

pub fn create_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    name: String,
    game_version: String,
    loader: ModLoaderType,
    loader_version: Option<String>,
) -> Result<InstanceConfig, InstanceError> {
    let instance = prepare_instance(name, game_version, loader, loader_version)?;
    let parent = get_instances_dir(app)?;
    let staging = tempfile::tempdir_in(&parent)?;
    let result = (|| {
        for directory in ["mods", "config", "saves", "resourcepacks"] {
            fs::create_dir_all(staging.path().join(directory))?;
        }
        storage::publish_instance(
            &crate::app::state(app).instances,
            &parent,
            instance,
            staging.path(),
        )
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

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

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

pub fn update_last_played<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    played_seconds: u64,
) -> Result<(), InstanceError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| InstanceError::Directory(error.to_string()))?
        .as_secs();
    patch_instance(app, instance_id, |existing| {
        existing.last_played = Some(now);
        existing.total_play_time_seconds = existing
            .total_play_time_seconds
            .saturating_add(played_seconds);
    })
}

fn ensure_instance_directory(
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

pub fn duplicate_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    source_id: &str,
    name: String,
    include_worlds: bool,
) -> Result<InstanceConfig, InstanceError> {
    let parent = get_instances_dir(app)?;
    duplicate::duplicate_instance_record(
        &crate::app::state(app).instances,
        &parent,
        source_id,
        name,
        include_worlds,
    )
}

pub fn delete_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    id: &str,
) -> Result<(), InstanceError> {
    let parent = get_instances_dir(app)?;
    storage::delete_instance_record(&crate::app::state(app).instances, &parent, id)
}

pub fn update_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance: InstanceConfig,
) -> Result<(), InstanceError> {
    let parent = get_instances_dir(app)?;
    storage::update_instance_record(&crate::app::state(app).instances, &parent, instance)
}
