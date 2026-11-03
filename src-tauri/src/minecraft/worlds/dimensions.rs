use super::error::WorldError;
use super::models::{WorldDimension, WorldResetResult};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn nether_candidate_paths(world_dir: &Path) -> [PathBuf; 2] {
    [
        world_dir.join("DIM-1"),
        world_dir
            .join("dimensions")
            .join("minecraft")
            .join("the_nether"),
    ]
}

pub fn the_end_candidate_paths(world_dir: &Path) -> [PathBuf; 2] {
    [
        world_dir.join("DIM1"),
        world_dir
            .join("dimensions")
            .join("minecraft")
            .join("the_end"),
    ]
}

pub fn has_nether(world_dir: &Path) -> bool {
    nether_candidate_paths(world_dir).iter().any(|p| p.is_dir())
}

pub fn has_the_end(world_dir: &Path) -> bool {
    the_end_candidate_paths(world_dir)
        .iter()
        .any(|p| p.is_dir())
}

pub fn existing_dimension_paths(world_dir: &Path, dimension: WorldDimension) -> Vec<PathBuf> {
    let candidates = match dimension {
        WorldDimension::Nether => nether_candidate_paths(world_dir).to_vec(),
        WorldDimension::TheEnd => the_end_candidate_paths(world_dir).to_vec(),
    };
    candidates.into_iter().filter(|p| p.is_dir()).collect()
}

pub fn reset_dimension(
    world_dir: &Path,
    dimension: WorldDimension,
) -> Result<WorldResetResult, WorldError> {
    if !world_dir.join("level.dat").is_file() {
        return Err(WorldError::NotFound(
            world_dir.to_string_lossy().to_string(),
        ));
    }

    let existing = existing_dimension_paths(world_dir, dimension);
    if existing.is_empty() {
        return Err(WorldError::DimensionNotFound(dimension));
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let trash_dir = world_dir
        .join(".ingot-trash")
        .join(format!("{timestamp}_{dimension:?}"));

    fs::create_dir_all(&trash_dir)?;

    let mut reset_paths = Vec::new();

    for path in &existing {
        let relative = path
            .strip_prefix(world_dir)
            .map_err(|e| WorldError::Archive(e.to_string()))?;
        let dest = trash_dir.join(relative);

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        // Rename dimension folder to trash folder
        if let Err(_e) = fs::rename(path, &dest) {
            // If cross-device or permission rename failure, attempt remove as fallback
            fs::remove_dir_all(path).map_err(WorldError::Io)?;
        }

        reset_paths.push(relative.to_string_lossy().replace('\\', "/"));
    }

    Ok(WorldResetResult {
        dimension,
        reset_paths,
        backup_path: Some(trash_dir.to_string_lossy().to_string()),
    })
}
