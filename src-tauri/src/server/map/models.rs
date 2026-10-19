use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use thiserror::Error;

pub const TILE: usize = 512;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Air and things the map should look through (glass, torches, tall grass...)
    Clear,
    Water,
    Solid([u8; 3]),
}

#[derive(Debug, Error)]
pub enum MapError {
    #[error("Failed to read region file {path}: {source}")]
    RegionRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to read map tile {path}: {source}")]
    TileRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to open Bedrock world {path}: {source}")]
    BedrockOpen {
        path: PathBuf,
        #[source]
        source: bedrock_world::BedrockWorldError,
    },
    #[error("Failed to query Bedrock chunks in {path}: {source}")]
    BedrockQuery {
        path: PathBuf,
        #[source]
        source: bedrock_world::BedrockWorldError,
    },
    #[error("Failed to encode map tile PNG: {0}")]
    PngEncoding(#[from] png::EncodingError),
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

pub fn secs(time: SystemTime) -> u32 {
    time.duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as u32)
}

pub fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

pub fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    [0, 1, 2].map(|i| (a[i] as f32 * (1.0 - t) + b[i] as f32 * t) as u8)
}
