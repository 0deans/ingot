use super::dimensions::{existing_dimension_paths, reset_dimension};
use super::discovery::get_instance_worlds;
use super::level_dat::parse_level_dat;
use super::models::{validate_world_folder_name, WorldDimension};
use super::transfer::{backup_world, export_world, import_world, list_world_backups};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use tempfile::tempdir;

fn create_dummy_level_dat(level_name: &str, game_type: i32, hardcore: i8, seed: i64) -> Vec<u8> {
    let mut data = HashMap::new();
    data.insert(
        "LevelName".to_string(),
        fastnbt::Value::String(level_name.to_string()),
    );
    data.insert("GameType".to_string(), fastnbt::Value::Int(game_type));
    data.insert("hardcore".to_string(), fastnbt::Value::Byte(hardcore));
    data.insert(
        "LastPlayed".to_string(),
        fastnbt::Value::Long(1700000000000),
    );

    let mut wgs = HashMap::new();
    wgs.insert("seed".to_string(), fastnbt::Value::Long(seed));
    data.insert(
        "WorldGenSettings".to_string(),
        fastnbt::Value::Compound(wgs),
    );

    let mut root = HashMap::new();
    root.insert("Data".to_string(), fastnbt::Value::Compound(data));

    let nbt_bytes = fastnbt::to_bytes(&fastnbt::Value::Compound(root)).unwrap();

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&nbt_bytes).unwrap();
    encoder.finish().unwrap()
}

#[test]
fn test_validate_world_folder_name() {
    assert!(validate_world_folder_name("My World").is_ok());
    assert!(validate_world_folder_name("Survival-1.20").is_ok());
    assert!(validate_world_folder_name("").is_err());
    assert!(validate_world_folder_name(".hidden").is_err());
    assert!(validate_world_folder_name("qp_world").is_err());
    assert!(validate_world_folder_name("test/world").is_err());
    assert!(validate_world_folder_name("test\\world").is_err());
    assert!(validate_world_folder_name("CON").is_err());
    assert!(validate_world_folder_name("NUL").is_err());
    assert!(validate_world_folder_name("COM1").is_err());
}

#[test]
fn test_parse_level_dat_survival() {
    let bytes = create_dummy_level_dat("Test World", 0, 0, 9876543210123);
    let parsed = parse_level_dat(&bytes, "Fallback").unwrap();

    assert_eq!(parsed.display_name, "Test World");
    assert_eq!(parsed.game_mode, "Survival");
    assert!(!parsed.is_hardcore);
    assert_eq!(parsed.seed, Some("9876543210123".to_string()));
    assert_eq!(parsed.last_played, Some(1700000000));
}

#[test]
fn test_parse_level_dat_hardcore() {
    let bytes = create_dummy_level_dat("Hardcore Run", 0, 1, -123456789);
    let parsed = parse_level_dat(&bytes, "Fallback").unwrap();

    assert_eq!(parsed.display_name, "Hardcore Run");
    assert_eq!(parsed.game_mode, "Hardcore");
    assert!(parsed.is_hardcore);
    assert_eq!(parsed.seed, Some("-123456789".to_string()));
}

#[test]
fn test_dimension_reset() {
    let dir = tempdir().unwrap();
    let world_dir = dir.path().join("saves").join("TestWorld");
    fs::create_dir_all(&world_dir).unwrap();

    let level_dat = create_dummy_level_dat("TestWorld", 0, 0, 12345);
    fs::write(world_dir.join("level.dat"), level_dat).unwrap();

    // Create nether directory
    let nether_dir = world_dir.join("DIM-1");
    fs::create_dir_all(&nether_dir).unwrap();
    fs::write(nether_dir.join("test_chunk.mca"), b"dummy chunk").unwrap();

    let existing = existing_dimension_paths(&world_dir, WorldDimension::Nether);
    assert_eq!(existing.len(), 1);

    // Reset Nether
    let res = reset_dimension(&world_dir, WorldDimension::Nether).unwrap();
    assert_eq!(res.dimension, WorldDimension::Nether);
    assert_eq!(res.reset_paths, vec!["DIM-1"]);
    assert!(!nether_dir.exists());

    // Check that it was preserved in .ingot-trash
    let trash_path = res.backup_path.unwrap();
    assert!(std::path::Path::new(&trash_path).exists());
}

#[test]
fn test_export_backup_import_roundtrip() {
    let dir = tempdir().unwrap();
    let instance_dir = dir.path().join("instance");
    let saves_dir = instance_dir.join("saves");
    let world_dir = saves_dir.join("OriginalWorld");
    fs::create_dir_all(&world_dir).unwrap();

    let level_dat = create_dummy_level_dat("OriginalWorld", 1, 0, 424242);
    fs::write(world_dir.join("level.dat"), &level_dat).unwrap();
    fs::write(world_dir.join("test_file.txt"), b"saved data").unwrap();

    // 1. Export world to zip
    let export_zip = dir.path().join("exported.zip");
    export_world(&world_dir, &export_zip).unwrap();
    assert!(export_zip.is_file());

    // 2. Backup world
    let backup_info = backup_world(&instance_dir, "OriginalWorld").unwrap();
    assert!(std::path::Path::new(&backup_info.file_path).is_file());

    let backups = list_world_backups(&instance_dir, Some("OriginalWorld")).unwrap();
    assert_eq!(backups.len(), 1);

    // 3. Import world from exported zip into a second instance
    let instance2_saves = dir.path().join("instance2").join("saves");
    let imported_folder = import_world(&instance2_saves, &export_zip).unwrap();

    let imported_world_dir = instance2_saves.join(&imported_folder);
    assert!(imported_world_dir.join("level.dat").is_file());
    assert_eq!(
        fs::read_to_string(imported_world_dir.join("test_file.txt")).unwrap(),
        "saved data"
    );

    // 4. Discover imported world
    let worlds = get_instance_worlds(&dir.path().join("instance2"), "inst2", "Instance 2").unwrap();
    assert_eq!(worlds.len(), 1);
    assert_eq!(worlds[0].display_name, "OriginalWorld");
    assert_eq!(worlds[0].seed, Some("424242".to_string()));
    assert_eq!(worlds[0].game_mode, "Creative");
}

#[test]
fn test_parse_level_dat_random_seed() {
    let mut data = HashMap::new();
    data.insert(
        "LevelName".to_string(),
        fastnbt::Value::String("Beta World".to_string()),
    );
    data.insert("GameType".to_string(), fastnbt::Value::Int(0));
    data.insert("RandomSeed".to_string(), fastnbt::Value::Long(1122334455));

    let mut root = HashMap::new();
    root.insert("Data".to_string(), fastnbt::Value::Compound(data));

    let nbt_bytes = fastnbt::to_bytes(&fastnbt::Value::Compound(root)).unwrap();
    let parsed = parse_level_dat(&nbt_bytes, "Fallback").unwrap();
    assert_eq!(parsed.seed, Some("1122334455".to_string()));
}

#[test]
fn test_decoupled_world_gen_settings_seed() {
    let dir = tempdir().unwrap();
    let saves_dir = dir.path().join("saves");
    let world_dir = saves_dir.join("ModernWorld");
    fs::create_dir_all(&world_dir).unwrap();

    // level.dat with no seed tag (modern decoupling)
    let mut data = HashMap::new();
    data.insert(
        "LevelName".to_string(),
        fastnbt::Value::String("Modern World".to_string()),
    );
    data.insert("GameType".to_string(), fastnbt::Value::Int(0));
    let mut root = HashMap::new();
    root.insert("Data".to_string(), fastnbt::Value::Compound(data));
    let level_dat = fastnbt::to_bytes(&fastnbt::Value::Compound(root)).unwrap();
    fs::write(world_dir.join("level.dat"), level_dat).unwrap();

    // data/minecraft/world_gen_settings.dat
    let wgs_dir = world_dir.join("data").join("minecraft");
    fs::create_dir_all(&wgs_dir).unwrap();
    let mut wgs_data = HashMap::new();
    wgs_data.insert(
        "seed".to_string(),
        fastnbt::Value::Long(7640191713413565640),
    );
    let mut wgs_root = HashMap::new();
    wgs_root.insert("data".to_string(), fastnbt::Value::Compound(wgs_data));
    let wgs_bytes = fastnbt::to_bytes(&fastnbt::Value::Compound(wgs_root)).unwrap();
    fs::write(wgs_dir.join("world_gen_settings.dat"), wgs_bytes).unwrap();

    let worlds = get_instance_worlds(dir.path(), "inst-id", "Instance Name").unwrap();
    assert_eq!(worlds.len(), 1);
    assert_eq!(worlds[0].display_name, "Modern World");
    assert_eq!(worlds[0].seed, Some("7640191713413565640".to_string()));
}
