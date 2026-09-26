//! Top-down world map rendered straight from the world's Anvil region files.
//!
//! Each region (32x32 chunks) becomes a 512x512 PNG tile (1 px per block), cached next
//! to the world and re-rendered when the region file changes. Works for any server core
//! that writes Anvil (vanilla, Paper, Fabric, Pumpkin) - no map plugin needed.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

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

fn list_regions(dir: &Path) -> Vec<MapRegion> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    entries
        .flatten()
        .filter_map(|e| {
            let meta = e.metadata().ok().filter(|m| m.len() > 8192)?;
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs() as u32);
            let name = e.file_name().to_string_lossy().into_owned();
            let mut parts = name.strip_prefix("r.")?.strip_suffix(".mca")?.split('.');
            Some(MapRegion { x: parts.next()?.parse().ok()?, z: parts.next()?.parse().ok()?, modified })
        })
        .collect()
}

pub fn dimensions(server_dir: &Path) -> Vec<MapDimension> {
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
            let regions = region_dirs(server_dir, &level, &id)
                .iter()
                .map(|d| list_regions(d))
                .find(|r| !r.is_empty())?;
            Some(MapDimension { id, regions })
        })
        .collect()
}

/// Returns the tile as a PNG data URL, rendering (or re-rendering) it when needed
pub fn tile(server_dir: &Path, dimension: &str, x: i32, z: i32) -> Result<Option<String>, String> {
    let level = level_name(server_dir);
    let file_name = format!("r.{x}.{z}.mca");
    let Some(region_path) = region_dirs(server_dir, &level, dimension)
        .into_iter()
        .map(|d| d.join(&file_name))
        .find(|p| p.exists())
    else {
        return Ok(None);
    };

    let cache_dir = server_dir
        .join(".ingot")
        .join("map")
        .join(dimension.replace(':', "_"));
    let cache_path = cache_dir.join(format!("r.{x}.{z}.png"));
    let mtime = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let png = match (mtime(&cache_path), mtime(&region_path)) {
        (Some(cached), Some(source)) if cached >= source => {
            std::fs::read(&cache_path).map_err(|e| format!("Failed to read map tile: {e}"))?
        }
        _ => {
            let png = render_region(&region_path, dimension == "minecraft:the_nether")?;
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

fn read_chunks(region_path: &Path) -> Result<Vec<(usize, usize, Chunk)>, String> {
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
                chunks.push((i % 32, i / 32, chunk));
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

fn render_region(region_path: &Path, ceiling: bool) -> Result<Vec<u8>, String> {
    let chunks = read_chunks(region_path)?;
    let mut kind_cache: HashMap<String, Kind> = HashMap::new();
    let mut pixels = vec![0u8; TILE * TILE * 4];
    // Surface height per pixel, for relief shading; i32::MIN = no data / water
    let mut heights = vec![i32::MIN; TILE * TILE];
    let mut is_water = vec![false; TILE * TILE];

    for (cx, cz, chunk) in chunks {
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
                let px = cx * 16 + x;
                let pz = cz * 16 + z;
                let Some((color, height, water)) = column(&sections, x, z, ceiling) else { continue };
                let i = pz * TILE + px;
                pixels[i * 4..i * 4 + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
                heights[i] = height;
                is_water[i] = water;
            }
        }
    }

    // Relief: lighten blocks higher than their northern neighbour, darken lower ones
    for pz in 1..TILE {
        for px in 0..TILE {
            let i = pz * TILE + px;
            let north = heights[i - TILE];
            if heights[i] == i32::MIN || north == i32::MIN || is_water[i] {
                continue;
            }
            let factor = match heights[i].cmp(&north) {
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

/// Finds the visible surface of one column: (color, height, is_water)
fn column(sections: &[ResolvedSection], x: usize, z: usize, ceiling: bool) -> Option<([u8; 3], i32, bool)> {
    const WATER: [u8; 3] = [52, 94, 196];
    // In the Nether, start below the bedrock roof: skip down to the first open space
    let mut searching_for_air = ceiling;
    let mut water_top: Option<i32> = None;
    for section in sections {
        if ceiling && section.y * 16 > 120 {
            continue;
        }
        for ly in (0..16).rev() {
            let y = section.y * 16 + ly as i32;
            let kind = section.kind_at(x, ly, z);
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

