use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::error::InstanceContentError;
use super::models::TrackedOrigin;

pub fn tracking_path(instance_dir: &Path) -> PathBuf {
    instance_dir.join(".ingot").join("content.json")
}

pub fn read_tracking(
    instance_dir: &Path,
) -> Result<HashMap<String, TrackedOrigin>, InstanceContentError> {
    let path = tracking_path(instance_dir);
    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
        Err(e) => return Err(InstanceContentError::io(path, e)),
    };

    serde_json::from_slice(&bytes).map_err(|e| InstanceContentError::Serialization(e.to_string()))
}

pub fn write_tracking(
    instance_dir: &Path,
    tracking: &HashMap<String, TrackedOrigin>,
) -> Result<(), InstanceContentError> {
    let path = tracking_path(instance_dir);
    let parent = instance_dir.join(".ingot");
    if let Err(e) = fs::create_dir_all(&parent) {
        return Err(InstanceContentError::io(parent, e));
    }

    let json = serde_json::to_vec_pretty(tracking)
        .map_err(|e| InstanceContentError::Serialization(e.to_string()))?;

    ingot_core::write_file_atomic(&path, &json).map_err(|e| InstanceContentError::io(&path, e))?;

    Ok(())
}
