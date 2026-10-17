use super::models::SharedSyncStatus;
use super::policy::{decide_bidirectional_sync_action, BidirectionalSyncAction};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub fn backup_file_if_exists(file_path: &Path) -> std::io::Result<()> {
    if file_exists(file_path)? {
        let bak_path = PathBuf::from(format!("{}.bak", file_path.to_string_lossy()));
        copy_file_atomic(file_path, &bak_path)?;
    }
    Ok(())
}

pub fn sync_file_bidirectional(shared_path: &Path, instance_path: &Path) -> std::io::Result<()> {
    let shared_exists = file_exists(shared_path)?;
    let instance_exists = file_exists(instance_path)?;

    if !shared_exists && !instance_exists {
        return Ok(());
    }

    let shared_time = if shared_exists {
        fs::metadata(shared_path).and_then(|m| m.modified())?
    } else {
        UNIX_EPOCH
    };

    let inst_time = if instance_exists {
        fs::metadata(instance_path).and_then(|m| m.modified())?
    } else {
        UNIX_EPOCH
    };

    match decide_bidirectional_sync_action(shared_exists, instance_exists, shared_time, inst_time) {
        BidirectionalSyncAction::CopySharedToInstance { backup_instance } => {
            if backup_instance {
                backup_file_if_exists(instance_path)?;
            }
            copy_file_atomic(shared_path, instance_path)?;
        }
        BidirectionalSyncAction::CopyInstanceToShared { backup_shared } => {
            if backup_shared {
                backup_file_if_exists(shared_path)?;
            }
            copy_file_atomic(instance_path, shared_path)?;
        }
        BidirectionalSyncAction::DoNothing => {}
    }
    Ok(())
}

pub fn sync_resourcepacks_dir(source: &Path, target: &Path) -> std::io::Result<u32> {
    match fs::metadata(source) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
    }
    fs::create_dir_all(target)?;
    let mut count = 0;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let name = file_name.to_str().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid resource-pack name",
            )
        })?;
        if crate::app::safe_enclosed_filename(source, name).is_none()
            || crate::app::safe_enclosed_filename(target, name).is_none()
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Linked resource packs are unsupported",
            ));
        }
        let dest_file = target.join(&file_name);
        if !dest_file.exists() {
            let src_path = entry.path();
            let ft = entry.file_type()?;
            if ft.is_file() {
                copy_file_atomic(&src_path, &dest_file)?;
                count += 1;
            } else if ft.is_dir() {
                let staging = tempfile::tempdir_in(target)?;
                copy_dir_all(&src_path, staging.path())?;
                fs::rename(staging.path(), &dest_file)?;
                count += 1;
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Unsupported resource-pack entry",
                ));
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
        let name = entry.file_name();
        let name = name.to_str().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid resource-pack name",
            )
        })?;
        if crate::app::safe_enclosed_filename(src, name).is_none()
            || (!ty.is_dir() && !ty.is_file())
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Resource-pack links and special entries are unsupported",
            ));
        }
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            copy_file_atomic(&entry.path(), &dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn ensure_shared_directories(shared: &Path) -> std::io::Result<()> {
    fs::create_dir_all(shared)?;
    fs::create_dir_all(shared.join("resourcepacks"))?;
    Ok(())
}

pub(crate) fn file_exists(path: &Path) -> std::io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(true),
        Ok(_) => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Expected a regular sync file",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

pub(super) fn copy_file_atomic(source: &Path, destination: &Path) -> std::io::Result<()> {
    for path in [source, destination] {
        let parent = path.parent().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Sync file has no parent")
        })?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid sync filename")
            })?;
        if crate::app::safe_enclosed_filename(parent, name).is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Linked sync files are unsupported",
            ));
        }
    }
    let parent = destination.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Sync destination has no parent",
        )
    })?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    let mut input = fs::File::open(source)?;
    std::io::copy(&mut input, &mut staged)?;
    staged.as_file().sync_all()?;
    staged.persist(destination).map_err(|error| error.error)?;
    Ok(())
}

pub fn inspect_shared_status(shared: &Path) -> std::io::Result<SharedSyncStatus> {
    let has_options = file_exists(&shared.join("options.txt"))?;
    let has_servers = file_exists(&shared.join("servers.dat"))?;
    let has_command_history = file_exists(&shared.join("command_history.txt"))?;
    let has_creative_hotbars = file_exists(&shared.join("hotbar.nbt"))?;
    let shared_packs = shared.join("resourcepacks");
    let mut pack_count = 0u32;
    match fs::read_dir(shared_packs) {
        Ok(entries) => {
            for entry in entries {
                entry?;
                pack_count = pack_count.checked_add(1).ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "Too many resource packs")
                })?;
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let mut latest_mod: Option<u64> = None;
    for name in [
        "options.txt",
        "servers.dat",
        "command_history.txt",
        "hotbar.nbt",
    ] {
        let path = shared.join(name);
        if file_exists(&path)? {
            let modified = fs::metadata(&path)?.modified()?;
            let seconds = modified
                .duration_since(UNIX_EPOCH)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?
                .as_secs();
            latest_mod = Some(
                latest_mod
                    .map(|previous| previous.max(seconds))
                    .unwrap_or(seconds),
            );
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
