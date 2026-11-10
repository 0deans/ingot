use super::models::{modified, Kind, MapError};
use super::palette::{classify, surface_of};
use super::raster::Raster;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

#[derive(Deserialize)]
pub struct Chunk {
    #[serde(rename = "Status")]
    pub status: Option<String>,
    #[serde(default)]
    pub sections: Vec<Section>,
}

#[derive(Deserialize)]
pub struct Section {
    #[serde(rename = "Y")]
    pub y: i8,
    pub block_states: Option<BlockStates>,
}

#[derive(Deserialize)]
pub struct BlockStates {
    pub palette: Vec<PaletteEntry>,
    pub data: Option<fastnbt::LongArray>,
}

#[derive(Deserialize)]
pub struct PaletteEntry {
    #[serde(rename = "Name")]
    pub name: String,
}

/// A section with its palette resolved to map kinds
pub struct ResolvedSection {
    pub y: i32,
    pub kinds: Vec<Kind>,
    pub data: Vec<i64>,
    pub bits: usize,
}

impl ResolvedSection {
    pub fn kind_at(&self, x: usize, y: usize, z: usize) -> Kind {
        if self.kinds.len() == 1 || self.data.is_empty() {
            return self.kinds[0];
        }
        let index = (y * 16 + z) * 16 + x;
        let per_long = 64 / self.bits;
        let long = self.data[index / per_long] as u64;
        let value = (long >> ((index % per_long) * self.bits)) & ((1u64 << self.bits) - 1);
        self.kinds
            .get(value as usize)
            .copied()
            .unwrap_or(Kind::Clear)
    }
}

/// Chunks of a region file with their stored generation step and save time
pub fn read_chunks(region_path: &Path) -> Result<Vec<(usize, usize, Chunk, u32)>, MapError> {
    let bytes = std::fs::read(region_path).map_err(|source| MapError::RegionRead {
        path: region_path.to_path_buf(),
        source,
    })?;
    if bytes.len() < 8192 {
        return Ok(Vec::new());
    }
    let mut chunks = Vec::new();
    for i in 0..1024 {
        let header = &bytes[i * 4..i * 4 + 4];
        let offset =
            ((header[0] as usize) << 16 | (header[1] as usize) << 8 | header[2] as usize) * 4096;
        if offset == 0 || offset + 5 > bytes.len() {
            continue;
        }
        // Second header table: when each chunk was last saved (unix seconds)
        let saved = u32::from_be_bytes(
            bytes[4096 + i * 4..4096 + i * 4 + 4]
                .try_into()
                .unwrap_or_default(),
        );
        let length =
            u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap_or_default()) as usize;
        let Some(payload) = bytes.get(offset + 5..offset + 4 + length) else {
            continue;
        };
        let mut raw = Vec::new();
        let ok = match bytes[offset + 4] {
            1 => flate2::read::GzDecoder::new(payload)
                .read_to_end(&mut raw)
                .is_ok(),
            2 => flate2::read::ZlibDecoder::new(payload)
                .read_to_end(&mut raw)
                .is_ok(),
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
pub fn has_terrain(status: Option<&str>) -> bool {
    let Some(status) = status else {
        return true;
    };
    let step = status.trim_start_matches("minecraft:");
    matches!(
        step,
        "surface" | "carvers" | "features" | "initialize_light" | "light" | "spawn" | "full"
    )
}

/// Parsed saved regions, so live updates don't re-read the whole region file each time
static SAVED_RASTERS: LazyLock<Mutex<Vec<(PathBuf, SystemTime, Raster)>>> =
    LazyLock::new(Default::default);
const SAVED_RASTER_CACHE: usize = 6;

/// The region as saved on disk (cached in memory until the file changes)
pub fn saved_raster(region_path: &Path, ceiling: bool) -> Result<Raster, MapError> {
    let time = modified(region_path).unwrap_or(SystemTime::UNIX_EPOCH);
    if let Ok(cache) = SAVED_RASTERS.lock() {
        if let Some((_, _, raster)) = cache
            .iter()
            .find(|(p, t, _)| p == region_path && *t == time)
        {
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

pub fn render_saved(region_path: &Path, ceiling: bool) -> Result<Raster, MapError> {
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
                    .map(|p| {
                        *kind_cache
                            .entry(p.name.clone())
                            .or_insert_with(|| classify(&p.name))
                    })
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
        sections.sort_unstable_by_key(|s| std::cmp::Reverse(s.y));

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

/// Finds the visible surface of one column in a saved chunk: (color, height, is_water)
pub fn column(
    sections: &[ResolvedSection],
    x: usize,
    z: usize,
    ceiling: bool,
) -> Option<([u8; 3], i32, bool)> {
    let blocks = sections
        .iter()
        // In the Nether, start below the bedrock roof
        .filter(|s| !(ceiling && s.y * 16 > 120))
        .flat_map(|s| {
            (0..16)
                .rev()
                .map(move |ly| (s.y * 16 + ly as i32, s.kind_at(x, ly, z)))
        });
    surface_of(blocks, ceiling)
}
