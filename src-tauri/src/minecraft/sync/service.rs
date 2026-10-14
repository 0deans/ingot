use super::execute::{
    backup_file_if_exists, ensure_shared_directories, inspect_shared_status,
    sync_file_bidirectional, sync_resourcepacks_dir,
};
use super::models::{
    ConflictResolution, SharedSyncStatus, SyncCategory, SyncConflictInfo, SyncError, SyncReport,
};
use super::policy::{
    evaluate_conflict, resolve_effective_sync, ConflictCheckInputs, EffectiveSync,
};
use crate::minecraft::instance::{get_instance_dir, InstanceConfig};
use crate::system::load_settings;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tauri::{Manager, Runtime};

pub fn execute_sync_before_launch(
    shared: &Path,
    instance_dir: &Path,
    eff: &EffectiveSync,
) -> Result<(), SyncError> {
    if eff.options {
        sync_file_bidirectional(
            &shared.join("options.txt"),
            &instance_dir.join("options.txt"),
        )?;
    }
    if eff.servers {
        sync_file_bidirectional(
            &shared.join("servers.dat"),
            &instance_dir.join("servers.dat"),
        )?;
    }
    if eff.command_history {
        sync_file_bidirectional(
            &shared.join("command_history.txt"),
            &instance_dir.join("command_history.txt"),
        )?;
    }
    if eff.creative_hotbars {
        sync_file_bidirectional(&shared.join("hotbar.nbt"), &instance_dir.join("hotbar.nbt"))?;
    }
    if eff.resource_packs {
        let shared_packs = shared.join("resourcepacks");
        let instance_packs = instance_dir.join("resourcepacks");
        fs::create_dir_all(&instance_packs)?;
        sync_resourcepacks_dir(&shared_packs, &instance_packs)?;
    }
    Ok(())
}

pub fn execute_sync_after_exit(
    shared: &Path,
    instance_dir: &Path,
    eff: &EffectiveSync,
) -> Result<(), SyncError> {
    if eff.options {
        let src = instance_dir.join("options.txt");
        if src.exists() {
            let dst = shared.join("options.txt");
            backup_file_if_exists(&dst)?;
            fs::copy(&src, dst)?;
        }
    }
    if eff.servers {
        let src = instance_dir.join("servers.dat");
        if src.exists() {
            let dst = shared.join("servers.dat");
            backup_file_if_exists(&dst)?;
            fs::copy(&src, dst)?;
        }
    }
    if eff.command_history {
        let src = instance_dir.join("command_history.txt");
        if src.exists() {
            let dst = shared.join("command_history.txt");
            backup_file_if_exists(&dst)?;
            fs::copy(&src, dst)?;
        }
    }
    if eff.creative_hotbars {
        let src = instance_dir.join("hotbar.nbt");
        if src.exists() {
            let dst = shared.join("hotbar.nbt");
            backup_file_if_exists(&dst)?;
            fs::copy(&src, dst)?;
        }
    }
    if eff.resource_packs {
        let shared_packs = shared.join("resourcepacks");
        let instance_packs = instance_dir.join("resourcepacks");
        fs::create_dir_all(&shared_packs)?;
        sync_resourcepacks_dir(&instance_packs, &shared_packs)?;
    }
    Ok(())
}

pub fn export_instance_category_files(
    shared: &Path,
    instance_dir: &Path,
    category: SyncCategory,
) -> Result<SyncReport, SyncError> {
    let mut report = SyncReport {
        options_synced: false,
        servers_synced: false,
        resource_packs_count: 0,
        command_history_synced: false,
        creative_hotbars_synced: false,
        message: String::new(),
    };

    let do_all = category == SyncCategory::All;

    if do_all || category == SyncCategory::Options {
        let opt_src = instance_dir.join("options.txt");
        if opt_src.exists() {
            let dst = shared.join("options.txt");
            backup_file_if_exists(&dst)?;
            fs::copy(&opt_src, dst)?;
            report.options_synced = true;
        }
    }

    if do_all || category == SyncCategory::Servers {
        let srv_src = instance_dir.join("servers.dat");
        if srv_src.exists() {
            let dst = shared.join("servers.dat");
            backup_file_if_exists(&dst)?;
            fs::copy(&srv_src, dst)?;
            report.servers_synced = true;
        }
    }

    if do_all || category == SyncCategory::CommandHistory {
        let cmd_src = instance_dir.join("command_history.txt");
        if cmd_src.exists() {
            let dst = shared.join("command_history.txt");
            backup_file_if_exists(&dst)?;
            fs::copy(&cmd_src, dst)?;
            report.command_history_synced = true;
        }
    }

    if do_all || category == SyncCategory::Hotbar {
        let hot_src = instance_dir.join("hotbar.nbt");
        if hot_src.exists() {
            let dst = shared.join("hotbar.nbt");
            backup_file_if_exists(&dst)?;
            fs::copy(&hot_src, dst)?;
            report.creative_hotbars_synced = true;
        }
    }

    if do_all || category == SyncCategory::ResourcePacks {
        let inst_packs = instance_dir.join("resourcepacks");
        if inst_packs.exists() {
            let shared_packs = shared.join("resourcepacks");
            fs::create_dir_all(&shared_packs)?;
            report.resource_packs_count = sync_resourcepacks_dir(&inst_packs, &shared_packs)?;
        }
    }

    report.message = format!(
        "Successfully exported {} to shared sync storage.",
        category.as_str()
    );
    Ok(report)
}

pub fn import_shared_files_to_instance(
    shared: &Path,
    instance_dir: &Path,
) -> Result<SyncReport, SyncError> {
    let mut report = SyncReport {
        options_synced: false,
        servers_synced: false,
        resource_packs_count: 0,
        command_history_synced: false,
        creative_hotbars_synced: false,
        message: "Successfully imported shared sync data into instance.".to_string(),
    };

    let opt_shared = shared.join("options.txt");
    if opt_shared.exists() {
        let dst = instance_dir.join("options.txt");
        backup_file_if_exists(&dst)?;
        fs::copy(&opt_shared, dst)?;
        report.options_synced = true;
    }

    let srv_shared = shared.join("servers.dat");
    if srv_shared.exists() {
        let dst = instance_dir.join("servers.dat");
        backup_file_if_exists(&dst)?;
        fs::copy(&srv_shared, dst)?;
        report.servers_synced = true;
    }

    let cmd_shared = shared.join("command_history.txt");
    if cmd_shared.exists() {
        let dst = instance_dir.join("command_history.txt");
        backup_file_if_exists(&dst)?;
        fs::copy(&cmd_shared, dst)?;
        report.command_history_synced = true;
    }

    let hot_shared = shared.join("hotbar.nbt");
    if hot_shared.exists() {
        let dst = instance_dir.join("hotbar.nbt");
        backup_file_if_exists(&dst)?;
        fs::copy(&hot_shared, dst)?;
        report.creative_hotbars_synced = true;
    }

    let shared_packs = shared.join("resourcepacks");
    if shared_packs.exists() {
        let inst_packs = instance_dir.join("resourcepacks");
        fs::create_dir_all(&inst_packs)?;
        report.resource_packs_count = sync_resourcepacks_dir(&shared_packs, &inst_packs)?;
    }

    Ok(report)
}

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
    let mut instances = crate::minecraft::instance::load_instances(app)
        .map_err(SyncError::Instances)?;
    let now = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Some(inst) = instances.iter_mut().find(|i| i.id == instance_id) {
        inst.last_synced_at = Some(now);
        crate::minecraft::instance::save_instances(app, &instances)
            .map_err(SyncError::Instances)?;
    }
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
    let instance_dir = get_instance_dir(app, instance_id).map_err(SyncError::Instances)?;
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
    let instance_dir = get_instance_dir(app, instance_id).map_err(SyncError::Instances)?;
    let report = import_shared_files_to_instance(&shared, &instance_dir)?;
    mark_instance_synced(app, instance_id)?;
    Ok(report)
}

pub fn get_shared_sync_status<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<SharedSyncStatus, SyncError> {
    let shared = get_shared_dir(app)?;
    Ok(inspect_shared_status(&shared))
}

pub fn check_sync_conflict<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<Option<SyncConflictInfo>, SyncError> {
    let instances =
        crate::minecraft::instance::load_instances(app).map_err(SyncError::Instances)?;
    let instance = instances
        .iter()
        .find(|i| i.id == instance_id)
        .ok_or_else(|| SyncError::InstanceNotFound(instance_id.to_string()))?;

    let eff = get_effective_sync(app, instance)?;
    let shared = get_shared_dir(app)?;
    let instance_dir = get_instance_dir(app, instance_id).map_err(SyncError::Instances)?;

    let has_instance_options = instance_dir.join("options.txt").exists();
    let has_instance_servers = instance_dir.join("servers.dat").exists();
    let has_shared_options = shared.join("options.txt").exists();
    let has_shared_servers = shared.join("servers.dat").exists();

    let conflict_inputs = ConflictCheckInputs {
        has_last_synced: instance.last_synced_at.is_some(),
        effective: eff,
        has_instance_options,
        has_instance_servers,
        has_shared_options,
        has_shared_servers,
    };

    if evaluate_conflict(&conflict_inputs) {
        let shared_modified = inspect_shared_status(&shared).last_synced_timestamp;
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
            mark_instance_synced(app, instance_id)?;
            Ok(report)
        }
        ConflictResolution::UseInstance => {
            let report = export_instance_to_shared(app, instance_id)?;
            mark_instance_synced(app, instance_id)?;
            Ok(report)
        }
        ConflictResolution::DisableSync => {
            let mut instances =
                crate::minecraft::instance::load_instances(app).map_err(SyncError::Instances)?;
            if let Some(inst) = instances.iter_mut().find(|i| i.id == instance_id) {
                inst.sync_options = Some(false);
                inst.sync_servers = Some(false);
                inst.sync_resource_packs = Some(false);
                inst.sync_command_history = Some(false);
                inst.sync_creative_hotbars = Some(false);
                crate::minecraft::instance::save_instances(app, &instances)
                    .map_err(SyncError::Instances)?;
            }
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
