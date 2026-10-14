use super::discovery::{dimensions, region_dirs, tile};
use super::java::has_terrain;
use super::live::{apply_live_chunks, decode_live_chunk, live_dir};
use super::models::{level_name, mix, Kind, TILE};
use super::palette::classify;
use super::raster::Raster;
use std::path::PathBuf;

pub fn live_chunk_bytes() -> Vec<u8> {
    let mut out = b"IGC\x01".to_vec();
    let palette = ["minecraft:grass_block", "minecraft:water", "minecraft:sand"];
    out.extend((palette.len() as u16).to_be_bytes());
    for name in palette {
        out.extend((name.len() as u16).to_be_bytes());
        out.extend(name.as_bytes());
    }
    for i in 0..256 {
        // Column 0 is a lake: 3 water over sand; the rest is grass at y=70
        let runs: &[(u16, i16, u16)] = if i == 0 {
            &[(1, 62, 3), (2, 59, 1)]
        } else {
            &[(0, 70, 1)]
        };
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
    assert!(
        decode_live_chunk(b"IGC\x02").is_none(),
        "unknown versions are ignored"
    );
    assert!(
        decode_live_chunk(b"IGC\x01\x00").is_none(),
        "truncated files are ignored"
    );

    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    let region = live_dir(dir, "minecraft:overworld").join("-1.0");
    std::fs::create_dir_all(&region).unwrap();
    std::fs::write(region.join("-1.3.bin"), live_chunk_bytes()).unwrap();
    let dims = dimensions(dir);
    assert_eq!(dims[0].regions.len(), 1);
    assert_eq!((dims[0].regions[0].x, dims[0].regions[0].z), (-1, 0));

    let mut raster = Raster::empty();
    apply_live_chunks(&mut raster, &region);
    // Chunk -1 sits at local x 31; column 0 is the lake, column 1 is grass
    let lake = (3 * 16) * TILE + 31 * 16;
    assert!(raster.is_water[lake] && raster.heights[lake] == 62);
    assert!(!raster.is_water[lake + 1] && raster.heights[lake + 1] == 70);
    assert!(tile(dir, "minecraft:overworld", -1, 0).unwrap().is_some());
}

#[test]
fn classifies_common_blocks() {
    assert_eq!(classify("minecraft:air"), Kind::Clear);
    assert_eq!(classify("minecraft:short_grass"), Kind::Clear);
    assert_eq!(classify("minecraft:glass_pane"), Kind::Clear);
    assert_eq!(classify("minecraft:water"), Kind::Water);
    assert_eq!(
        classify("minecraft:grass_block"),
        Kind::Solid([109, 153, 60])
    );
    assert_eq!(
        classify("minecraft:light_blue_wool"),
        Kind::Solid([58, 175, 217])
    );
    assert_eq!(classify("minecraft:oak_log"), Kind::Solid([162, 130, 78]));
    assert_eq!(
        classify("minecraft:dark_oak_planks"),
        Kind::Solid([66, 43, 20])
    );
}

#[test]
fn mixes_colors_correctly() {
    let black = [0, 0, 0];
    let white = [200, 200, 200];
    let half = mix(black, white, 0.5);
    assert_eq!(half, [100, 100, 100]);
    let full = mix(black, white, 1.0);
    assert_eq!(full, white);
    let zero = mix(black, white, 0.0);
    assert_eq!(zero, black);
}

#[test]
fn resolves_level_name_with_fallbacks() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    // Default fallback when server.properties is missing
    assert_eq!(level_name(dir), "world");

    // Reading configured level name
    std::fs::write(
        dir.join("server.properties"),
        "motd=Ingot Server\nlevel-name=survival_world\ngamemode=survival\n",
    )
    .unwrap();
    assert_eq!(level_name(dir), "survival_world");

    // Reject path traversals
    std::fs::write(dir.join("server.properties"), "level-name=../traversal\n").unwrap();
    assert_eq!(level_name(dir), "world");
}

#[test]
fn verifies_terrain_generation_steps() {
    assert!(has_terrain(None));
    assert!(has_terrain(Some("minecraft:full")));
    assert!(has_terrain(Some("surface")));
    assert!(has_terrain(Some("minecraft:features")));
    assert!(has_terrain(Some("light")));
    assert!(!has_terrain(Some("minecraft:biomes")));
    assert!(!has_terrain(Some("structure_starts")));
}

#[test]
fn produces_valid_png_from_raster() {
    let mut raster = Raster::empty();
    raster.set(10, 10, Some(([100, 150, 200], 64, false)));
    raster.set(10, 11, Some(([120, 160, 210], 65, false)));
    let png = raster.to_png().unwrap();
    assert!(!png.is_empty());
    assert_eq!(&png[0..8], b"\x89PNG\r\n\x1a\n");
}

#[test]
fn constructs_candidate_region_dirs() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    let overworld_dirs = region_dirs(dir, "myworld", "minecraft:overworld");
    assert!(overworld_dirs
        .iter()
        .any(|d| d.ends_with("myworld\\region") || d.ends_with("myworld/region")));

    let nether_dirs = region_dirs(dir, "myworld", "minecraft:the_nether");
    assert!(nether_dirs
        .iter()
        .any(|d| d.ends_with("myworld_nether\\DIM-1\\region")
            || d.ends_with("myworld_nether/DIM-1/region")));
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
    for d in &dims {
        for r in &d.regions {
            tile(&dir, &d.id, r.x, r.z).unwrap();
        }
    }
    let overworld = &dims[0];
    let r = &overworld.regions[0];
    let start = std::time::Instant::now();
    let url = tile(&dir, &overworld.id, r.x, r.z).unwrap().unwrap();
    println!(
        "tile r.{}.{} {} bytes in {:?}",
        r.x,
        r.z,
        url.len(),
        start.elapsed()
    );
}

#[cfg(test)]
mod perf {
    #[test]
    #[ignore]
    fn live_region_timing() {
        let dir = std::env::temp_dir().join(format!("ingot-perf-{}", std::process::id()));
        let region = super::live_dir(&dir, "minecraft:overworld").join("0.0");
        std::fs::create_dir_all(&region).unwrap();
        let bytes = super::live_chunk_bytes();
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
            println!(
                "run {run}: apply {:?}, png {:?} ({} KB)",
                applied,
                t.elapsed() - applied,
                png.len() / 1024
            );
        }
        let t = std::time::Instant::now();
        let url = super::tile(&dir, "minecraft:overworld", 0, 0)
            .unwrap()
            .unwrap();
        println!(
            "tile() cold {:?} ({} KB data url)",
            t.elapsed(),
            url.len() / 1024
        );
        let t = std::time::Instant::now();
        super::tile(&dir, "minecraft:overworld", 0, 0).unwrap();
        println!("tile() cached {:?}", t.elapsed());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
