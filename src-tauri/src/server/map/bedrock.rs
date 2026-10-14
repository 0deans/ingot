use super::models::{level_name, mix, modified, secs, Kind, MapDimension, MapError, MapRegion};
use super::palette::classify;
use super::raster::Raster;
use base64::Engine;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn bedrock_world_dir(server_dir: &Path) -> Option<PathBuf> {
    let level = level_name(server_dir);
    if let Ok(true) = server_dir
        .join("worlds")
        .join(&level)
        .join("db")
        .try_exists()
    {
        return Some(server_dir.join("worlds").join(&level));
    }
    if let Ok(entries) = std::fs::read_dir(server_dir.join("worlds")) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() && p.join("db").exists() {
                return Some(p);
            }
        }
    }
    if let Ok(true) = server_dir.join(&level).join("db").try_exists() {
        return Some(server_dir.join(&level));
    }
    None
}

pub fn dimension_to_id(dim: bedrock_world::Dimension) -> Option<&'static str> {
    match dim {
        bedrock_world::Dimension::Overworld => Some("minecraft:overworld"),
        bedrock_world::Dimension::Nether => Some("minecraft:the_nether"),
        bedrock_world::Dimension::End => Some("minecraft:the_end"),
        bedrock_world::Dimension::Unknown(_) => None,
    }
}

pub fn id_to_dimension(id: &str) -> Option<bedrock_world::Dimension> {
    match id {
        "minecraft:overworld" => Some(bedrock_world::Dimension::Overworld),
        "minecraft:the_nether" => Some(bedrock_world::Dimension::Nether),
        "minecraft:the_end" => Some(bedrock_world::Dimension::End),
        _ => None,
    }
}

pub fn bedrock_dimensions(server_dir: &Path, world_dir: &Path) -> Vec<MapDimension> {
    let cache_file = server_dir
        .join(".ingot")
        .join("map")
        .join("bedrock_dimensions.json");
    let db_path = world_dir.join("db");
    let db_mod_time = modified(&db_path.join("CURRENT"))
        .or_else(|| modified(&db_path))
        .map_or(0, secs);

    let cached_dims: Option<Vec<MapDimension>> = std::fs::read(&cache_file)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());

    let open_res = bedrock_world::BedrockWorld::open_blocking(
        world_dir,
        bedrock_world::world::OpenOptions {
            read_only: true,
            ..Default::default()
        },
    );

    match open_res {
        Ok(world) => {
            let positions = world
                .list_render_chunk_positions_blocking(
                    bedrock_world::world::WorldScanOptions::default(),
                )
                .unwrap_or_default();

            let mut dim_regions: HashMap<&'static str, HashMap<(i32, i32), u32>> = HashMap::new();
            for pos in positions {
                let Some(dim_id) = dimension_to_id(pos.dimension) else {
                    continue;
                };
                let rx = pos.x.div_euclid(32);
                let rz = pos.z.div_euclid(32);
                dim_regions
                    .entry(dim_id)
                    .or_default()
                    .entry((rx, rz))
                    .or_insert(db_mod_time);
            }

            let mut result = Vec::new();
            for dim_id in [
                "minecraft:overworld",
                "minecraft:the_nether",
                "minecraft:the_end",
            ] {
                if let Some(regions_map) = dim_regions.remove(dim_id) {
                    if !regions_map.is_empty() {
                        let mut regions: Vec<MapRegion> = regions_map
                            .into_iter()
                            .map(|((x, z), modified)| MapRegion { x, z, modified })
                            .collect();
                        regions.sort_by_key(|r| (r.x, r.z));
                        result.push(MapDimension {
                            id: dim_id.to_string(),
                            regions,
                        });
                    }
                }
            }

            if let Ok(json) = serde_json::to_vec(&result) {
                let _ = std::fs::create_dir_all(server_dir.join(".ingot").join("map"));
                let _ = std::fs::write(&cache_file, json);
            }

            if !result.is_empty() {
                return result;
            }
            cached_dims.unwrap_or_default()
        }
        Err(e) => {
            eprintln!("[Ingot] Could not open Bedrock world for dimension scan: {e}");
            cached_dims.unwrap_or_default()
        }
    }
}

pub fn bedrock_surface_of(
    sample: &bedrock_world::world::TerrainColumnSample,
    ceiling: bool,
) -> Option<([u8; 3], i32, bool)> {
    const WATER: [u8; 3] = [52, 94, 196];

    // In the Nether, skip the bedrock ceiling if the top block is bedrock
    if ceiling && sample.surface_y > 120 && sample.surface_block_state.name.contains("bedrock") {
        let under_y = sample.relief_y as i32;
        if under_y <= 120 {
            return match classify(&sample.relief_block_state.name) {
                Kind::Solid(color) => Some((color, under_y, false)),
                Kind::Water => Some((WATER, under_y, true)),
                Kind::Clear => None,
            };
        }
    }

    if let Some(water) = &sample.water {
        let depth = (water.depth as f32).clamp(1.0, 24.0);
        let t = (0.45 + depth / 24.0 * 0.5).min(0.95);
        let under_color = match &water.underwater_block_state {
            Some(state) => match classify(&state.name) {
                Kind::Solid(c) => c,
                _ => [134, 96, 67],
            },
            None => [134, 96, 67],
        };
        let color = mix(under_color, WATER, t);
        Some((color, water.surface_y as i32, true))
    } else {
        match classify(&sample.surface_block_state.name) {
            Kind::Solid(color) => Some((color, sample.surface_y as i32, false)),
            Kind::Water => Some((WATER, sample.surface_y as i32, true)),
            Kind::Clear => None,
        }
    }
}

pub fn bedrock_tile(
    server_dir: &Path,
    world_dir: &Path,
    dimension: &str,
    x: i32,
    z: i32,
) -> Result<Option<String>, MapError> {
    let cache_dir = server_dir
        .join(".ingot")
        .join("map")
        .join(dimension.replace(':', "_"));
    let cache_path = cache_dir.join(format!("r.{x}.{z}.png"));

    let db_path = world_dir.join("db");
    let db_mod_time = modified(&db_path.join("CURRENT"))
        .or_else(|| modified(&db_path))
        .map_or(0, secs);

    if let Some(cached_time) = modified(&cache_path).map(secs) {
        if cached_time >= db_mod_time && db_mod_time > 0 {
            if let Ok(bytes) = std::fs::read(&cache_path) {
                return Ok(Some(format!(
                    "data:image/png;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(bytes)
                )));
            }
        }
    }

    let b_dim = match id_to_dimension(dimension) {
        Some(d) => d,
        None => return Ok(None),
    };

    let world = match bedrock_world::BedrockWorld::open_blocking(
        world_dir,
        bedrock_world::world::OpenOptions {
            read_only: true,
            ..Default::default()
        },
    ) {
        Ok(w) => w,
        Err(e) => {
            if let Ok(bytes) = std::fs::read(&cache_path) {
                return Ok(Some(format!(
                    "data:image/png;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(bytes)
                )));
            }
            return Err(MapError::Bedrock(format!(
                "Could not open Bedrock world: {e}"
            )));
        }
    };

    let mut positions = Vec::with_capacity(1024);
    for cz in 0..32 {
        for cx in 0..32 {
            positions.push(bedrock_world::ChunkPos {
                x: x * 32 + cx,
                z: z * 32 + cz,
                dimension: b_dim,
            });
        }
    }

    let load_options = bedrock_world::world::ChunkLoadOptions::exact_surface_columns(
        bedrock_world::world::ExactSurfaceSubchunkPolicy::Full,
        bedrock_world::world::ExactSurfaceBiomeLoad::None,
        false,
    );

    let chunks = world
        .query_chunk_data_many_blocking(positions, load_options)
        .map_err(|e| MapError::Bedrock(format!("Failed to read Bedrock chunk data: {e}")))?;

    let non_empty = chunks.iter().any(|c| c.column_samples.is_some());
    if !non_empty {
        return Ok(None);
    }

    let mut raster = Raster::empty();
    let ceiling = dimension == "minecraft:the_nether";

    for chunk in chunks {
        let lx_base = (chunk.pos.x.rem_euclid(32) as usize) * 16;
        let lz_base = (chunk.pos.z.rem_euclid(32) as usize) * 16;
        let chunk_idx =
            (chunk.pos.z.rem_euclid(32) as usize) * 32 + (chunk.pos.x.rem_euclid(32) as usize);
        raster.chunk_times[chunk_idx] = 1;

        for cz in 0..16u8 {
            for cx in 0..16u8 {
                if let Some(col) = chunk.column_sample_at(cx, cz) {
                    let px = lx_base + cx as usize;
                    let pz = lz_base + cz as usize;
                    if let Some(surface) = bedrock_surface_of(col, ceiling) {
                        raster.set(px, pz, Some(surface));
                    }
                }
            }
        }
    }

    let png = raster.to_png()?;
    let _ = std::fs::create_dir_all(&cache_dir);
    let _ = std::fs::write(&cache_path, &png);
    Ok(Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    )))
}
