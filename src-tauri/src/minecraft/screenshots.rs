use crate::minecraft::instance;
use md5::Digest;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tauri::{Manager, Runtime};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotInfo {
    pub id: String,
    pub instance_id: String,
    pub instance_name: String,
    pub file_name: String,
    pub file_path: String,
    pub thumbnail_path: Option<String>,
    pub file_size_bytes: u64,
    pub created_at: u64,
    pub modified_at: u64,
}

const SUPPORTED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];

pub fn get_thumbnail_cache_dir<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("screenshot_thumbnails");

    if !cache_dir.exists() {
        let _ = fs::create_dir_all(&cache_dir);
    }
    Ok(cache_dir)
}

pub fn get_thumbnail_file_path(
    cache_dir: &Path,
    instance_id: &str,
    file_name: &str,
    modified_at: u64,
) -> PathBuf {
    let key = format!("{}:{}:{}", instance_id, file_name, modified_at);
    let hash = format!("{:x}", md5::Md5::digest(key.as_bytes()));
    cache_dir.join(format!("{}.jpg", hash))
}

pub fn create_thumbnail(source_path: &Path, dest_path: &Path) -> Result<(), String> {
    if dest_path.exists() {
        return Ok(());
    }

    let img = image::open(source_path)
        .map_err(|e| format!("Failed to open screenshot {}: {e}", source_path.display()))?;

    let thumb = img.thumbnail(480, 270);

    if let Some(parent) = dest_path.parent() {
        if !parent.exists() {
            let _ = fs::create_dir_all(parent);
        }
    }

    thumb
        .save_with_format(dest_path, image::ImageFormat::Jpeg)
        .map_err(|e| format!("Failed to save thumbnail {}: {e}", dest_path.display()))?;

    Ok(())
}

pub fn get_or_create_thumbnail<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    file_name: &str,
    file_path: &str,
    modified_at: u64,
) -> Result<String, String> {
    let cache_dir = get_thumbnail_cache_dir(app)?;
    let thumb_path = get_thumbnail_file_path(&cache_dir, instance_id, file_name, modified_at);

    if thumb_path.exists() {
        return Ok(thumb_path.to_string_lossy().to_string());
    }

    create_thumbnail(Path::new(file_path), &thumb_path)?;
    Ok(thumb_path.to_string_lossy().to_string())
}

pub fn get_all_screenshots<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<ScreenshotInfo>, String> {
    let instances = instance::load_instances(app)?;
    let mut screenshots = Vec::new();
    let cache_dir = get_thumbnail_cache_dir(app).ok();
    let mut missing_thumbnails = Vec::new();

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
                    let thumb_path = get_thumbnail_file_path(dir, &item.instance_id, &item.file_name, item.modified_at);
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

pub fn delete_screenshot<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    file_name: &str,
) -> Result<(), String> {
    // Basic sanitization
    if file_name.contains("..") || file_name.contains('/') || file_name.contains('\\') {
        return Err("Invalid file name".to_string());
    }

    let instance_dir = instance::get_instance_dir(app, instance_id)?;
    let file_path = instance_dir.join("screenshots").join(file_name);

    if !file_path.exists() {
        return Err("Screenshot file not found".to_string());
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

    fs::remove_file(&file_path).map_err(|e| format!("Failed to delete screenshot: {e}"))?;
    Ok(())
}

pub fn open_screenshots_folder<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: Option<&str>,
) -> Result<(), String> {
    let target_dir: PathBuf = match instance_id {
        Some(id) if !id.is_empty() => {
            let inst_dir = instance::get_instance_dir(app, id)?;
            let screenshots_dir = inst_dir.join("screenshots");
            if !screenshots_dir.exists() {
                let _ = fs::create_dir_all(&screenshots_dir);
            }
            screenshots_dir
        }
        _ => instance::get_instances_dir(app)?,
    };

    let _path_str = target_dir.to_string_lossy().to_string();
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(&_path_str).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&_path_str).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(&_path_str).spawn();
    }

    Ok(())
}

pub fn reveal_screenshot_file(file_path: &str) -> Result<(), String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err("File does not exist".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let win_path = file_path.replace('/', "\\");
        let _ = std::process::Command::new("explorer")
            .arg(format!("/select,{}", win_path))
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("-R")
            .arg(file_path)
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let parent = path.parent().unwrap_or(path);
        let _ = std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn();
    }

    Ok(())
}
