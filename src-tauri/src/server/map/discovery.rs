use super::bedrock::{bedrock_dimensions, bedrock_tile, bedrock_world_dir};
use super::java::saved_raster;
use super::live::{apply_live_chunks, list_live_regions, live_dir};
use super::models::{level_name, modified, secs, MapDimension, MapError, MapRegion};
use super::raster::Raster;
use base64::Engine;
use std::path::{Path, PathBuf};

/// Candidate region folders for a dimension: 26.x layout first, then legacy
/// vanilla (`DIM-1` inside the world) and Bukkit (`world_nether/DIM-1`) layouts.
pub fn region_dirs(server_dir: &Path, level: &str, dimension: &str) -> Vec<PathBuf> {
    let world = server_dir.join(level);
    let (ns, path) = dimension
        .split_once(':')
        .unwrap_or(("minecraft", dimension));
    let mut dirs = vec![world.join("dimensions").join(ns).join(path).join("region")];
    match dimension {
        "minecraft:overworld" => dirs.push(world.join("region")),
        "minecraft:the_nether" => {
            dirs.push(world.join("DIM-1").join("region"));
            dirs.push(
                server_dir
                    .join(format!("{level}_nether"))
                    .join("DIM-1")
                    .join("region"),
            );
        }
        "minecraft:the_end" => {
            dirs.push(world.join("DIM1").join("region"));
            dirs.push(
                server_dir
                    .join(format!("{level}_the_end"))
                    .join("DIM1")
                    .join("region"),
            );
        }
        _ => {}
    }
    dirs
}

pub fn list_regions(dir: &Path) -> Vec<MapRegion> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|e| {
            let meta = e.metadata().ok().filter(|m| m.len() > 8192)?;
            let name = e.file_name().to_string_lossy().into_owned();
            let mut parts = name.strip_prefix("r.")?.strip_suffix(".mca")?.split('.');
            Some(MapRegion {
                x: parts.next()?.parse().ok()?,
                z: parts.next()?.parse().ok()?,
                modified: meta.modified().map_or(0, secs),
            })
        })
        .collect()
}

pub fn dimensions(server_dir: &Path) -> Vec<MapDimension> {
    if let Some(bedrock_dir) = bedrock_world_dir(server_dir) {
        return bedrock_dimensions(server_dir, &bedrock_dir);
    }

    let level = level_name(server_dir);
    let mut ids: Vec<String> = [
        "minecraft:overworld",
        "minecraft:the_nether",
        "minecraft:the_end",
    ]
    .map(String::from)
    .to_vec();
    // Custom (datapack) dimensions in the 26.x layout
    if let Ok(namespaces) = std::fs::read_dir(server_dir.join(&level).join("dimensions")) {
        for ns in namespaces.flatten() {
            let Ok(dims) = std::fs::read_dir(ns.path()) else {
                continue;
            };
            for dim in dims.flatten() {
                let id = format!(
                    "{}:{}",
                    ns.file_name().to_string_lossy(),
                    dim.file_name().to_string_lossy()
                );
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
        }
    }
    ids.into_iter()
        .filter_map(|id| {
            let mut regions = region_dirs(server_dir, &level, &id)
                .iter()
                .map(|d| list_regions(d))
                .find(|r| !r.is_empty())
                .unwrap_or_default();
            // Live chunks can be in regions the server hasn't saved yet
            for live in list_live_regions(&live_dir(server_dir, &id)) {
                match regions.iter_mut().find(|r| r.x == live.x && r.z == live.z) {
                    Some(region) => region.modified = region.modified.max(live.modified),
                    None => regions.push(live),
                }
            }
            (!regions.is_empty()).then_some(MapDimension { id, regions })
        })
        .collect()
}

/// Returns the tile as a PNG data URL, rendering (or re-rendering) it when needed
pub fn tile_typed(
    server_dir: &Path,
    dimension: &str,
    x: i32,
    z: i32,
) -> Result<Option<String>, MapError> {
    if let Some(bedrock_dir) = bedrock_world_dir(server_dir) {
        return bedrock_tile(server_dir, &bedrock_dir, dimension, x, z);
    }

    let level = level_name(server_dir);
    let file_name = format!("r.{x}.{z}.mca");
    let region_path = region_dirs(server_dir, &level, dimension)
        .into_iter()
        .map(|d| d.join(&file_name))
        .find(|p| p.exists());
    let live_path = live_dir(server_dir, dimension).join(format!("{x}.{z}"));
    let live_time = modified(&live_path);
    if region_path.is_none() && live_time.is_none() {
        return Ok(None);
    }

    let cache_dir = server_dir
        .join(".ingot")
        .join("map")
        .join(dimension.replace(':', "_"));
    let cache_path = cache_dir.join(format!("r.{x}.{z}.png"));
    let source_time = region_path.as_deref().and_then(modified).max(live_time);
    let png = match (modified(&cache_path), source_time) {
        (Some(cached), Some(source)) if cached >= source => std::fs::read(&cache_path)
            .map_err(|e| MapError::TileRead(format!("Failed to read map tile: {e}")))?,
        _ => {
            let ceiling = dimension == "minecraft:the_nether";
            let mut raster = match &region_path {
                Some(path) => saved_raster(path, ceiling)?,
                None => Raster::empty(),
            };
            if live_time.is_some() {
                apply_live_chunks(&mut raster, &live_path);
            }
            let png = raster.to_png()?;
            let _ = std::fs::create_dir_all(&cache_dir);
            let _ = std::fs::write(&cache_path, &png);
            png
        }
    };
    Ok(Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    )))
}

pub fn tile(
    server_dir: &Path,
    dimension: &str,
    x: i32,
    z: i32,
) -> Result<Option<String>, MapError> {
    tile_typed(server_dir, dimension, x, z)
}
