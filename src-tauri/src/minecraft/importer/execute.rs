//! Instance import execution and transactional file copying.
use super::models::{ImportError, ImportInstanceOptions, ImportReport};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Runtime;

pub(crate) const IGNORED_ENTRIES: &[&str] = &[
    "assets",
    "libraries",
    "versions",
    "natives",
    "jre",
    "java",
    "runtime",
    "launcher",
    "bin",
    ".fabric",
    ".quilt",
    ".mixin.out",
    ".gradle",
    ".cache",
    "logs",
    ".tlauncher",
    ".curseforge",
    ".modrinth",
    ".atlauncher",
    ".lock",
    "running.lock",
    ".ds_store",
    "thumbs.db",
];

/// Recursively copies a directory tree from `src` to `dst`.
pub(crate) fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
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

/// Executes copy operations from `source_dir` to `dest_dir` according to `options`.
/// Executes copy operations from `source_dir` to `dest_dir` according to `options`.
/// Propagates all I/O errors from required copies instead of silently suppressing them.
/// Returns `(mods_copied, saves_copied)`.
pub(crate) fn copy_instance_contents(
    source_dir: &Path,
    dest_dir: &Path,
    options: &ImportInstanceOptions,
) -> Result<(u32, u32), std::io::Error> {
    let mut mods_copied = 0u32;
    let mut saves_copied = 0u32;
    let mut handled_entries = HashSet::new();

    // 1. Copy mods
    handled_entries.insert("mods".to_string());
    if options.copy_mods {
        let src_mods = source_dir.join("mods");
        let dst_mods = dest_dir.join("mods");
        if src_mods.exists() && src_mods.is_dir() {
            fs::create_dir_all(&dst_mods)?;
            for entry in fs::read_dir(&src_mods)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    let fname = entry.file_name();
                    fs::copy(&path, dst_mods.join(fname))?;
                    mods_copied += 1;
                }
            }
        }
    }

    // 2. Copy config & defaultconfigs
    handled_entries.insert("config".to_string());
    handled_entries.insert("defaultconfigs".to_string());
    if options.copy_configs {
        let src_config = source_dir.join("config");
        let dst_config = dest_dir.join("config");
        if src_config.exists() && src_config.is_dir() {
            copy_dir_all(&src_config, &dst_config)?;
        }
        let src_def = source_dir.join("defaultconfigs");
        let dst_def = dest_dir.join("defaultconfigs");
        if src_def.exists() && src_def.is_dir() {
            copy_dir_all(&src_def, &dst_def)?;
        }
    }

    // 3. Copy saves (worlds)
    handled_entries.insert("saves".to_string());
    if options.copy_saves {
        let src_saves = source_dir.join("saves");
        let dst_saves = dest_dir.join("saves");
        if src_saves.exists() && src_saves.is_dir() {
            fs::create_dir_all(&dst_saves)?;
            for entry in fs::read_dir(&src_saves)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    let fname = entry.file_name();
                    copy_dir_all(&path, &dst_saves.join(fname))?;
                    saves_copied += 1;
                }
            }
        }
    }

    // 4. Copy resourcepacks & shaderpacks
    handled_entries.insert("resourcepacks".to_string());
    handled_entries.insert("shaderpacks".to_string());
    if options.copy_resource_packs {
        let src_rp = source_dir.join("resourcepacks");
        let dst_rp = dest_dir.join("resourcepacks");
        if src_rp.exists() && src_rp.is_dir() {
            copy_dir_all(&src_rp, &dst_rp)?;
        }
        let src_sp = source_dir.join("shaderpacks");
        let dst_sp = dest_dir.join("shaderpacks");
        if src_sp.exists() && src_sp.is_dir() {
            copy_dir_all(&src_sp, &dst_sp)?;
        }
    }

    // 5. Copy options.txt & optionsshaders.txt
    handled_entries.insert("options.txt".to_string());
    handled_entries.insert("optionsshaders.txt".to_string());
    if options.copy_options {
        let src_opt = source_dir.join("options.txt");
        if src_opt.exists() {
            fs::copy(&src_opt, dest_dir.join("options.txt"))?;
        }
        let src_opt_sh = source_dir.join("optionsshaders.txt");
        if src_opt_sh.exists() {
            fs::copy(&src_opt_sh, dest_dir.join("optionsshaders.txt"))?;
        }
    }

    // 6. Copy servers.dat
    handled_entries.insert("servers.dat".to_string());
    if options.copy_servers {
        let src_srv = source_dir.join("servers.dat");
        if src_srv.exists() {
            fs::copy(&src_srv, dest_dir.join("servers.dat"))?;
        }
    }

    // 7. Copy screenshots
    handled_entries.insert("screenshots".to_string());
    if options.copy_screenshots {
        let src_ss = source_dir.join("screenshots");
        let dst_ss = dest_dir.join("screenshots");
        if src_ss.exists() && src_ss.is_dir() {
            copy_dir_all(&src_ss, &dst_ss)?;
        }
    }

    // 8. Copy extra data
    if options.copy_extra_data {
        for entry in fs::read_dir(source_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let name_lower = name.to_lowercase();

            if handled_entries.contains(&name_lower) || IGNORED_ENTRIES.contains(&name_lower.as_str()) {
                continue;
            }

            let path = entry.path();
            let target_path = dest_dir.join(&name);

            if path.is_dir() {
                copy_dir_all(&path, &target_path)?;
            } else if path.is_file() {
                fs::copy(&path, &target_path)?;
            }
        }
    }

    Ok((mods_copied, saves_copied))
}

/// Executes the transactional import of an external instance:
/// 1. Stages all content into a temporary isolated staging directory
/// 2. Validates and renames staging directory to destination instance directory
/// 3. Invokes `publish_metadata` closure, rolling back destination directory if metadata persistence fails
pub fn execute_import_instance<F>(
    instances_dir: &Path,
    options: ImportInstanceOptions,
    publish_metadata: F,
) -> Result<ImportReport, ImportError>
where
    F: FnOnce(crate::minecraft::instance::InstanceConfig) -> Result<(), String>,
{
    let source_dir = PathBuf::from(&options.source_path);
    if !source_dir.exists() {
        return Err(ImportError::SourceNotFound(options.source_path));
    }

    let staging_id = format!(".staging-import-{}", uuid::Uuid::new_v4());
    let staging_dir = instances_dir.join(&staging_id);
    fs::create_dir_all(&staging_dir).map_err(ImportError::Io)?;

    // 1. Stage all requested content into staging_dir
    let (mods_copied, saves_copied) = match copy_instance_contents(&source_dir, &staging_dir, &options) {
        Ok(counts) => counts,
        Err(err) => {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(ImportError::Io(err));
        }
    };

    // Ensure standard subdirectories are initialized in staging
    let _ = fs::create_dir_all(staging_dir.join("mods"));
    let _ = fs::create_dir_all(staging_dir.join("config"));
    let _ = fs::create_dir_all(staging_dir.join("saves"));
    let _ = fs::create_dir_all(staging_dir.join("resourcepacks"));

    // 2. Publish staging directory to target instance ID
    let instance_id = uuid::Uuid::new_v4().to_string();
    let dest_dir = instances_dir.join(&instance_id);

    if let Err(e) = fs::rename(&staging_dir, &dest_dir) {
        let _ = fs::remove_dir_all(&staging_dir);
        let _ = fs::remove_dir_all(&dest_dir);
        return Err(ImportError::Io(e));
    }

    // 3. Atomically publish instance metadata
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let instance = crate::minecraft::instance::InstanceConfig {
        id: instance_id.clone(),
        name: options.name.clone(),
        game_version: options.game_version.clone(),
        loader: options.loader.clone(),
        loader_version: options.loader_version.clone(),
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

    if let Err(e) = publish_metadata(instance) {
        let _ = fs::remove_dir_all(&dest_dir);
        return Err(ImportError::InstanceCreate(e));
    }

    Ok(ImportReport {
        success: true,
        instance_id,
        name: options.name,
        mods_copied,
        saves_copied,
        message: format!(
            "Successfully imported instance with {mods_copied} mods and {saves_copied} worlds."
        ),
    })
}

/// Imports an external instance into Ingot transactionally:
/// Stages and validates all content in an isolated staging folder first,
/// and only publishes the new instance and configuration once all required copies succeed.
pub fn import_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    options: ImportInstanceOptions,
) -> Result<ImportReport, ImportError> {
    let instances_dir = crate::minecraft::instance::get_instances_dir(app)
        .map_err(ImportError::InstanceDir)?;

    execute_import_instance(&instances_dir, options, |instance| {
        let mut instances = crate::minecraft::instance::load_instances(app)?;
        instances.push(instance);
        crate::minecraft::instance::save_instances(app, &instances)
    })
}
