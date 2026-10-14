//! Minecraft screenshots discovery, thumbnail management, and viewer coordinator.
pub(crate) mod discovery;
pub(crate) mod models;
pub(crate) mod platform;
pub(crate) mod service;
#[cfg(test)]
mod tests;
pub(crate) mod thumbnails;

pub use discovery::{get_bedrock_screenshot_dirs, get_primary_bedrock_screenshots_folder};
pub use models::{ScreenshotError, ScreenshotInfo};
pub use platform::{open_screenshots_folder, reveal_screenshot_file};
pub use thumbnails::{
    create_thumbnail, get_thumbnail_cache_dir, get_thumbnail_file_path, SUPPORTED_EXTENSIONS,
};

use tauri::Runtime;

/// Gathers all screenshots across all Java Edition instances and Bedrock Edition.
pub fn get_all_screenshots<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<ScreenshotInfo>, String> {
    service::get_all_screenshots(app).map_err(|e| e.to_string())
}

/// Retrieves an existing thumbnail or generates one immediately if missing.
pub fn get_or_create_thumbnail<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    file_name: &str,
    file_path: &str,
    modified_at: u64,
) -> Result<String, String> {
    thumbnails::get_or_create_thumbnail(app, instance_id, file_name, file_path, modified_at)
        .map_err(|e| e.to_string())
}

/// Deletes a specific screenshot file and removes its cached thumbnail and companion files.
pub fn delete_screenshot<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    file_name: &str,
) -> Result<(), String> {
    service::delete_screenshot(app, instance_id, file_name).map_err(|e| e.to_string())
}
