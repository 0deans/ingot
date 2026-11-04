use std::io::Write;
use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use super::dependencies::check_dependencies;
use super::file_name::ContentFileName;
use super::metadata::read_content_metadata;
use super::models::{
    ContentCategory, ContentDependency, ContentMetadata, ContentOrigin, ContentState,
    InstalledContent, ProviderSource, TrackedOrigin,
};
use super::tracking::{read_tracking, write_tracking};

#[test]
fn file_name_validation_and_disabled_state() {
    // Valid mod file names
    let mod_file =
        ContentFileName::parse("sodium-mc1.20.1-0.5.8.jar", ContentCategory::Mod).unwrap();
    assert_eq!(mod_file.as_str(), "sodium-mc1.20.1-0.5.8.jar");
    assert_eq!(mod_file.base_name(), "sodium-mc1.20.1-0.5.8.jar");
    assert!(!mod_file.is_disabled());

    let disabled_mod = mod_file.with_enabled_state(false);
    assert_eq!(disabled_mod.as_str(), "sodium-mc1.20.1-0.5.8.jar.disabled");
    assert_eq!(disabled_mod.base_name(), "sodium-mc1.20.1-0.5.8.jar");
    assert!(disabled_mod.is_disabled());

    let reenabled_mod = disabled_mod.with_enabled_state(true);
    assert_eq!(reenabled_mod.as_str(), "sodium-mc1.20.1-0.5.8.jar");
    assert!(!reenabled_mod.is_disabled());

    // Invalid mod file names
    assert!(ContentFileName::parse("invalid.zip", ContentCategory::Mod).is_err());
    assert!(ContentFileName::parse("../escape.jar", ContentCategory::Mod).is_err());
    assert!(ContentFileName::parse("sub/folder.jar", ContentCategory::Mod).is_err());
    assert!(ContentFileName::parse(".hidden.jar", ContentCategory::Mod).is_err());
    assert!(ContentFileName::parse("", ContentCategory::Mod).is_err());

    // Resource pack names
    assert!(ContentFileName::parse("Faithful.zip", ContentCategory::ResourcePack).is_ok());
    assert!(
        ContentFileName::parse("BareBones.zip.disabled", ContentCategory::ResourcePack).is_ok()
    );
    assert!(ContentFileName::parse("UnpackedFolder", ContentCategory::ResourcePack).is_ok());
    assert!(ContentFileName::parse("Bad.tar.gz", ContentCategory::ResourcePack).is_err());
}

#[test]
fn tracking_read_write_roundtrip() {
    let dir = tempdir().unwrap();
    let instance_dir = dir.path();

    let initial = read_tracking(instance_dir).unwrap();
    assert!(initial.is_empty());

    let mut map = std::collections::HashMap::new();
    map.insert(
        "sodium.jar".to_string(),
        TrackedOrigin {
            source: ProviderSource::Modrinth,
            project_id: "AANobbMI".to_string(),
            project_slug: Some("sodium".to_string()),
            version_id: "123".to_string(),
            version_number: "0.5.8".to_string(),
            title: "Sodium".to_string(),
            icon_url: Some("https://example.com/icon.png".to_string()),
        },
    );

    write_tracking(instance_dir, &map).unwrap();
    let loaded = read_tracking(instance_dir).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded.get("sodium.jar").unwrap().title, "Sodium");
}

#[test]
fn dependency_issue_checking() {
    let items = vec![
        InstalledContent {
            file_name: "iris.jar".to_string(),
            category: ContentCategory::Mod,
            state: ContentState::Enabled,
            size_bytes: 100,
            modified_at: 100,
            metadata: ContentMetadata {
                name: "Iris".to_string(),
                mod_id: Some("iris".to_string()),
                version: Some("1.6.0".to_string()),
                description: None,
                authors: vec![],
                icon_data_url: None,
                dependencies: vec![
                    ContentDependency {
                        id: "sodium".to_string(),
                        name: None,
                        required: true,
                    },
                    ContentDependency {
                        id: "optional_dep".to_string(),
                        name: None,
                        required: false,
                    },
                ],
            },
            origin: ContentOrigin::Unknown,
        },
        InstalledContent {
            file_name: "sodium.jar.disabled".to_string(),
            category: ContentCategory::Mod,
            state: ContentState::Disabled,
            size_bytes: 100,
            modified_at: 100,
            metadata: ContentMetadata {
                name: "Sodium".to_string(),
                mod_id: Some("sodium".to_string()),
                version: Some("0.5.8".to_string()),
                description: None,
                authors: vec![],
                icon_data_url: None,
                dependencies: vec![],
            },
            origin: ContentOrigin::Unknown,
        },
    ];

    let issues = check_dependencies(&items);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].mod_id, "iris");
    assert_eq!(issues[0].missing_dependency_id, "sodium");
    assert!(issues[0].disabled); // It is installed, but disabled!
}

#[test]
fn fabric_mod_metadata_parsing() {
    let dir = tempdir().unwrap();
    let jar_path = dir.path().join("test_fabric.jar");

    let file = std::fs::File::create(&jar_path).unwrap();
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default();

    zip.start_file("fabric.mod.json", options).unwrap();
    let json = serde_json::json!({
        "schemaVersion": 1,
        "id": "testmod",
        "name": "Test Mod",
        "version": "1.0.0",
        "description": "A fabric test mod",
        "authors": ["Author One", {"name": "Author Two"}],
        "depends": {
            "fabricloader": ">=0.14.0",
            "minecraft": "~1.20.1",
            "sodium": ">=0.5.0"
        }
    });
    zip.write_all(json.to_string().as_bytes()).unwrap();
    zip.finish().unwrap();

    let meta = read_content_metadata(&jar_path, false).unwrap();
    assert_eq!(meta.name, "Test Mod");
    assert_eq!(meta.mod_id.as_deref(), Some("testmod"));
    assert_eq!(meta.version.as_deref(), Some("1.0.0"));
    assert_eq!(meta.authors, vec!["Author One", "Author Two"]);
    assert_eq!(meta.dependencies.len(), 1);
    assert_eq!(meta.dependencies[0].id, "sodium");
}

#[test]
fn pack_mcmeta_parsing() {
    let dir = tempdir().unwrap();
    let zip_path = dir.path().join("test_pack.zip");

    let file = std::fs::File::create(&zip_path).unwrap();
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default();

    zip.start_file("pack.mcmeta", options).unwrap();
    let json = serde_json::json!({
        "pack": {
            "pack_format": 15,
            "description": "Test Resource Pack"
        }
    });
    zip.write_all(json.to_string().as_bytes()).unwrap();
    zip.finish().unwrap();

    let meta = read_content_metadata(&zip_path, true).unwrap();
    assert_eq!(meta.name, "test_pack");
    assert_eq!(meta.description.as_deref(), Some("Test Resource Pack"));
}

#[test]
fn mutation_set_enabled_and_delete() {
    let dir = tempdir().unwrap();
    let instance_dir = dir.path();
    let mods_dir = instance_dir.join("mods");
    std::fs::create_dir_all(&mods_dir).unwrap();

    let jar_name = "test-mod-1.0.jar";
    std::fs::write(mods_dir.join(jar_name), b"test").unwrap();

    // Toggle off
    let disabled =
        super::mutation::set_enabled(instance_dir, ContentCategory::Mod, jar_name, false).unwrap();
    assert_eq!(disabled, "test-mod-1.0.jar.disabled");
    assert!(!mods_dir.join(jar_name).exists());
    assert!(mods_dir.join("test-mod-1.0.jar.disabled").exists());

    // Toggle on
    let enabled =
        super::mutation::set_enabled(instance_dir, ContentCategory::Mod, &disabled, true).unwrap();
    assert_eq!(enabled, "test-mod-1.0.jar");
    assert!(mods_dir.join(jar_name).exists());

    // Conflict detection
    std::fs::write(mods_dir.join("test-mod-1.0.jar.disabled"), b"conflict").unwrap();
    let conflict_err =
        super::mutation::set_enabled(instance_dir, ContentCategory::Mod, jar_name, false);
    assert!(conflict_err.is_err());

    // Delete
    let deleted = super::mutation::delete_content(
        instance_dir,
        ContentCategory::Mod,
        &[
            jar_name.to_string(),
            "test-mod-1.0.jar.disabled".to_string(),
        ],
    )
    .unwrap();
    assert_eq!(deleted.len(), 2);
    assert!(!mods_dir.join(jar_name).exists());
    assert!(!mods_dir.join("test-mod-1.0.jar.disabled").exists());
}
