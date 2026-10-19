use crate::app::instances::ensure_instance_dir;
use crate::minecraft::instance::InstanceConfig;
use crate::minecraft::sync::{
    execute::{ensure_shared_directories, file_exists, inspect_shared_status},
    models::{
        ConflictResolution, SharedSyncStatus, SyncCategory, SyncConflictInfo, SyncError, SyncReport,
    },
    policy::{evaluate_conflict, resolve_effective_sync, ConflictCheckInputs, EffectiveSync},
    service::{
        execute_sync_after_exit, execute_sync_before_launch, export_instance_category_files,
        import_shared_files_to_instance,
    },
};
use crate::system::load_settings;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tauri::{Manager, Runtime};
pub fn get_shared_dir<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, SyncError> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| SyncError::AppDataDir(e.to_string()))?;
    let shared = data_dir.join("shared");
    ensure_shared_directories(&shared).map_err(SyncError::SharedDirCreation)?;
    Ok(shared)
}

pub fn get_effective_sync<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance: &InstanceConfig,
) -> Result<EffectiveSync, SyncError> {
    let global = load_settings(app)
        .map_err(|e| SyncError::Settings(e.to_string()))?
        .sync;
    Ok(resolve_effective_sync(instance, &global))
}

pub fn mark_instance_synced<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<(), SyncError> {
    let now = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| SyncError::Io(std::io::Error::other(error)))?
        .as_secs();
    crate::app::instances::patch_instance(app, instance_id, |instance| {
        instance.last_synced_at = Some(now);
    })
    .map_err(SyncError::Instances)?;
    Ok(())
}

pub fn sync_before_launch<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance: &InstanceConfig,
    instance_dir: &Path,
) -> Result<(), SyncError> {
    let shared = get_shared_dir(app)?;
    let eff = get_effective_sync(app, instance)?;
    execute_sync_before_launch(&shared, instance_dir, &eff)?;
    mark_instance_synced(app, &instance.id)?;
    Ok(())
}

pub fn sync_after_exit<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance: &InstanceConfig,
    instance_dir: &Path,
) -> Result<(), SyncError> {
    let shared = get_shared_dir(app)?;
    let eff = get_effective_sync(app, instance)?;
    execute_sync_after_exit(&shared, instance_dir, &eff)?;
    mark_instance_synced(app, &instance.id)?;
    Ok(())
}

pub fn export_instance_category_to_shared<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    category: &str,
) -> Result<SyncReport, SyncError> {
    let cat = SyncCategory::parse(category)
        .ok_or_else(|| SyncError::InvalidCategory(category.to_string()))?;
    let shared = get_shared_dir(app)?;
    let instance_dir = ensure_instance_dir(app, instance_id).map_err(SyncError::Instances)?;
    let report = export_instance_category_files(&shared, &instance_dir, cat)?;
    mark_instance_synced(app, instance_id)?;
    Ok(report)
}

pub fn export_instance_to_shared<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<SyncReport, SyncError> {
    export_instance_category_to_shared(app, instance_id, "all")
}

pub fn import_shared_to_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<SyncReport, SyncError> {
    let shared = get_shared_dir(app)?;
    let instance_dir = ensure_instance_dir(app, instance_id).map_err(SyncError::Instances)?;
    let report = import_shared_files_to_instance(&shared, &instance_dir)?;
    mark_instance_synced(app, instance_id)?;
    Ok(report)
}

pub fn get_shared_sync_status<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<SharedSyncStatus, SyncError> {
    let shared = get_shared_dir(app)?;
    Ok(inspect_shared_status(&shared)?)
}

pub fn check_sync_conflict<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<Option<SyncConflictInfo>, SyncError> {
    let instances = crate::app::instances::load_instances(app).map_err(SyncError::Instances)?;
    let instance = instances
        .iter()
        .find(|i| i.id == instance_id)
        .ok_or_else(|| SyncError::InstanceNotFound(instance_id.to_string()))?;

    let eff = get_effective_sync(app, instance)?;
    let shared = get_shared_dir(app)?;
    let instance_dir = ensure_instance_dir(app, instance_id).map_err(SyncError::Instances)?;

    let has_instance_options = file_exists(&instance_dir.join("options.txt"))?;
    let has_instance_servers = file_exists(&instance_dir.join("servers.dat"))?;
    let has_shared_options = file_exists(&shared.join("options.txt"))?;
    let has_shared_servers = file_exists(&shared.join("servers.dat"))?;

    let conflict_inputs = ConflictCheckInputs {
        has_last_synced: instance.last_synced_at.is_some(),
        effective: eff,
        has_instance_options,
        has_instance_servers,
        has_shared_options,
        has_shared_servers,
    };

    if evaluate_conflict(&conflict_inputs) {
        let shared_modified = inspect_shared_status(&shared)?.last_synced_timestamp;
        Ok(Some(SyncConflictInfo {
            instance_id: instance.id.clone(),
            instance_name: instance.name.clone(),
            has_instance_options,
            has_instance_servers,
            has_shared_options,
            has_shared_servers,
            shared_modified,
        }))
    } else {
        Ok(None)
    }
}

pub fn resolve_sync_conflict<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    resolution: &str,
) -> Result<SyncReport, SyncError> {
    let mode = ConflictResolution::parse(resolution)
        .ok_or_else(|| SyncError::InvalidResolution(resolution.to_string()))?;

    match mode {
        ConflictResolution::UseShared => {
            let report = import_shared_to_instance(app, instance_id)?;
            Ok(report)
        }
        ConflictResolution::UseInstance => {
            let report = export_instance_to_shared(app, instance_id)?;
            Ok(report)
        }
        ConflictResolution::DisableSync => {
            crate::app::instances::patch_instance(app, instance_id, |instance| {
                instance.sync_options = Some(false);
                instance.sync_servers = Some(false);
                instance.sync_resource_packs = Some(false);
                instance.sync_command_history = Some(false);
                instance.sync_creative_hotbars = Some(false);
            })
            .map_err(SyncError::Instances)?;
            Ok(SyncReport {
                options_synced: false,
                servers_synced: false,
                resource_packs_count: 0,
                command_history_synced: false,
                creative_hotbars_synced: false,
                message: "Sync disabled for this instance.".to_string(),
            })
        }
    }
}
