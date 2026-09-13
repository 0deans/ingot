use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tauri::{Manager, Runtime};
use crate::minecraft::instance::{get_instance_dir, InstanceConfig};
use crate::system::load_settings;

#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub options_synced: bool,
    pub servers_synced: bool,
    pub resource_packs_count: u32,
    pub command_history_synced: bool,
    pub creative_hotbars_synced: bool,
    pub message: String,
}

#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct SharedSyncStatus {
    pub has_options: bool,
    pub has_servers: bool,
    pub resource_packs_count: u32,
    pub has_command_history: bool,
    pub has_creative_hotbars: bool,
    pub last_synced_timestamp: Option<u64>,
}

#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct SyncConflictInfo {
    pub instance_id: String,
    pub instance_name: String,
    pub has_instance_options: bool,
    pub has_instance_servers: bool,
    pub has_shared_options: bool,
    pub has_shared_servers: bool,
    pub shared_modified: Option<u64>,
}

pub fn get_shared_dir<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {e}"))?;
    let shared = data_dir.join("shared");
    if !shared.exists() {
        fs::create_dir_all(&shared)
            .map_err(|e| format!("Failed to create shared sync directory: {e}"))?;
    }
    let packs = shared.join("resourcepacks");
    if !packs.exists() {
        let _ = fs::create_dir_all(&packs);
    }
    Ok(shared)
}

struct EffectiveSync {
    options: bool,
    servers: bool,
    resource_packs: bool,
    command_history: bool,
    creative_hotbars: bool,
}

fn get_effective_sync<R: Runtime>(app: &tauri::AppHandle<R>, instance: &InstanceConfig) -> EffectiveSync {
    let global = load_settings(app).sync;
    EffectiveSync {
        options: instance.sync_options.unwrap_or(global.sync_options),
        servers: instance.sync_servers.unwrap_or(global.sync_servers),
        resource_packs: instance.sync_resource_packs.unwrap_or(global.sync_resource_packs),
        command_history: instance.sync_command_history.unwrap_or(global.sync_command_history),
        creative_hotbars: instance.sync_creative_hotbars.unwrap_or(global.sync_creative_hotbars),
    }
}

pub fn backup_file_if_exists(file_path: &Path) {
    if file_path.exists() && file_path.is_file() {
        let bak_path = PathBuf::from(format!("{}.bak", file_path.to_string_lossy()));
        let _ = fs::copy(file_path, bak_path);
    }
}

pub fn mark_instance_synced<R: Runtime>(app: &tauri::AppHandle<R>, instance_id: &str) {
    if let Ok(mut instances) = crate::minecraft::instance::load_instances(app) {
        let now = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if let Some(inst) = instances.iter_mut().find(|i| i.id == instance_id) {
            inst.last_synced_at = Some(now);
            let _ = crate::minecraft::instance::save_instances(app, &instances);
        }
    }
}

fn sync_file_bidirectional(shared_path: &Path, instance_path: &Path) {
    if shared_path.exists() && !instance_path.exists() {
        let _ = fs::copy(shared_path, instance_path);
    } else if !shared_path.exists() && instance_path.exists() {
        let _ = fs::copy(instance_path, shared_path);
    } else if shared_path.exists() && instance_path.exists() {
        let shared_time = fs::metadata(shared_path).and_then(|m| m.modified()).unwrap_or(UNIX_EPOCH);
        let inst_time = fs::metadata(instance_path).and_then(|m| m.modified()).unwrap_or(UNIX_EPOCH);
        if shared_time > inst_time {
            backup_file_if_exists(instance_path);
            let _ = fs::copy(shared_path, instance_path);
        } else if inst_time > shared_time {
            backup_file_if_exists(shared_path);
            let _ = fs::copy(instance_path, shared_path);
        }
    }
}

fn sync_resourcepacks_dir(source: &Path, target: &Path) -> u32 {
    if !source.exists() || !target.exists() {
        return 0;
    }
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(source) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let dest_file = target.join(&file_name);
            if !dest_file.exists() {
                let src_path = entry.path();
                if src_path.is_file() {
                    if fs::copy(&src_path, &dest_file).is_ok() {
                        count += 1;
                    }
                } else if src_path.is_dir() {
                    // Copy folder if not exists
                    let _ = copy_dir_all(&src_path, &dest_file);
                    count += 1;
                }
            }
        }
    }
    count
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn sync_before_launch<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance: &InstanceConfig,
    instance_dir: &Path,
) -> Result<(), String> {
    let shared = get_shared_dir(app)?;
    let eff = get_effective_sync(app, instance);

    if eff.options {
        sync_file_bidirectional(&shared.join("options.txt"), &instance_dir.join("options.txt"));
    }
    if eff.servers {
        sync_file_bidirectional(&shared.join("servers.dat"), &instance_dir.join("servers.dat"));
    }
    if eff.command_history {
        sync_file_bidirectional(&shared.join("command_history.txt"), &instance_dir.join("command_history.txt"));
    }
    if eff.creative_hotbars {
        sync_file_bidirectional(&shared.join("hotbar.nbt"), &instance_dir.join("hotbar.nbt"));
    }
    if eff.resource_packs {
        let shared_packs = shared.join("resourcepacks");
        let instance_packs = instance_dir.join("resourcepacks");
        let _ = fs::create_dir_all(&instance_packs);
        sync_resourcepacks_dir(&shared_packs, &instance_packs);
    }

    mark_instance_synced(app, &instance.id);
    Ok(())
}

pub fn sync_after_exit<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance: &InstanceConfig,
    instance_dir: &Path,
) -> Result<(), String> {
    let shared = get_shared_dir(app)?;
    let eff = get_effective_sync(app, instance);

    if eff.options {
        let src = instance_dir.join("options.txt");
        if src.exists() {
            let dst = shared.join("options.txt");
            backup_file_if_exists(&dst);
            let _ = fs::copy(&src, dst);
        }
    }
    if eff.servers {
        let src = instance_dir.join("servers.dat");
        if src.exists() {
            let dst = shared.join("servers.dat");
            backup_file_if_exists(&dst);
            let _ = fs::copy(&src, dst);
        }
    }
    if eff.command_history {
        let src = instance_dir.join("command_history.txt");
        if src.exists() {
            let dst = shared.join("command_history.txt");
            backup_file_if_exists(&dst);
            let _ = fs::copy(&src, dst);
        }
    }
    if eff.creative_hotbars {
        let src = instance_dir.join("hotbar.nbt");
        if src.exists() {
            let dst = shared.join("hotbar.nbt");
            backup_file_if_exists(&dst);
            let _ = fs::copy(&src, dst);
        }
    }
    if eff.resource_packs {
        let shared_packs = shared.join("resourcepacks");
        let instance_packs = instance_dir.join("resourcepacks");
        let _ = fs::create_dir_all(&shared_packs);
        sync_resourcepacks_dir(&instance_packs, &shared_packs);
    }

    mark_instance_synced(app, &instance.id);
    Ok(())
}

pub fn export_instance_category_to_shared<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    category: &str,
) -> Result<SyncReport, String> {
    let shared = get_shared_dir(app)?;
    let instance_dir = get_instance_dir(app, instance_id)?;

    let mut report = SyncReport {
        options_synced: false,
        servers_synced: false,
        resource_packs_count: 0,
        command_history_synced: false,
        creative_hotbars_synced: false,
        message: String::new(),
    };

    let do_all = category == "all";

    if do_all || category == "options" {
        let opt_src = instance_dir.join("options.txt");
        if opt_src.exists() {
            let dst = shared.join("options.txt");
            backup_file_if_exists(&dst);
            fs::copy(&opt_src, dst).map_err(|e| e.to_string())?;
            report.options_synced = true;
        }
    }

    if do_all || category == "servers" {
        let srv_src = instance_dir.join("servers.dat");
        if srv_src.exists() {
            let dst = shared.join("servers.dat");
            backup_file_if_exists(&dst);
            fs::copy(&srv_src, dst).map_err(|e| e.to_string())?;
            report.servers_synced = true;
        }
    }

    if do_all || category == "command_history" {
        let cmd_src = instance_dir.join("command_history.txt");
        if cmd_src.exists() {
            let dst = shared.join("command_history.txt");
            backup_file_if_exists(&dst);
            fs::copy(&cmd_src, dst).map_err(|e| e.to_string())?;
            report.command_history_synced = true;
        }
    }

    if do_all || category == "hotbar" {
        let hot_src = instance_dir.join("hotbar.nbt");
        if hot_src.exists() {
            let dst = shared.join("hotbar.nbt");
            backup_file_if_exists(&dst);
            fs::copy(&hot_src, dst).map_err(|e| e.to_string())?;
            report.creative_hotbars_synced = true;
        }
    }

    if do_all || category == "resourcepacks" {
        let inst_packs = instance_dir.join("resourcepacks");
        if inst_packs.exists() {
            let shared_packs = shared.join("resourcepacks");
            let _ = fs::create_dir_all(&shared_packs);
            report.resource_packs_count = sync_resourcepacks_dir(&inst_packs, &shared_packs);
        }
    }

    mark_instance_synced(app, instance_id);
    report.message = format!("Successfully exported {category} to shared sync storage.");
    Ok(report)
}

pub fn export_instance_to_shared<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<SyncReport, String> {
    export_instance_category_to_shared(app, instance_id, "all")
}

pub fn import_shared_to_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<SyncReport, String> {
    let shared = get_shared_dir(app)?;
    let instance_dir = get_instance_dir(app, instance_id)?;

    let mut report = SyncReport {
        options_synced: false,
        servers_synced: false,
        resource_packs_count: 0,
        command_history_synced: false,
        creative_hotbars_synced: false,
        message: String::new(),
    };

    let opt_shared = shared.join("options.txt");
    if opt_shared.exists() {
        let dst = instance_dir.join("options.txt");
        backup_file_if_exists(&dst);
        fs::copy(&opt_shared, dst).map_err(|e| e.to_string())?;
        report.options_synced = true;
    }

    let srv_shared = shared.join("servers.dat");
    if srv_shared.exists() {
        let dst = instance_dir.join("servers.dat");
        backup_file_if_exists(&dst);
        fs::copy(&srv_shared, dst).map_err(|e| e.to_string())?;
        report.servers_synced = true;
    }

    let cmd_shared = shared.join("command_history.txt");
    if cmd_shared.exists() {
        let dst = instance_dir.join("command_history.txt");
        backup_file_if_exists(&dst);
        fs::copy(&cmd_shared, dst).map_err(|e| e.to_string())?;
        report.command_history_synced = true;
    }

    let hot_shared = shared.join("hotbar.nbt");
    if hot_shared.exists() {
        let dst = instance_dir.join("hotbar.nbt");
        backup_file_if_exists(&dst);
        fs::copy(&hot_shared, dst).map_err(|e| e.to_string())?;
        report.creative_hotbars_synced = true;
    }

    let shared_packs = shared.join("resourcepacks");
    if shared_packs.exists() {
        let inst_packs = instance_dir.join("resourcepacks");
        let _ = fs::create_dir_all(&inst_packs);
        report.resource_packs_count = sync_resourcepacks_dir(&shared_packs, &inst_packs);
    }

    mark_instance_synced(app, instance_id);
    report.message = "Successfully imported shared sync data into instance.".to_string();
    Ok(report)
}

pub fn get_shared_sync_status<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<SharedSyncStatus, String> {
    let shared = get_shared_dir(app)?;

    let has_options = shared.join("options.txt").exists();
    let has_servers = shared.join("servers.dat").exists();
    let has_command_history = shared.join("command_history.txt").exists();
    let has_creative_hotbars = shared.join("hotbar.nbt").exists();

    let mut pack_count = 0;
    let shared_packs = shared.join("resourcepacks");
    if let Ok(entries) = fs::read_dir(&shared_packs) {
        pack_count = entries.flatten().count() as u32;
    }

    let mut latest_mod: Option<u64> = None;
    for file_name in ["options.txt", "servers.dat", "command_history.txt", "hotbar.nbt"] {
        let p = shared.join(file_name);
        if let Ok(meta) = fs::metadata(&p) {
            if let Ok(mtime) = meta.modified() {
                if let Ok(secs) = mtime.duration_since(UNIX_EPOCH) {
                    latest_mod = Some(latest_mod.map(|prev| prev.max(secs.as_secs())).unwrap_or(secs.as_secs()));
                }
            }
        }
    }

    Ok(SharedSyncStatus {
        has_options,
        has_servers,
        resource_packs_count: pack_count,
        has_command_history,
        has_creative_hotbars,
        last_synced_timestamp: latest_mod,
    })
}

pub fn check_sync_conflict<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<Option<SyncConflictInfo>, String> {
    let instances = crate::minecraft::instance::load_instances(app)?;
    let instance = instances
        .iter()
        .find(|i| i.id == instance_id)
        .ok_or_else(|| format!("Instance not found: {instance_id}"))?;

    let eff = get_effective_sync(app, instance);
    if !eff.options && !eff.servers && !eff.resource_packs && !eff.command_history && !eff.creative_hotbars {
        return Ok(None);
    }

    if instance.last_synced_at.is_some() {
        return Ok(None);
    }

    let shared = get_shared_dir(app)?;
    let instance_dir = get_instance_dir(app, instance_id)?;

    let has_instance_options = instance_dir.join("options.txt").exists();
    let has_instance_servers = instance_dir.join("servers.dat").exists();

    let has_shared_options = shared.join("options.txt").exists();
    let has_shared_servers = shared.join("servers.dat").exists();

    let options_conflict = eff.options && has_instance_options && has_shared_options;
    let servers_conflict = eff.servers && has_instance_servers && has_shared_servers;

    if options_conflict || servers_conflict {
        let shared_modified = get_shared_sync_status(app)?.last_synced_timestamp;
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
) -> Result<SyncReport, String> {
    match resolution {
        "use_shared" => {
            let report = import_shared_to_instance(app, instance_id)?;
            mark_instance_synced(app, instance_id);
            Ok(report)
        }
        "use_instance" => {
            let report = export_instance_to_shared(app, instance_id)?;
            mark_instance_synced(app, instance_id);
            Ok(report)
        }
        "disable_sync" => {
            let mut instances = crate::minecraft::instance::load_instances(app)?;
            if let Some(inst) = instances.iter_mut().find(|i| i.id == instance_id) {
                inst.sync_options = Some(false);
                inst.sync_servers = Some(false);
                inst.sync_resource_packs = Some(false);
                inst.sync_command_history = Some(false);
                inst.sync_creative_hotbars = Some(false);
                crate::minecraft::instance::save_instances(app, &instances)?;
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
        other => Err(format!("Unknown conflict resolution: {other}")),
    }
}
