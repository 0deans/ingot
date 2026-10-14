pub mod execute;
pub mod models;
pub mod policy;
pub mod service;

#[cfg(test)]
mod tests;

pub use execute::backup_file_if_exists;
pub use models::{
    ConflictResolution, SharedSyncStatus, SyncCategory, SyncConflictInfo, SyncError, SyncReport,
};
pub use policy::EffectiveSync;

use crate::minecraft::instance::InstanceConfig;
use std::path::{Path, PathBuf};
use tauri::Runtime;

pub fn get_shared_dir<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    service::get_shared_dir(app).map_err(Into::into)
}

pub fn mark_instance_synced<R: Runtime>(app: &tauri::AppHandle<R>, instance_id: &str) -> Result<(), String> {
    service::mark_instance_synced(app, instance_id).map_err(Into::into)
}

pub fn sync_before_launch<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance: &InstanceConfig,
    instance_dir: &Path,
) -> Result<(), String> {
    service::sync_before_launch(app, instance, instance_dir).map_err(Into::into)
}

pub fn sync_after_exit<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance: &InstanceConfig,
    instance_dir: &Path,
) -> Result<(), String> {
    service::sync_after_exit(app, instance, instance_dir).map_err(Into::into)
}

pub fn export_instance_category_to_shared<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    category: &str,
) -> Result<SyncReport, String> {
    service::export_instance_category_to_shared(app, instance_id, category).map_err(Into::into)
}

pub fn export_instance_to_shared<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<SyncReport, String> {
    service::export_instance_to_shared(app, instance_id).map_err(Into::into)
}

pub fn import_shared_to_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<SyncReport, String> {
    service::import_shared_to_instance(app, instance_id).map_err(Into::into)
}

pub fn get_shared_sync_status<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<SharedSyncStatus, String> {
    service::get_shared_sync_status(app).map_err(Into::into)
}

pub fn check_sync_conflict<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<Option<SyncConflictInfo>, String> {
    service::check_sync_conflict(app, instance_id).map_err(Into::into)
}

pub fn resolve_sync_conflict<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    resolution: &str,
) -> Result<SyncReport, String> {
    service::resolve_sync_conflict(app, instance_id, resolution).map_err(Into::into)
}
