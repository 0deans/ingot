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

pub fn get_bedrock_screenshot_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // 1. Modern GDK path: %APPDATA%\Minecraft Bedrock\Users\<UserID>\games\com.mojang\Screenshots
    if let Ok(appdata) = std::env::var("APPDATA") {
        let gdk_users = PathBuf::from(&appdata).join("Minecraft Bedrock").join("Users");
        if gdk_users.exists() {
            if let Ok(entries) = fs::read_dir(&gdk_users) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let ss_dir = path.join("games").join("com.mojang").join("Screenshots");
                        if ss_dir.exists() {
                            dirs.push(ss_dir);
                        }
                    }
                }
            }
        }
    }

    // 2. Legacy UWP retail: %LOCALAPPDATA%\Packages\Microsoft.MinecraftUWP_8wekyb3d8bbwe\LocalState\games\com.mojang\Screenshots
    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        let uwp_ss = PathBuf::from(&localappdata)
            .join("Packages")
            .join("Microsoft.MinecraftUWP_8wekyb3d8bbwe")
            .join("LocalState")
            .join("games")
            .join("com.mojang")
            .join("Screenshots");
        if uwp_ss.exists() {
            dirs.push(uwp_ss);
        }

        // 3. Bedrock Preview: %LOCALAPPDATA%\Packages\Microsoft.MinecraftWindowsBeta_8wekyb3d8bbwe\LocalState\games\com.mojang\Screenshots
        let beta_ss = PathBuf::from(&localappdata)
            .join("Packages")
            .join("Microsoft.MinecraftWindowsBeta_8wekyb3d8bbwe")
            .join("LocalState")
            .join("games")
            .join("com.mojang")
            .join("Screenshots");
        if beta_ss.exists() {
            dirs.push(beta_ss);
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let mcpe_dir = PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("mcpelauncher")
                .join("games")
                .join("com.mojang")
                .join("Screenshots");
            if mcpe_dir.exists() {
                dirs.push(mcpe_dir);
            }
        }
    }

    dirs
}

fn scan_screenshot_files(dir: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if max_depth == 0 || !dir.is_dir() {
        return files;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let ext = path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
                    files.push(path);
                }
            } else if path.is_dir() && max_depth > 1 {
                files.extend(scan_screenshot_files(&path, max_depth - 1));
            }
        }
    }
    files
}

fn get_companion_capture_time(image_path: &Path) -> Option<u64> {
    let parent = image_path.parent()?;
    let stem = image_path.file_stem()?.to_str()?;
    let json_path = parent.join(format!("{}.json", stem));
    if json_path.exists() {
        if let Ok(content) = fs::read_to_string(json_path) {
            #[derive(Deserialize)]
            struct CompanionMeta {
                #[serde(rename = "captureTime")]
                capture_time: Option<u64>,
            }
            if let Ok(meta) = serde_json::from_str::<CompanionMeta>(&content) {
                if let Some(time) = meta.capture_time {
                    return Some(if time < 100_000_000_000 { time * 1000 } else { time });
                }
            }
        }
    }
    None
}

pub fn get_all_screenshots<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<ScreenshotInfo>, String> {
    let instances = instance::load_instances(app)?;
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

            let id = format!("bedrock_{}", file_name);
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

        let file_path = found_path.ok_or_else(|| "Screenshot file not found".to_string())?;

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

        // Clean up companion files (.json, .mc)
        if let Some(parent) = file_path.parent() {
            if let Some(stem) = file_path.file_stem().and_then(|s| s.to_str()) {
                let json_path = parent.join(format!("{}.json", stem));
                if json_path.exists() {
                    let _ = fs::remove_file(json_path);
                }
                let mc_path = parent.join(format!("{}.mc", stem));
                if mc_path.exists() {
                    let _ = fs::remove_file(mc_path);
                }
            }
        }

        fs::remove_file(&file_path).map_err(|e| format!("Failed to delete screenshot: {e}"))?;
        return Ok(());
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

pub fn get_primary_bedrock_screenshots_folder() -> PathBuf {
    let dirs = get_bedrock_screenshot_dirs();

    // Prefer the subfolder with the most recent screenshot
    let mut best_folder: Option<(PathBuf, u64)> = None;

    for dir in &dirs {
        let files = scan_screenshot_files(dir, 3);
        for file in files {
            let modified = file
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            if let Some(parent) = file.parent() {
                if let Some((_, best_mod)) = best_folder {
                    if modified > best_mod {
                        best_folder = Some((parent.to_path_buf(), modified));
                    }
                } else {
                    best_folder = Some((parent.to_path_buf(), modified));
                }
            }
        }
    }

    if let Some((folder, _)) = best_folder {
        return folder;
    }

    // Fallback to first detected screenshot dir if exists
    if let Some(first) = dirs.into_iter().next() {
        return first;
    }

    // Fallback to APPDATA root
    std::env::var("APPDATA")
        .map(|a| PathBuf::from(a).join("Minecraft Bedrock").join("Users"))
        .unwrap_or_else(|_| PathBuf::from("."))
}

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
        let _ = std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bedrock_screenshot_dirs_detection() {
        let dirs = get_bedrock_screenshot_dirs();
        for d in &dirs {
            let files = scan_screenshot_files(d, 3);
            assert!(d.exists(), "Detected directory must exist");
            for f in files {
                assert!(f.is_file(), "Scanned item must be a file");
            }
        }
    }
}

