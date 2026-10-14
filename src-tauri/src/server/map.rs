//! Top-down world map rendered from the world's Anvil region files, plus live chunks
//! from Ingot's companion plugin when it's installed.

pub mod bedrock;
pub mod discovery;
pub mod java;
pub mod live;
pub mod models;
pub mod palette;
pub mod raster;

#[cfg(test)]
mod tests;

pub use discovery::{dimensions, region_dirs, tile, tile_typed};
pub use models::{level_name, MapDimension, MapError, MapRegion};
