use super::models::SharedSyncStatus;
use super::policy::{decide_bidirectional_sync_action, BidirectionalSyncAction};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub fn backup_file_if_exists(file_path: &Path) -> std::io::Result<()> {
    if file_path.exists() && file_path.is_file() {
        let bak_path = PathBuf::from(format!("{}.bak", file_path.to_string_lossy()));
        fs::copy(file_path, bak_path)?;
    }
    Ok(())
}

pub fn sync_file_bidirectional(shared_path: &Path, instance_path: &Path) -> std::io::Result<()> {
    let shared_exists = shared_path.exists();
    let instance_exists = instance_path.exists();

    if !shared_exists && !instance_exists {
        return Ok(());
    }

    let shared_time = if shared_exists {
        fs::metadata(shared_path)
            .and_then(|m| m.modified())
            .unwrap_or(UNIX_EPOCH)
    } else {
        UNIX_EPOCH
    };

    let inst_time = if instance_exists {
        fs::metadata(instance_path)
            .and_then(|m| m.modified())
            .unwrap_or(UNIX_EPOCH)
    } else {
        UNIX_EPOCH
    };

    match decide_bidirectional_sync_action(shared_exists, instance_exists, shared_time, inst_time) {
        BidirectionalSyncAction::CopySharedToInstance { backup_instance } => {
            if backup_instance {
                backup_file_if_exists(instance_path)?;
            }
            fs::copy(shared_path, instance_path)?;
        }
        BidirectionalSyncAction::CopyInstanceToShared { backup_shared } => {
            if backup_shared {
                backup_file_if_exists(shared_path)?;
            }
            fs::copy(instance_path, shared_path)?;
        }
        BidirectionalSyncAction::DoNothing => {}
    }
    Ok(())
}

pub fn sync_resourcepacks_dir(source: &Path, target: &Path) -> std::io::Result<u32> {
    if !source.exists() || !target.exists() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let dest_file = target.join(&file_name);
        if !dest_file.exists() {
            let src_path = entry.path();
            let ft = entry.file_type()?;
            if ft.is_file() {
                fs::copy(&src_path, &dest_file)?;
                count += 1;
            } else if ft.is_dir() {
                copy_dir_all(&src_path, &dest_file)?;
                count += 1;
            }
        }
    }
    Ok(count)
}

pub fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
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

pub fn ensure_shared_directories(shared: &Path) -> std::io::Result<()> {
    if !shared.exists() {
        fs::create_dir_all(shared)?;
    }
    let packs = shared.join("resourcepacks");
    if !packs.exists() {
        let _ = fs::create_dir_all(&packs);
    }
    Ok(())
}

pub fn inspect_shared_status(shared: &Path) -> SharedSyncStatus {
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
    for file_name in [
        "options.txt",
        "servers.dat",
        "command_history.txt",
        "hotbar.nbt",
    ] {
        let p = shared.join(file_name);
        if let Ok(meta) = fs::metadata(&p) {
            if let Ok(mtime) = meta.modified() {
                if let Ok(secs) = mtime.duration_since(UNIX_EPOCH) {
                    let secs_val = secs.as_secs();
                    latest_mod = Some(
                        latest_mod
                            .map(|prev| prev.max(secs_val))
                            .unwrap_or(secs_val),
                    );
                }
            }
        }
    }

    SharedSyncStatus {
        has_options,
        has_servers,
        resource_packs_count: pack_count,
        has_command_history,
        has_creative_hotbars,
        last_synced_timestamp: latest_mod,
    }
}
