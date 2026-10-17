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
pub use platform::reveal_screenshot_file;
pub use thumbnails::{
    create_thumbnail, get_thumbnail_cache_dir, get_thumbnail_file_path, SUPPORTED_EXTENSIONS,
};
