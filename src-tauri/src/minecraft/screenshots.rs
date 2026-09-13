use crate::minecraft::instance;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tauri::Runtime;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotInfo {
    pub id: String,
    pub instance_id: String,
    pub instance_name: String,
    pub file_name: String,
    pub file_path: String,
    pub file_size_bytes: u64,
    pub created_at: u64,
    pub modified_at: u64,
}

const SUPPORTED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];

pub fn get_all_screenshots<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<ScreenshotInfo>, String> {
    let instances = instance::load_instances(app)?;
    let mut screenshots = Vec::new();

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

            screenshots.push(ScreenshotInfo {
                id,
                instance_id: inst.id.clone(),
                instance_name: inst.name.clone(),
                file_name,
                file_path,
                file_size_bytes,
                created_at,
                modified_at,
            });
        }
    }

    // Sort newest first by modified_at
    screenshots.sort_by_key(|a| std::cmp::Reverse(a.modified_at));
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

    let path_str = target_dir.to_string_lossy().to_string();
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(&path_str).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&path_str).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(&path_str).spawn();
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
