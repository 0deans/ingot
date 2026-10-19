use super::bedrock::bedrock_tile;
use super::discovery::tile_typed;
use super::java::read_chunks;
use super::models::MapError;
use super::raster::Raster;
use std::error::Error;

#[test]
fn missing_region_retains_path_and_io_cause() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("missing.mca");
    let error = match read_chunks(&path) {
        Ok(_) => panic!("missing region must fail"),
        Err(error) => error,
    };

    assert!(matches!(
        &error,
        MapError::RegionRead { path: failed, source }
            if failed == &path && source.kind() == std::io::ErrorKind::NotFound
    ));
    assert!(error.source().unwrap().is::<std::io::Error>());
}

#[test]
fn unreadable_cached_tile_retains_cache_path_and_io_cause() {
    let directory = tempfile::tempdir().unwrap();
    let region_dir = directory.path().join("world/region");
    std::fs::create_dir_all(&region_dir).unwrap();
    let region_path = region_dir.join("r.0.0.mca");
    let region = std::fs::File::create(region_path).unwrap();
    let old_time = std::fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH);
    region.set_times(old_time).unwrap();
    let cache_path = directory
        .path()
        .join(".ingot/map/minecraft_overworld/r.0.0.png");
    std::fs::create_dir_all(&cache_path).unwrap();

    let error = tile_typed(directory.path(), "minecraft:overworld", 0, 0).unwrap_err();

    assert!(matches!(
        &error,
        MapError::TileRead { path, .. } if path == &cache_path
    ));
    assert!(error.source().unwrap().is::<std::io::Error>());
}

#[test]
fn missing_bedrock_world_retains_open_operation_and_provider_cause() {
    let directory = tempfile::tempdir().unwrap();
    let world_path = directory.path().join("missing-world");

    let error =
        bedrock_tile(directory.path(), &world_path, "minecraft:overworld", 0, 0).unwrap_err();

    assert!(matches!(
        &error,
        MapError::BedrockOpen { path, .. } if path == &world_path
    ));
    assert!(error
        .source()
        .unwrap()
        .is::<bedrock_world::BedrockWorldError>());
}

#[test]
fn bedrock_open_failure_still_uses_existing_tile_cache() {
    let directory = tempfile::tempdir().unwrap();
    let cache_path = directory
        .path()
        .join(".ingot/map/minecraft_overworld/r.0.0.png");
    std::fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
    let png = Raster::empty().to_png().unwrap();
    std::fs::write(cache_path, png).unwrap();
    let world_path = directory.path().join("missing-world");

    let tile = bedrock_tile(directory.path(), &world_path, "minecraft:overworld", 0, 0)
        .unwrap()
        .unwrap();

    assert!(tile.starts_with("data:image/png;base64,iVBORw0KGgo"));
}

#[test]
fn invalid_raster_data_retains_png_encoding_cause() {
    let mut raster = Raster::empty();
    raster.pixels.clear();

    let error = raster.to_png().unwrap_err();

    assert!(matches!(&error, MapError::PngEncoding(_)));
    assert!(error.source().unwrap().is::<png::EncodingError>());
}
