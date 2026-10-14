//! Platform-specific folder opening and file revealing in OS file managers.
use super::discovery::get_primary_bedrock_screenshots_folder;
use crate::minecraft::instance;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Runtime;

/// Opens the screenshots folder corresponding to an instance or Bedrock in the OS file explorer.
pub fn open_screenshots_folder<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: Option<&str>,
) -> Result<(), String> {
    let target_dir: PathBuf = match instance_id {
        Some("bedrock") => get_primary_bedrock_screenshots_folder(),
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
        let _ = std::process::Command::new("explorer")
            .arg(&_path_str)
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&_path_str).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(&_path_str)
            .spawn();
    }

    Ok(())
}

/// Highlights and selects a screenshot file in the OS file manager.
pub fn reveal_screenshot_file(file_path: &str) -> Result<(), String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err("File does not exist".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let win_path = file_path.replace('/', "\\");
        let _ = std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&win_path)
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
        let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
    }

    Ok(())
}
