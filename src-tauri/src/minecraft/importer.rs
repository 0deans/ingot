//! Minecraft external launcher discovery and instance import coordinator.
pub(crate) mod discovery;
pub(crate) mod execute;
pub(crate) mod formats;
pub(crate) mod models;
#[cfg(test)]
mod tests;
pub(crate) mod version_id;

pub use models::{
    DetectedLauncher, ImportError, ImportInstanceOptions, ImportReport, ImportableInstance,
};
use tauri::Runtime;

/// Detects known launcher installations on the system and returns their metadata.
pub fn get_detected_launchers<R: Runtime>(app: &tauri::AppHandle<R>) -> Vec<DetectedLauncher> {
    discovery::get_detected_launchers(app)
}

/// Scans instances for a specific launcher or custom path.
pub fn get_launcher_instances<R: Runtime>(
    app: &tauri::AppHandle<R>,
    launcher_id: &str,
    custom_path: Option<&str>,
) -> Result<Vec<ImportableInstance>, String> {
    discovery::get_launcher_instances(app, launcher_id, custom_path).map_err(|e| e.to_string())
}

/// Detects an instance from a custom directory path.
pub fn detect_custom_instance(path_str: &str) -> Result<ImportableInstance, String> {
    discovery::detect_custom_instance(path_str).map_err(|e| e.to_string())
}

/// Imports an external instance into Ingot.
pub fn import_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    options: ImportInstanceOptions,
) -> Result<ImportReport, String> {
    execute::import_instance(app, options).map_err(|e| e.to_string())
}
