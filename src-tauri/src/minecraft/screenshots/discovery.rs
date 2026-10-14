//! Bedrock and companion screenshot discovery and directory scanning.
use super::thumbnails::SUPPORTED_EXTENSIONS;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Returns known candidate directories where Minecraft Bedrock Edition screenshots reside.
pub fn get_bedrock_screenshot_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // 1. Modern GDK path: %APPDATA%\Minecraft Bedrock\Users\<UserID>\games\com.mojang\Screenshots
    if let Ok(appdata) = std::env::var("APPDATA") {
        let gdk_users = PathBuf::from(&appdata)
            .join("Minecraft Bedrock")
            .join("Users");
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

/// Recursively finds all image files up to `max_depth` inside `dir`.
pub(crate) fn scan_screenshot_files(dir: &Path, max_depth: usize) -> Vec<PathBuf> {
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

/// Reads companion `.json` metadata file if available to extract accurate capture timestamp.
pub(crate) fn get_companion_capture_time(image_path: &Path) -> Option<u64> {
    let parent = image_path.parent()?;
    let stem = image_path.file_stem()?.to_str()?;
    let json_path = parent.join(format!("{stem}.json"));
    if json_path.exists() {
        if let Ok(content) = fs::read_to_string(json_path) {
            #[derive(Deserialize)]
            struct CompanionMeta {
                #[serde(rename = "captureTime")]
                capture_time: Option<u64>,
            }
            if let Ok(meta) = serde_json::from_str::<CompanionMeta>(&content) {
                if let Some(time) = meta.capture_time {
                    return Some(if time < 100_000_000_000 {
                        time * 1000
                    } else {
                        time
                    });
                }
            }
        }
    }
    None
}

/// Identifies the primary or most active Bedrock screenshots directory.
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
