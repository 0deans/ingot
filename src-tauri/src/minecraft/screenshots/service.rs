//! Screenshot indexing, background thumbnail scheduling, and deletion operations.
use super::{
    discovery::{get_bedrock_screenshot_dirs, get_companion_capture_time, scan_screenshot_files},
    models::{ScreenshotError, ScreenshotInfo},
    thumbnails::{
        create_thumbnail, get_thumbnail_cache_dir, get_thumbnail_file_path, SUPPORTED_EXTENSIONS,
    },
};
use crate::minecraft::instance;
use std::fs;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;
use tauri::Runtime;

/// Gathers all screenshots across all Java Edition instances and Bedrock Edition.
pub fn get_all_screenshots<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<ScreenshotInfo>, ScreenshotError> {
    let instances = instance::load_instances(app).map_err(ScreenshotError::LoadInstances)?;
    let mut screenshots = Vec::new();
    let cache_dir = get_thumbnail_cache_dir(app).ok();
    let mut missing_thumbnails = Vec::new();

    // 1. Scan Java Edition instances
    for inst in instances {
        let instance_dir = match instance::get_instance_dir(app, &inst.id) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let screenshots_dir = instance_dir.join("screenshots");
        if !screenshots_dir.exists() || !screenshots_dir.is_dir() {
            continue;
        }

        let entries = match fs::read_dir(&screenshots_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();

            if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
                continue;
            }

            let file_name = entry.file_name().to_string_lossy().to_string();
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let file_size_bytes = metadata.len();

            let modified_at = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            let created_at = metadata
                .created()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(modified_at);

            let id = format!("{}_{}", inst.id, file_name);
            let file_path = path.to_string_lossy().to_string();

            let thumbnail_path = if let Some(ref dir) = cache_dir {
                let thumb_path = get_thumbnail_file_path(dir, &inst.id, &file_name, modified_at);
                if thumb_path.exists() {
                    Some(thumb_path.to_string_lossy().to_string())
                } else {
                    missing_thumbnails.push((path.clone(), thumb_path));
                    None
                }
            } else {
                None
            };

            screenshots.push(ScreenshotInfo {
                id,
                instance_id: inst.id.clone(),
                instance_name: inst.name.clone(),
                file_name,
                file_path,
                thumbnail_path,
                file_size_bytes,
                created_at,
                modified_at,
            });
        }
    }

    // 2. Scan Bedrock Edition screenshots
    let bedrock_dirs = get_bedrock_screenshot_dirs();
    for b_dir in bedrock_dirs {
        let b_files = scan_screenshot_files(&b_dir, 3);
        for path in b_files {
            let file_name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            if file_name.is_empty() {
                continue;
            }

            let metadata = match path.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let file_size_bytes = metadata.len();

            let mut modified_at = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            let mut created_at = metadata
                .created()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(modified_at);

            if let Some(capture_time) = get_companion_capture_time(&path) {
                created_at = capture_time;
                modified_at = capture_time;
            }

            let id = format!("bedrock_{file_name}");
            let file_path = path.to_string_lossy().to_string();

            let thumbnail_path = if let Some(ref dir) = cache_dir {
                let thumb_path = get_thumbnail_file_path(dir, "bedrock", &file_name, modified_at);
                if thumb_path.exists() {
                    Some(thumb_path.to_string_lossy().to_string())
                } else {
                    missing_thumbnails.push((path.clone(), thumb_path));
                    None
                }
            } else {
                None
            };

            screenshots.push(ScreenshotInfo {
                id,
                instance_id: "bedrock".to_string(),
                instance_name: "Bedrock Edition".to_string(),
                file_name,
                file_path,
                thumbnail_path,
                file_size_bytes,
                created_at,
                modified_at,
            });
        }
    }

    // Sort newest first by modified_at
    screenshots.sort_by_key(|a| std::cmp::Reverse(a.modified_at));

    // If only a few thumbnails are missing (e.g. newly taken screenshots),
    // create them immediately so the frontend receives thumbnail paths right away.
    if !missing_thumbnails.is_empty() && missing_thumbnails.len() <= 4 {
        for (source, dest) in &missing_thumbnails {
            let _ = create_thumbnail(source, dest);
        }
        for item in &mut screenshots {
            if item.thumbnail_path.is_none() {
                if let Some(ref dir) = cache_dir {
                    let thumb_path = get_thumbnail_file_path(
                        dir,
                        &item.instance_id,
                        &item.file_name,
                        item.modified_at,
                    );
                    if thumb_path.exists() {
                        item.thumbnail_path = Some(thumb_path.to_string_lossy().to_string());
                    }
                }
            }
        }
    } else if !missing_thumbnails.is_empty() {
        tokio::task::spawn_blocking(move || {
            for (source, dest) in missing_thumbnails {
                if !dest.exists() {
                    let _ = create_thumbnail(&source, &dest);
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
        });
    }

    Ok(screenshots)
}

/// Deletes a specific screenshot file and removes its cached thumbnail and companion files.
pub fn delete_screenshot<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    file_name: &str,
) -> Result<(), ScreenshotError> {
    // Basic sanitization
    if file_name.contains("..") || file_name.contains('/') || file_name.contains('\\') {
        return Err(ScreenshotError::InvalidFilename(file_name.to_string()));
    }

    if instance_id == "bedrock" {
        let dirs = get_bedrock_screenshot_dirs();
        let mut found_path: Option<PathBuf> = None;
        for dir in dirs {
            let files = scan_screenshot_files(&dir, 3);
            if let Some(f) = files.into_iter().find(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n == file_name)
                    .unwrap_or(false)
            }) {
                found_path = Some(f);
                break;
            }
        }

        let file_path =
            found_path.ok_or_else(|| ScreenshotError::NotFound(file_name.to_string()))?;

        let modified_at = file_path
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        if let Ok(cache_dir) = get_thumbnail_cache_dir(app) {
            let thumb_path =
                get_thumbnail_file_path(&cache_dir, instance_id, file_name, modified_at);
            if thumb_path.exists() {
                let _ = fs::remove_file(&thumb_path);
            }
        }

        // Clean up companion files (.json, .mc)
        if let Some(parent) = file_path.parent() {
            if let Some(stem) = file_path.file_stem().and_then(|s| s.to_str()) {
                let json_path = parent.join(format!("{stem}.json"));
                if json_path.exists() {
                    let _ = fs::remove_file(json_path);
                }
                let mc_path = parent.join(format!("{stem}.mc"));
                if mc_path.exists() {
                    let _ = fs::remove_file(mc_path);
                }
            }
        }

        fs::remove_file(&file_path).map_err(ScreenshotError::Io)?;
        return Ok(());
    }

    let instance_dir =
        instance::get_instance_dir(app, instance_id).map_err(ScreenshotError::InstanceDir)?;
    let file_path = instance_dir.join("screenshots").join(file_name);

    if !file_path.exists() {
        return Err(ScreenshotError::NotFound(file_name.to_string()));
    }

    let modified_at = file_path
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    if let Ok(cache_dir) = get_thumbnail_cache_dir(app) {
        let thumb_path = get_thumbnail_file_path(&cache_dir, instance_id, file_name, modified_at);
        if thumb_path.exists() {
            let _ = fs::remove_file(&thumb_path);
        }
    }

    fs::remove_file(&file_path).map_err(ScreenshotError::Io)?;
    Ok(())
}
