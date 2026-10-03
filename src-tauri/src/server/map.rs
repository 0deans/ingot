//! Top-down world map rendered from the world's Anvil region files, plus live chunks
//! from Ingot's companion plugin when it's installed.
//!
//! Each region (32x32 chunks) becomes a 512x512 PNG tile (1 px per block), cached next
//! to the world and re-rendered when the region file or its live chunks change. Saved
//! regions work for any core that writes Anvil (vanilla, Paper, Fabric, Pumpkin). The
//! plugin (Paper/Purpur/Folia) adds chunks straight from server memory, so the map is
//! current without the server ever having to save.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::SystemTime;

const TILE: usize = 512;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MapRegion {
    pub x: i32,
    pub z: i32,
    /// When the region file last changed (unix seconds), so the UI refetches only changed tiles
    pub modified: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MapDimension {
    /// Namespaced id, e.g. "minecraft:overworld"
    pub id: String,
    pub regions: Vec<MapRegion>,
}

/// Name of the world folder (`level-name` in server.properties)
pub fn level_name(server_dir: &Path) -> String {
    std::fs::read_to_string(server_dir.join("server.properties"))
        .ok()
        .and_then(|raw| {
            raw.lines()
                .find_map(|l| l.strip_prefix("level-name=").map(|v| v.trim().to_string()))
        })
        .filter(|name| !name.is_empty() && !name.contains(['/', '\\']) && name != "..")
        .unwrap_or_else(|| "world".to_string())
}

/// Candidate region folders for a dimension: 26.x layout first, then legacy
/// vanilla (`DIM-1` inside the world) and Bukkit (`world_nether/DIM-1`) layouts.
fn region_dirs(server_dir: &Path, level: &str, dimension: &str) -> Vec<PathBuf> {
    let world = server_dir.join(level);
    let (ns, path) = dimension.split_once(':').unwrap_or(("minecraft", dimension));
    let mut dirs = vec![world.join("dimensions").join(ns).join(path).join("region")];
    match dimension {
        "minecraft:overworld" => dirs.push(world.join("region")),
        "minecraft:the_nether" => {
            dirs.push(world.join("DIM-1").join("region"));
            dirs.push(server_dir.join(format!("{level}_nether")).join("DIM-1").join("region"));
        }
        "minecraft:the_end" => {
            dirs.push(world.join("DIM1").join("region"));
            dirs.push(server_dir.join(format!("{level}_the_end")).join("DIM1").join("region"));
        }
        _ => {}
    }
    dirs
}

fn secs(time: SystemTime) -> u32 {
    time.duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as u32)
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn list_regions(dir: &Path) -> Vec<MapRegion> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
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

// ─── Live chunks from the companion plugin ────────────────────────────────────

/// Where the Ingot plugin writes chunks it read from server memory:
/// `.ingot/live/<namespace>/<dimension>/<rx>.<rz>/<cx>.<cz>.bin`
fn live_dir(server_dir: &Path, dimension: &str) -> PathBuf {
    let (ns, path) = dimension.split_once(':').unwrap_or(("minecraft", dimension));
    server_dir.join(".ingot").join("live").join(ns).join(path)
}

/// Regions with live chunks. The plugin replaces files by renaming, which updates the
/// folder's time, so one stat per region tells whether anything in it changed.
fn list_live_regions(dir: &Path) -> Vec<MapRegion> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let (x, z) = name.split_once('.')?;
            Some(MapRegion {
                x: x.parse().ok()?,
                z: z.parse().ok()?,
                modified: e.metadata().ok()?.modified().map_or(0, secs),
            })
        })
        .collect()
}

fn bedrock_world_dir(server_dir: &Path) -> Option<PathBuf> {
    let level = level_name(server_dir);
    if let Ok(true) = server_dir.join("worlds").join(&level).join("db").try_exists() {
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

fn dimension_to_id(dim: bedrock_world::Dimension) -> Option<&'static str> {
    match dim {
        bedrock_world::Dimension::Overworld => Some("minecraft:overworld"),
        bedrock_world::Dimension::Nether => Some("minecraft:the_nether"),
        bedrock_world::Dimension::End => Some("minecraft:the_end"),
        bedrock_world::Dimension::Unknown(_) => None,
    }
}

fn id_to_dimension(id: &str) -> Option<bedrock_world::Dimension> {
    match id {
        "minecraft:overworld" => Some(bedrock_world::Dimension::Overworld),
        "minecraft:the_nether" => Some(bedrock_world::Dimension::Nether),
        "minecraft:the_end" => Some(bedrock_world::Dimension::End),
        _ => None,
    }
}

fn bedrock_dimensions(server_dir: &Path, world_dir: &Path) -> Vec<MapDimension> {
    let cache_file = server_dir.join(".ingot").join("map").join("bedrock_dimensions.json");
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
                .list_render_chunk_positions_blocking(bedrock_world::world::WorldScanOptions::default())
                .unwrap_or_default();

            let mut dim_regions: HashMap<&'static str, HashMap<(i32, i32), u32>> = HashMap::new();
            for pos in positions {
                let Some(dim_id) = dimension_to_id(pos.dimension) else { continue };
                let rx = pos.x.div_euclid(32);
                let rz = pos.z.div_euclid(32);
                dim_regions
                    .entry(dim_id)
                    .or_default()
                    .entry((rx, rz))
                    .or_insert(db_mod_time);
            }

            let mut result = Vec::new();
            for dim_id in ["minecraft:overworld", "minecraft:the_nether", "minecraft:the_end"] {
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

fn bedrock_surface_of(
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

fn bedrock_tile(
    server_dir: &Path,
    world_dir: &Path,
    dimension: &str,
    x: i32,
    z: i32,
) -> Result<Option<String>, String> {
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
            return Err(format!("Could not open Bedrock world: {e}"));
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
        .map_err(|e| format!("Failed to read Bedrock chunk data: {e}"))?;

    let non_empty = chunks.iter().any(|c| c.column_samples.is_some());
    if !non_empty {
        return Ok(None);
    }

    let mut raster = Raster::empty();
    let ceiling = dimension == "minecraft:the_nether";

    for chunk in chunks {
        let lx_base = (chunk.pos.x.rem_euclid(32) as usize) * 16;
        let lz_base = (chunk.pos.z.rem_euclid(32) as usize) * 16;
        let chunk_idx = (chunk.pos.z.rem_euclid(32) as usize) * 32 + (chunk.pos.x.rem_euclid(32) as usize);
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

pub fn dimensions(server_dir: &Path) -> Vec<MapDimension> {
    if let Some(bedrock_dir) = bedrock_world_dir(server_dir) {
        return bedrock_dimensions(server_dir, &bedrock_dir);
    }

    let level = level_name(server_dir);
    let mut ids: Vec<String> = ["minecraft:overworld", "minecraft:the_nether", "minecraft:the_end"]
        .map(String::from)
        .to_vec();
    // Custom (datapack) dimensions in the 26.x layout
    if let Ok(namespaces) = std::fs::read_dir(server_dir.join(&level).join("dimensions")) {
        for ns in namespaces.flatten() {
            let Ok(dims) = std::fs::read_dir(ns.path()) else { continue };
            for dim in dims.flatten() {
                let id = format!("{}:{}", ns.file_name().to_string_lossy(), dim.file_name().to_string_lossy());
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
pub fn tile(server_dir: &Path, dimension: &str, x: i32, z: i32) -> Result<Option<String>, String> {
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
        (Some(cached), Some(source)) if cached >= source => {
            std::fs::read(&cache_path).map_err(|e| format!("Failed to read map tile: {e}"))?
        }
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

// ─── Chunk format (1.18+) ─────────────────────────────────────────────────────

#[derive(Deserialize)]
struct Chunk {
    #[serde(rename = "Status")]
    status: Option<String>,
    #[serde(default)]
    sections: Vec<Section>,
}

#[derive(Deserialize)]
struct Section {
    #[serde(rename = "Y")]
    y: i8,
    block_states: Option<BlockStates>,
}

#[derive(Deserialize)]
struct BlockStates {
    palette: Vec<PaletteEntry>,
    data: Option<fastnbt::LongArray>,
}

#[derive(Deserialize)]
struct PaletteEntry {
    #[serde(rename = "Name")]
    name: String,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// Air and things the map should look through (glass, torches, tall grass...)
    Clear,
    Water,
    Solid([u8; 3]),
}

/// A section with its palette resolved to map kinds
struct ResolvedSection {
    y: i32,
    kinds: Vec<Kind>,
    data: Vec<i64>,
    bits: usize,
}

impl ResolvedSection {
    fn kind_at(&self, x: usize, y: usize, z: usize) -> Kind {
        if self.kinds.len() == 1 || self.data.is_empty() {
            return self.kinds[0];
        }
        let index = (y * 16 + z) * 16 + x;
        let per_long = 64 / self.bits;
        let long = self.data[index / per_long] as u64;
        let value = (long >> ((index % per_long) * self.bits)) & ((1u64 << self.bits) - 1);
        self.kinds.get(value as usize).copied().unwrap_or(Kind::Clear)
    }
}

/// Chunks of a region file with their stored generation step and save time
fn read_chunks(region_path: &Path) -> Result<Vec<(usize, usize, Chunk, u32)>, String> {
    let bytes = std::fs::read(region_path).map_err(|e| format!("Failed to read region: {e}"))?;
    if bytes.len() < 8192 {
        return Ok(Vec::new());
    }
    let mut chunks = Vec::new();
    for i in 0..1024 {
        let header = &bytes[i * 4..i * 4 + 4];
        let offset = ((header[0] as usize) << 16 | (header[1] as usize) << 8 | header[2] as usize) * 4096;
        if offset == 0 || offset + 5 > bytes.len() {
            continue;
        }
        // Second header table: when each chunk was last saved (unix seconds)
        let saved = u32::from_be_bytes(bytes[4096 + i * 4..4096 + i * 4 + 4].try_into().unwrap());
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let Some(payload) = bytes.get(offset + 5..offset + 4 + length) else { continue };
        let mut raw = Vec::new();
        let ok = match bytes[offset + 4] {
            1 => flate2::read::GzDecoder::new(payload).read_to_end(&mut raw).is_ok(),
            2 => flate2::read::ZlibDecoder::new(payload).read_to_end(&mut raw).is_ok(),
            3 => {
                raw.extend_from_slice(payload);
                true
            }
            _ => false, // LZ4 / external chunks aren't supported
        };
        if !ok {
            continue;
        }
        if let Ok(chunk) = fastnbt::from_bytes::<Chunk>(&raw) {
            if has_terrain(chunk.status.as_deref()) {
                chunks.push((i % 32, i / 32, chunk, saved));
            }
        }
    }
    Ok(chunks)
}

/// Whether a chunk has its surface blocks yet. A running server writes loaded chunks
/// in batches, so many are still saved at an earlier generation step; drawing only
/// "full" ones leaves holes all over the explored area.
fn has_terrain(status: Option<&str>) -> bool {
    let Some(status) = status else { return true };
    let step = status.trim_start_matches("minecraft:");
    matches!(step, "surface" | "carvers" | "features" | "initialize_light" | "light" | "spawn" | "full")
}

/// One region's map before shading and encoding
#[derive(Clone)]
struct Raster {
    /// RGBA, alpha 0 where there's no data
    pixels: Vec<u8>,
    /// Surface height per pixel, for relief shading; i32::MIN = no data
    heights: Vec<i32>,
    is_water: Vec<bool>,
    /// When each chunk's data was taken (unix seconds); 0 = no data
    chunk_times: Vec<u32>,
}

impl Raster {
    fn empty() -> Self {
        Raster {
            pixels: vec![0; TILE * TILE * 4],
            heights: vec![i32::MIN; TILE * TILE],
            is_water: vec![false; TILE * TILE],
            chunk_times: vec![0; 1024],
        }
    }

    fn set(&mut self, px: usize, pz: usize, surface: Option<([u8; 3], i32, bool)>) {
        let i = pz * TILE + px;
        match surface {
            Some((color, height, water)) => {
                self.pixels[i * 4..i * 4 + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
                self.heights[i] = height;
                self.is_water[i] = water;
            }
            None => {
                self.pixels[i * 4..i * 4 + 4].fill(0);
                self.heights[i] = i32::MIN;
                self.is_water[i] = false;
            }
        }
    }

    /// Relief shading (lighter uphill from the north, darker downhill), then PNG
    fn to_png(&self) -> Result<Vec<u8>, String> {
        let mut pixels = self.pixels.clone();
        for pz in 1..TILE {
            for px in 0..TILE {
                let i = pz * TILE + px;
                let north = self.heights[i - TILE];
                if self.heights[i] == i32::MIN || north == i32::MIN || self.is_water[i] {
                    continue;
                }
                let factor = match self.heights[i].cmp(&north) {
                    std::cmp::Ordering::Greater => 1.12,
                    std::cmp::Ordering::Less => 0.84,
                    std::cmp::Ordering::Equal => 1.0,
                };
                for c in &mut pixels[i * 4..i * 4 + 3] {
                    *c = (*c as f32 * factor).min(255.0) as u8;
                }
            }
        }
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, TILE as u32, TILE as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_compression(png::Compression::Fast);
            let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
            writer.write_image_data(&pixels).map_err(|e| e.to_string())?;
        }
        Ok(out)
    }
}

/// Parsed saved regions, so live updates don't re-read the whole region file each time
static SAVED_RASTERS: LazyLock<Mutex<Vec<(PathBuf, SystemTime, Raster)>>> = LazyLock::new(Default::default);
const SAVED_RASTER_CACHE: usize = 6;

/// The region as saved on disk (cached in memory until the file changes)
fn saved_raster(region_path: &Path, ceiling: bool) -> Result<Raster, String> {
    let time = modified(region_path).unwrap_or(SystemTime::UNIX_EPOCH);
    if let Ok(cache) = SAVED_RASTERS.lock() {
        if let Some((_, _, raster)) = cache.iter().find(|(p, t, _)| p == region_path && *t == time) {
            return Ok(raster.clone());
        }
    }
    let raster = render_saved(region_path, ceiling)?;
    if let Ok(mut cache) = SAVED_RASTERS.lock() {
        cache.retain(|(p, _, _)| p != region_path);
        if cache.len() >= SAVED_RASTER_CACHE {
            cache.remove(0);
        }
        cache.push((region_path.to_path_buf(), time, raster.clone()));
    }
    Ok(raster)
}

fn render_saved(region_path: &Path, ceiling: bool) -> Result<Raster, String> {
    let chunks = read_chunks(region_path)?;
    let mut kind_cache: HashMap<String, Kind> = HashMap::new();
    let mut raster = Raster::empty();

    for (cx, cz, chunk, saved) in chunks {
        raster.chunk_times[cz * 32 + cx] = saved.max(1);
        let mut sections: Vec<ResolvedSection> = chunk
            .sections
            .into_iter()
            .filter_map(|s| {
                let states = s.block_states?;
                let kinds: Vec<Kind> = states
                    .palette
                    .iter()
                    .map(|p| *kind_cache.entry(p.name.clone()).or_insert_with(|| classify(&p.name)))
                    .collect();
                if kinds.iter().all(|k| *k == Kind::Clear) && !ceiling {
                    return None;
                }
                let bits = (usize::BITS - (kinds.len().max(2) - 1).leading_zeros()).max(4) as usize;
                Some(ResolvedSection {
                    y: s.y as i32,
                    kinds,
                    data: states.data.map(|d| d.into_inner()).unwrap_or_default(),
                    bits,
                })
            })
            .collect();
        sections.sort_by(|a, b| b.y.cmp(&a.y));

        for z in 0..16 {
            for x in 0..16 {
                if let Some(surface) = column(&sections, x, z, ceiling) {
                    raster.set(cx * 16 + x, cz * 16 + z, Some(surface));
                }
            }
        }
    }
    Ok(raster)
}

/// Draws the plugin's live chunks over the saved ones, where they're newer
fn apply_live_chunks(raster: &mut Raster, live_region: &Path) {
    let Ok(entries) = std::fs::read_dir(live_region) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some((cx, cz)) = name.strip_suffix(".bin").and_then(|n| n.split_once('.')) else { continue };
        let (Ok(cx), Ok(cz)) = (cx.parse::<i32>(), cz.parse::<i32>()) else { continue };
        let (lx, lz) = (cx.rem_euclid(32) as usize, cz.rem_euclid(32) as usize);
        let Some(modified) = entry.metadata().ok().and_then(|m| m.modified().ok()) else { continue };
        let taken = secs(modified);
        // The server saved this chunk after the plugin captured it: the saved one wins
        if taken < raster.chunk_times[lz * 32 + lx] {
            continue;
        }
        let Some(surface) = live_surface(&entry.path(), modified) else { continue };
        raster.chunk_times[lz * 32 + lx] = taken;
        for (i, column) in surface.iter().enumerate() {
            raster.set(lx * 16 + i % 16, lz * 16 + i / 16, *column);
        }
    }
}

/// A live chunk resolved to what the map shows in each column (index z * 16 + x)
type LiveSurface = [Option<([u8; 3], i32, bool)>; 256];

/// Resolved live chunks by file and its modification time: re-rendering a region then
/// only reads and decodes the chunks the plugin rewrote since, not all of them
type LiveChunkCache = Mutex<HashMap<PathBuf, (SystemTime, Arc<LiveSurface>)>>;
static LIVE_CHUNKS: LazyLock<LiveChunkCache> = LazyLock::new(|| Mutex::new(HashMap::new()));
/// About 12 MB of resolved chunks; cleared when full (rare: four fully explored regions)
const LIVE_CHUNK_CACHE: usize = 4096;

fn live_surface(path: &Path, modified: SystemTime) -> Option<Arc<LiveSurface>> {
    if let Ok(cache) = LIVE_CHUNKS.lock() {
        if let Some((time, surface)) = cache.get(path) {
            if *time == modified {
                return Some(surface.clone());
            }
        }
    }
    let surface: Arc<LiveSurface> = Arc::from(decode_live_chunk(&std::fs::read(path).ok()?)?);
    if let Ok(mut cache) = LIVE_CHUNKS.lock() {
        if cache.len() >= LIVE_CHUNK_CACHE {
            cache.clear();
        }
        cache.insert(path.to_path_buf(), (modified, surface.clone()));
    }
    Some(surface)
}

/// Reads the plugin's chunk format: "IGC" + version 1, a palette of block ids, then 256
/// columns (index z * 16 + x) of runs (palette index, top y, length) from the top down.
/// Big endian, as Java's DataOutputStream writes. Resolves each column to its surface.
fn decode_live_chunk(bytes: &[u8]) -> Option<Box<LiveSurface>> {
    let mut pos = 0;
    let mut take = |n: usize| -> Option<&[u8]> {
        let slice = bytes.get(pos..pos + n)?;
        pos += n;
        Some(slice)
    };
    if take(4)? != b"IGC\x01" {
        return None;
    }
    let u16_at = |b: &[u8]| u16::from_be_bytes([b[0], b[1]]);
    // Classify each block id once per chunk, not once per run
    let palette_len = u16_at(take(2)?);
    let mut kinds = Vec::with_capacity(palette_len as usize);
    for _ in 0..palette_len {
        let len = u16_at(take(2)?) as usize;
        kinds.push(classify(std::str::from_utf8(take(len)?).ok()?));
    }
    let mut surface: Box<LiveSurface> = Box::new([None; 256]);
    let mut runs = Vec::new();
    for column in surface.iter_mut() {
        runs.clear();
        for _ in 0..u16_at(take(2)?) {
            let run = take(6)?;
            let kind = *kinds.get(u16_at(run) as usize)?;
            let top = i16::from_be_bytes([run[2], run[3]]) as i32;
            runs.push((kind, top, u16_at(&run[4..]) as i32));
        }
        let blocks = runs.iter().flat_map(|&(kind, top, len)| (0..len).map(move |d| (top - d, kind)));
        // The plugin already starts below the Nether roof
        *column = surface_of(blocks, false);
    }
    Some(surface)
}

/// Finds the visible surface of one column in a saved chunk: (color, height, is_water)
fn column(sections: &[ResolvedSection], x: usize, z: usize, ceiling: bool) -> Option<([u8; 3], i32, bool)> {
    let blocks = sections
        .iter()
        // In the Nether, start below the bedrock roof
        .filter(|s| !(ceiling && s.y * 16 > 120))
        .flat_map(|s| (0..16).rev().map(move |ly| (s.y * 16 + ly as i32, s.kind_at(x, ly, z))));
    surface_of(blocks, ceiling)
}

/// The visible surface from blocks listed top-down: (color, height, is_water).
/// With `ceiling`, skips down to the first open space first (the Nether roof).
fn surface_of(blocks: impl Iterator<Item = (i32, Kind)>, ceiling: bool) -> Option<([u8; 3], i32, bool)> {
    const WATER: [u8; 3] = [52, 94, 196];
    let mut searching_for_air = ceiling;
    let mut water_top: Option<i32> = None;
    for (y, kind) in blocks {
        if searching_for_air {
            if kind == Kind::Clear {
                searching_for_air = false;
            }
            continue;
        }
        match kind {
            Kind::Clear => {}
            Kind::Water => {
                water_top.get_or_insert(y);
            }
            Kind::Solid(color) => {
                return Some(match water_top {
                    // Blend the lake/sea floor into the water by depth
                    Some(top) => {
                        let depth = (top - y).clamp(1, 24) as f32;
                        let t = (0.45 + depth / 24.0 * 0.5).min(0.95);
                        (mix(color, WATER, t), top, true)
                    }
                    None => (color, y, false),
                });
            }
        }
    }
    water_top.map(|top| (mix([20, 30, 60], WATER, 0.9), top, true))
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    [0, 1, 2].map(|i| (a[i] as f32 * (1.0 - t) + b[i] as f32 * t) as u8)
}

// ─── Block colors ─────────────────────────────────────────────────────────────

const DYES: [(&str, [u8; 3]); 16] = [
    ("light_blue", [58, 175, 217]),
    ("light_gray", [142, 142, 134]),
    ("white", [233, 236, 236]),
    ("orange", [240, 118, 19]),
    ("magenta", [189, 68, 179]),
    ("yellow", [248, 198, 39]),
    ("lime", [112, 185, 25]),
    ("pink", [237, 141, 172]),
    ("gray", [62, 68, 71]),
    ("cyan", [21, 137, 145]),
    ("purple", [121, 42, 172]),
    ("blue", [53, 57, 157]),
    ("brown", [114, 71, 40]),
    ("green", [84, 109, 27]),
    ("red", [161, 39, 34]),
    ("black", [20, 21, 25]),
];

const WOODS: [(&str, [u8; 3]); 12] = [
    ("dark_oak", [66, 43, 20]),
    ("pale_oak", [228, 218, 216]),
    ("oak", [162, 130, 78]),
    ("spruce", [114, 84, 48]),
    ("birch", [192, 175, 121]),
    ("jungle", [160, 115, 80]),
    ("acacia", [168, 90, 50]),
    ("mangrove", [117, 54, 48]),
    ("cherry", [226, 178, 172]),
    ("bamboo", [193, 173, 80]),
    ("crimson", [101, 48, 70]),
    ("warped", [43, 104, 99]),
];

/// Maps a block id to how the map shows it
fn classify(id: &str) -> Kind {
    let name = id.strip_prefix("minecraft:").unwrap_or(id);
    let has = |s: &str| name.contains(s);

    // Look-through blocks: air, glass, small decorations and plants
    const CLEAR: [&str; 23] = [
        "air", "glass", "pane", "barrier", "structure_void", "torch", "rail", "button",
        "pressure_plate", "sign", "tripwire", "string", "lever", "ladder", "redstone_wire",
        "cobweb", "vine", "vines", "short_grass", "tall_grass", "fern", "dead_bush", "iron_bars",
    ];
    if name == "light" || CLEAR.iter().any(|c| name.ends_with(c)) && !has("block") {
        return Kind::Clear;
    }
    if has("water") || has("seagrass") || has("kelp") || name == "bubble_column" {
        return Kind::Water;
    }

    let c = |r: u8, g: u8, b: u8| Kind::Solid([r, g, b]);
    // Exact / specific matches first
    match name {
        "grass_block" => return c(109, 153, 60),
        "lava" => return c(207, 92, 20),
        "snow" | "snow_block" | "powder_snow" => return c(245, 250, 252),
        "ice" | "frosted_ice" => return c(160, 190, 245),
        "packed_ice" | "blue_ice" => return c(130, 165, 235),
        "sand" | "sandstone" | "cut_sandstone" | "smooth_sandstone" | "chiseled_sandstone" => return c(219, 207, 163),
        "suspicious_sand" => return c(210, 196, 150),
        "red_sand" | "red_sandstone" | "cut_red_sandstone" | "smooth_red_sandstone" => return c(190, 103, 33),
        "gravel" | "suspicious_gravel" => return c(136, 126, 126),
        "clay" => return c(160, 166, 179),
        "dirt" | "rooted_dirt" => return c(134, 96, 67),
        "coarse_dirt" => return c(119, 85, 59),
        "podzol" => return c(91, 63, 24),
        "mycelium" => return c(111, 99, 105),
        "farmland" => return c(118, 80, 50),
        "dirt_path" => return c(148, 122, 65),
        "mud" => return c(60, 57, 60),
        "moss_block" | "moss_carpet" => return c(89, 109, 45),
        "pale_moss_block" | "pale_moss_carpet" => return c(107, 112, 101),
        "netherrack" => return c(111, 54, 52),
        "soul_sand" | "soul_soil" => return c(81, 62, 50),
        "crimson_nylium" => return c(130, 31, 31),
        "warped_nylium" => return c(43, 114, 101),
        "nether_wart_block" => return c(115, 3, 2),
        "warped_wart_block" => return c(22, 119, 121),
        "glowstone" | "shroomlight" => return c(207, 160, 90),
        "magma_block" => return c(142, 63, 31),
        "end_stone" | "end_stone_bricks" => return c(219, 222, 158),
        "obsidian" | "crying_obsidian" => return c(20, 18, 30),
        "bedrock" => return c(85, 85, 85),
        "cactus" => return c(85, 127, 43),
        "pumpkin" | "carved_pumpkin" | "jack_o_lantern" => return c(198, 118, 24),
        "melon" => return c(111, 145, 30),
        "hay_block" => return c(166, 139, 12),
        "lily_pad" => return c(32, 128, 48),
        "sugar_cane" | "bamboo" => return c(120, 170, 70),
        "calcite" => return c(223, 224, 220),
        "tuff" => return c(108, 109, 102),
        "dripstone_block" | "pointed_dripstone" => return c(134, 107, 92),
        "prismarine" | "prismarine_bricks" | "dark_prismarine" => return c(76, 150, 140),
        "sculk" => return c(12, 29, 36),
        "amethyst_block" => return c(133, 97, 191),
        "terracotta" => return c(152, 94, 67),
        _ => {}
    }

    if has("leaves") {
        return match () {
            _ if has("birch") => c(96, 130, 60),
            _ if has("spruce") => c(60, 90, 60),
            _ if has("cherry") => c(229, 172, 194),
            _ if has("azalea") => c(90, 125, 50),
            _ if has("pale_oak") => c(140, 150, 140),
            _ if has("mangrove") => c(80, 120, 40),
            _ => c(56, 105, 38),
        };
    }
    // Dyed blocks: wool, concrete, carpets, beds, stained terracotta...
    if let Some((_, dye)) = DYES.iter().find(|(d, _)| name.starts_with(d)) {
        return if has("terracotta") {
            Kind::Solid(mix(*dye, [152, 94, 67], 0.55))
        } else {
            Kind::Solid(*dye)
        };
    }
    if let Some((_, wood)) = WOODS.iter().find(|(w, _)| name.starts_with(w)) {
        return Kind::Solid(*wood);
    }
    if has("deepslate") || has("blackstone") || has("basalt") {
        return c(70, 70, 76);
    }
    if has("copper") {
        return if has("oxidized") {
            c(82, 162, 132)
        } else if has("weathered") {
            c(108, 153, 110)
        } else if has("exposed") {
            c(161, 125, 103)
        } else {
            c(192, 107, 79)
        };
    }
    if has("quartz") || has("bone_block") {
        return c(235, 229, 222);
    }
    if has("brick") && !has("stone") {
        return c(150, 97, 83);
    }
    if has("mushroom") {
        return c(160, 80, 60);
    }
    if has("flower") || has("tulip") || has("orchid") || has("sapling") || has("bush") || has("roots") {
        return c(90, 140, 50);
    }
    if has("granite") {
        return c(149, 103, 85);
    }
    if has("diorite") {
        return c(188, 188, 188);
    }
    if has("andesite") {
        return c(136, 136, 136);
    }
    // Stone and everything else stone-like
    c(125, 125, 125)
}

#[cfg(test)]
mod tests {
    use super::*;


    pub(super) fn live_chunk_bytes_pub() -> Vec<u8> {
        live_chunk_bytes()
    }

    /// A live chunk as the plugin's DataOutputStream writes it
    fn live_chunk_bytes() -> Vec<u8> {
        let mut out = b"IGC\x01".to_vec();
        let palette = ["minecraft:grass_block", "minecraft:water", "minecraft:sand"];
        out.extend((palette.len() as u16).to_be_bytes());
        for name in palette {
            out.extend((name.len() as u16).to_be_bytes());
            out.extend(name.as_bytes());
        }
        for i in 0..256 {
            // Column 0 is a lake: 3 water over sand; the rest is grass at y=70
            let runs: &[(u16, i16, u16)] = if i == 0 { &[(1, 62, 3), (2, 59, 1)] } else { &[(0, 70, 1)] };
            out.extend((runs.len() as u16).to_be_bytes());
            for (index, top, len) in runs {
                out.extend(index.to_be_bytes());
                out.extend(top.to_be_bytes());
                out.extend(len.to_be_bytes());
            }
        }
        out
    }

    #[test]
    fn decodes_and_renders_live_chunks() {
        let columns = decode_live_chunk(&live_chunk_bytes()).unwrap();
        // Column 0: 3 water over sand -> water surface at the top of the water
        let (_, height, water) = columns[0].unwrap();
        assert!(water && height == 62);
        // The rest: grass at y=70
        let (_, height, water) = columns[1].unwrap();
        assert!(!water && height == 70);
        assert!(decode_live_chunk(b"IGC\x02").is_none(), "unknown versions are ignored");
        assert!(decode_live_chunk(b"IGC\x01\x00").is_none(), "truncated files are ignored");

        // A region the server never saved still gets a tile from live chunks alone
        let dir = std::env::temp_dir().join(format!("ingot-live-{}", std::process::id()));
        let region = live_dir(&dir, "minecraft:overworld").join("-1.0");
        std::fs::create_dir_all(&region).unwrap();
        std::fs::write(region.join("-1.3.bin"), live_chunk_bytes()).unwrap();
        let dims = dimensions(&dir);
        assert_eq!(dims[0].regions.len(), 1);
        assert_eq!((dims[0].regions[0].x, dims[0].regions[0].z), (-1, 0));

        let mut raster = Raster::empty();
        apply_live_chunks(&mut raster, &region);
        // Chunk -1 sits at local x 31; column 0 is the lake, column 1 is grass
        let lake = (3 * 16) * TILE + 31 * 16;
        assert!(raster.is_water[lake] && raster.heights[lake] == 62);
        assert!(!raster.is_water[lake + 1] && raster.heights[lake + 1] == 70);
        assert!(tile(&dir, "minecraft:overworld", -1, 0).unwrap().is_some());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn classifies_common_blocks() {
        assert!(classify("minecraft:air") == Kind::Clear);
        assert!(classify("minecraft:short_grass") == Kind::Clear);
        assert!(classify("minecraft:glass_pane") == Kind::Clear);
        assert!(classify("minecraft:water") == Kind::Water);
        assert!(classify("minecraft:grass_block") == Kind::Solid([109, 153, 60]));
        assert!(classify("minecraft:light_blue_wool") == Kind::Solid([58, 175, 217]));
        assert!(classify("minecraft:oak_log") == Kind::Solid([162, 130, 78]));
        assert!(classify("minecraft:dark_oak_planks") == Kind::Solid([66, 43, 20]));
    }

    /// Run with INGOT_TEST_SERVER_DIR=<server dir> cargo test -- --ignored
    #[test]
    #[ignore]
    fn renders_real_world() {
        let dir = PathBuf::from(std::env::var("INGOT_TEST_SERVER_DIR").unwrap());
        let dims = dimensions(&dir);
        for d in &dims {
            println!("{} regions={}", d.id, d.regions.len());
        }
        for d in &dims { for r in &d.regions { tile(&dir, &d.id, r.x, r.z).unwrap(); } }
        let overworld = &dims[0];
        let r = &overworld.regions[0];
        let start = std::time::Instant::now();
        let url = tile(&dir, &overworld.id, r.x, r.z).unwrap().unwrap();
        println!("tile r.{}.{} {} bytes in {:?}", r.x, r.z, url.len(), start.elapsed());
    }
}


#[cfg(test)]
mod perf {
    #[test]
    #[ignore]
    fn live_region_timing() {
        let dir = std::env::temp_dir().join(format!("ingot-perf-{}", std::process::id()));
        let region = super::live_dir(&dir, "minecraft:overworld").join("0.0");
        std::fs::create_dir_all(&region).unwrap();
        let bytes = super::tests::live_chunk_bytes_pub();
        for cx in 0..32 {
            for cz in 0..32 {
                std::fs::write(region.join(format!("{cx}.{cz}.bin")), &bytes).unwrap();
            }
        }
        for run in 0..3 {
            let t = std::time::Instant::now();
            let mut raster = super::Raster::empty();
            super::apply_live_chunks(&mut raster, &region);
            let applied = t.elapsed();
            let png = raster.to_png().unwrap();
            println!("run {run}: apply {:?}, png {:?} ({} KB)", applied, t.elapsed() - applied, png.len() / 1024);
        }
        let t = std::time::Instant::now();
        let url = super::tile(&dir, "minecraft:overworld", 0, 0).unwrap().unwrap();
        println!("tile() cold {:?} ({} KB data url)", t.elapsed(), url.len() / 1024);
        let t = std::time::Instant::now();
        super::tile(&dir, "minecraft:overworld", 0, 0).unwrap();
        println!("tile() cached {:?}", t.elapsed());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
