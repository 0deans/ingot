use super::error::WorldError;
use super::level_dat::parse_level_dat;
use super::models::WorldBackupInfo;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

pub fn export_world(world_dir: &Path, dest_zip: &Path) -> Result<PathBuf, WorldError> {
    if !world_dir.join("level.dat").is_file() {
        return Err(WorldError::NotFound(
            world_dir.to_string_lossy().to_string(),
        ));
    }

    if let Some(parent) = dest_zip.parent() {
        fs::create_dir_all(parent)?;
    }

    let parent = dest_zip
        .parent()
        .ok_or_else(|| WorldError::Archive("Missing destination parent directory".to_string()))?;

    let tmp = tempfile::NamedTempFile::new_in(parent)?;
    let mut relative_files = Vec::new();
    collect_exportable_files(world_dir, world_dir, &mut relative_files)?;

    let write_res = (|| -> Result<(), WorldError> {
        let mut zip = ZipWriter::new(tmp.reopen()?);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .large_file(true);

        for rel in relative_files {
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            let mut source = File::open(world_dir.join(&rel))?;
            zip.start_file(rel_str, options)?;
            io::copy(&mut source, &mut zip)?;
        }

        zip.finish()?.sync_all()?;
        Ok(())
    })();

    write_res?;
    tmp.persist(dest_zip).map_err(|e| WorldError::Io(e.error))?;
    Ok(dest_zip.to_path_buf())
}

pub fn backup_world(
    instance_dir: &Path,
    world_folder_name: &str,
) -> Result<WorldBackupInfo, WorldError> {
    let world_dir = instance_dir.join("saves").join(world_folder_name);
    if !world_dir.join("level.dat").is_file() {
        return Err(WorldError::NotFound(world_folder_name.to_string()));
    }

    let backups_dir = instance_dir.join("backups");
    fs::create_dir_all(&backups_dir)?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let safe_stem = sanitize_filename(world_folder_name);
    let file_name = format!("{safe_stem}_{timestamp}.zip");
    let dest_path = backups_dir.join(&file_name);

    export_world(&world_dir, &dest_path)?;

    let size_bytes = fs::metadata(&dest_path).map(|m| m.len()).unwrap_or(0);

    Ok(WorldBackupInfo {
        file_name,
        file_path: dest_path.to_string_lossy().to_string(),
        size_bytes,
        created_at: timestamp,
    })
}

pub fn list_world_backups(
    instance_dir: &Path,
    world_folder_name: Option<&str>,
) -> Result<Vec<WorldBackupInfo>, WorldError> {
    let backups_dir = instance_dir.join("backups");
    if !backups_dir.is_dir() {
        return Ok(Vec::new());
    }

    let prefix = world_folder_name.map(|w| format!("{}_", sanitize_filename(w)));
    let mut backups = Vec::new();

    for entry in fs::read_dir(&backups_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("zip") {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().to_string();
        if let Some(p) = &prefix {
            if !file_name.starts_with(p) {
                continue;
            }
        }

        let meta = entry.metadata()?;
        let size_bytes = meta.len();
        let created_at = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        backups.push(WorldBackupInfo {
            file_name,
            file_path: path.to_string_lossy().to_string(),
            size_bytes,
            created_at,
        });
    }

    backups.sort_by_key(|b| std::cmp::Reverse(b.created_at));
    Ok(backups)
}

pub fn delete_world_backup(instance_dir: &Path, file_name: &str) -> Result<(), WorldError> {
    let safe_name = Path::new(file_name)
        .file_name()
        .ok_or_else(|| WorldError::InvalidFolderName(file_name.to_string()))?;
    let path = instance_dir.join("backups").join(safe_name);
    if path.is_file() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn import_world(saves_dir: &Path, source_zip: &Path) -> Result<String, WorldError> {
    if !saves_dir.is_dir() {
        fs::create_dir_all(saves_dir)?;
    }

    let staging_id = uuid::Uuid::new_v4();
    let staging_dir = saves_dir.join(format!(".staging_import_{staging_id}"));
    fs::create_dir_all(&staging_dir)?;

    let extract_result = (|| -> Result<String, WorldError> {
        let zip_file = File::open(source_zip)?;
        let mut archive = ZipArchive::new(zip_file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let enclosed = match file.enclosed_name() {
                Some(p) => p.to_owned(),
                None => {
                    return Err(WorldError::Archive(
                        "Zip contains invalid or traversing path".to_string(),
                    ))
                }
            };

            let out_path = staging_dir.join(enclosed);
            if file.is_dir() {
                fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut out_file = File::create(&out_path)?;
                io::copy(&mut file, &mut out_file)?;
            }
        }

        // Locate world directory inside staging
        let (world_source_dir, fallback_stem) = find_world_root(&staging_dir)?;
        let level_dat_path = world_source_dir.join("level.dat");
        let level_bytes = fs::read(&level_dat_path)?;
        let parsed = parse_level_dat(&level_bytes, &fallback_stem).unwrap_or_else(|_| {
            super::level_dat::ParsedLevelDat {
                display_name: fallback_stem.clone(),
                game_mode: "Survival".to_string(),
                is_hardcore: false,
                seed: None,
                last_played: None,
                version_name: None,
            }
        });

        let safe_name = sanitize_filename(&parsed.display_name);
        let final_folder_name = disambiguate_folder_name(saves_dir, &safe_name);
        let final_dest = saves_dir.join(&final_folder_name);

        fs::rename(&world_source_dir, &final_dest)?;
        Ok(final_folder_name)
    })();

    // Always clean up staging directory
    let _ = fs::remove_dir_all(&staging_dir);
    extract_result
}

fn find_world_root(staging_dir: &Path) -> Result<(PathBuf, String), WorldError> {
    if staging_dir.join("level.dat").is_file() {
        return Ok((staging_dir.to_path_buf(), "Imported World".to_string()));
    }

    for entry in fs::read_dir(staging_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.join("level.dat").is_file() {
            let name = entry.file_name().to_string_lossy().to_string();
            return Ok((path, name));
        }
    }

    Err(WorldError::Archive(
        "No level.dat file found in the archive".to_string(),
    ))
}

fn disambiguate_folder_name(saves_dir: &Path, base: &str) -> String {
    let mut candidate = base.to_string();
    let mut counter = 1;
    while saves_dir.join(&candidate).exists() {
        candidate = format!("{base} ({counter})");
        counter += 1;
    }
    candidate
}

pub fn sanitize_filename(name: &str) -> String {
    let filtered: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = filtered.trim();
    if trimmed.is_empty() {
        "World".to_string()
    } else {
        trimmed.to_string()
    }
}

fn collect_exportable_files(
    current: &Path,
    root: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), WorldError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "session.lock"
            || name == ".ingot-trash"
            || name.starts_with(".staging_")
            || name.starts_with(".qp_")
        {
            continue;
        }

        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_exportable_files(&path, root, out)?;
        } else if file_type.is_file() {
            if let Ok(rel) = path.strip_prefix(root) {
                out.push(rel.to_path_buf());
            }
        }
    }
    Ok(())
}
