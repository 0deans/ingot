use super::models::{secs, MapRegion};
use super::palette::{classify, surface_of};
use super::raster::Raster;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::SystemTime;

/// Where the Ingot plugin writes chunks it read from server memory:
/// `.ingot/live/<namespace>/<dimension>/<rx>.<rz>/<cx>.<cz>.bin`
pub fn live_dir(server_dir: &Path, dimension: &str) -> PathBuf {
    let (ns, path) = dimension
        .split_once(':')
        .unwrap_or(("minecraft", dimension));
    server_dir.join(".ingot").join("live").join(ns).join(path)
}

/// Regions with live chunks. The plugin replaces files by renaming, which updates the
/// folder's time, so one stat per region tells whether anything in it changed.
pub fn list_live_regions(dir: &Path) -> Vec<MapRegion> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
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

/// A live chunk resolved to what the map shows in each column (index z * 16 + x)
pub type LiveSurface = [Option<([u8; 3], i32, bool)>; 256];

/// Resolved live chunks by file and its modification time: re-rendering a region then
/// only reads and decodes the chunks the plugin rewrote since, not all of them
pub type LiveChunkCache = Mutex<HashMap<PathBuf, (SystemTime, Arc<LiveSurface>)>>;
static LIVE_CHUNKS: LazyLock<LiveChunkCache> = LazyLock::new(|| Mutex::new(HashMap::new()));
/// About 12 MB of resolved chunks; cleared when full (rare: four fully explored regions)
const LIVE_CHUNK_CACHE: usize = 4096;

pub fn live_surface(path: &Path, modified: SystemTime) -> Option<Arc<LiveSurface>> {
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
pub fn decode_live_chunk(bytes: &[u8]) -> Option<Box<LiveSurface>> {
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
        let blocks = runs
            .iter()
            .flat_map(|&(kind, top, len)| (0..len).map(move |d| (top - d, kind)));
        // The plugin already starts below the Nether roof
        *column = surface_of(blocks, false);
    }
    Some(surface)
}

/// Draws the plugin's live chunks over the saved ones, where they're newer
pub fn apply_live_chunks(raster: &mut Raster, live_region: &Path) {
    let Ok(entries) = std::fs::read_dir(live_region) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some((cx, cz)) = name.strip_suffix(".bin").and_then(|n| n.split_once('.')) else {
            continue;
        };
        let (Ok(cx), Ok(cz)) = (cx.parse::<i32>(), cz.parse::<i32>()) else {
            continue;
        };
        let (lx, lz) = (cx.rem_euclid(32) as usize, cz.rem_euclid(32) as usize);
        let Some(modified) = entry.metadata().ok().and_then(|m| m.modified().ok()) else {
            continue;
        };
        let taken = secs(modified);
        // The server saved this chunk after the plugin captured it: the saved one wins
        if taken < raster.chunk_times[lz * 32 + lx] {
            continue;
        }
        let Some(surface) = live_surface(&entry.path(), modified) else {
            continue;
        };
        raster.chunk_times[lz * 32 + lx] = taken;
        for (i, column) in surface.iter().enumerate() {
            raster.set(lx * 16 + i % 16, lz * 16 + i / 16, *column);
        }
    }
}
