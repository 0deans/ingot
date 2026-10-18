//! Comprehensive hermetic tests for launcher discovery, version ID parsing, and instance importing.

fn import_options(source: &std::path::Path) -> ImportInstanceOptions {
    ImportInstanceOptions {
        source_path: source.to_string_lossy().into_owned(),
        launcher_id: "custom".into(),
        name: "Imported".into(),
        game_version: "1.20.1".into(),
        loader: ModLoaderType::Vanilla,
        loader_version: None,
        copy_mods: true,
        copy_configs: true,
        copy_saves: true,
        copy_resource_packs: true,
        copy_options: true,
        copy_servers: true,
        copy_screenshots: true,
        copy_extra_data: true,
    }
}

#[test]
fn staged_import_publishes_complete_files_and_matching_metadata() {
    use crate::minecraft::{instance::InstanceConfig, metadata::MetadataRepository};
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let instances = destination.path().join("instances");
    fs::create_dir_all(source.path().join("mods")).unwrap();
    fs::write(source.path().join("mods/test.jar"), b"jar").unwrap();
    let repository = MetadataRepository::with_deletion_directory(
        destination.path().join("instances.json"),
        instances.clone(),
        |config: &InstanceConfig| &config.id,
    );
    let report = super::execute::execute_import_instance(
        &instances,
        import_options(source.path()),
        |stage, config| {
            assert_eq!(fs::read(stage.join("mods/test.jar")).unwrap(), b"jar");
            assert!(!instances.join(&config.id).exists());
            let target = instances.join(&config.id);
            repository.publish(stage, &target, config, |_, _| Ok(()))
        },
    )
    .unwrap();
    assert_eq!(report.mods_copied, 1);
    assert_eq!(repository.load().unwrap()[0].id, report.instance_id);
    assert_eq!(
        fs::read(instances.join(report.instance_id).join("mods/test.jar")).unwrap(),
        b"jar"
    );
    assert_eq!(fs::read_dir(instances).unwrap().count(), 1);
}

#[test]
fn unreadable_metadata_after_failed_import_retains_material_until_recovery() {
    use crate::minecraft::{instance::InstanceConfig, metadata::MetadataRepository};
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    fs::write(source.path().join("options.txt"), b"original").unwrap();
    let instances = destination.path().join("instances");
    let metadata = destination.path().join("instances.json");
    let repository = MetadataRepository::with_deletion_directory(
        metadata.clone(),
        instances.clone(),
        |config: &InstanceConfig| &config.id,
    );
    let result = super::execute::execute_import_instance(
        &instances,
        import_options(source.path()),
        |stage, config| {
            let target = instances.join(&config.id);
            repository.publish(stage, &target, config, |_, _| {
                // Inject failure after the repository read, before its atomic metadata replace.
                fs::create_dir(&metadata).unwrap();
                Ok(())
            })
        },
    );
    assert!(matches!(result, Err(super::ImportError::Metadata(_))));
    assert_eq!(fs::read_dir(&instances).unwrap().count(), 1);
    let receipt = metadata.with_extension("publication.json");
    assert!(receipt.is_file());
    assert!(repository.load().is_err());
    fs::remove_dir(&metadata).unwrap();
    assert!(repository.load().unwrap().is_empty());
    assert_eq!(fs::read_dir(instances).unwrap().count(), 0);
    assert!(!receipt.exists());
    assert_eq!(
        fs::read(source.path().join("options.txt")).unwrap(),
        b"original"
    );
}

#[test]
fn invalid_selected_category_never_reaches_publication() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    fs::write(source.path().join("config"), b"not a directory").unwrap();
    let result = super::execute::execute_import_instance(
        destination.path(),
        import_options(source.path()),
        |_, _| panic!("invalid content must not publish"),
    );
    assert!(matches!(result, Err(super::ImportError::Io(_))));
    assert_eq!(fs::read_dir(destination.path()).unwrap().count(), 0);
}

#[test]
fn import_source_cannot_recursively_include_its_stage() {
    let source = tempfile::tempdir().unwrap();
    let destination = source.path().join("instances");
    let result = super::execute::execute_import_instance(
        &destination,
        import_options(source.path()),
        |_, _| panic!("overlapping directories must not publish"),
    );
    assert!(matches!(result, Err(super::ImportError::OverlappingPaths)));
    assert_eq!(fs::read_dir(destination).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn linked_import_content_never_publishes_or_changes_external_data() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    fs::write(external.path().join("secret"), b"external").unwrap();
    std::os::unix::fs::symlink(external.path(), source.path().join("config")).unwrap();
    let result = super::execute::execute_import_instance(
        destination.path(),
        import_options(source.path()),
        |_, _| panic!("linked content must not publish"),
    );
    assert!(matches!(result, Err(super::ImportError::Io(_))));
    assert_eq!(fs::read_dir(destination.path()).unwrap().count(), 0);
    assert_eq!(
        fs::read(external.path().join("secret")).unwrap(),
        b"external"
    );
}

use super::{
    discovery::detect_custom_instance,
    execute::copy_instance_contents,
    formats::{
        atlauncher::scan_atlauncher_instances, curseforge::scan_curseforge_instances,
        legacy::scan_legacy_instances, modrinth::scan_modrinth_instances,
        vanilla::scan_vanilla_instances,
    },
    models::ImportInstanceOptions,
    version_id::parse_vanilla_version_id,
};
use crate::minecraft::instance::ModLoaderType;
use std::fs;

#[test]
fn test_parse_vanilla_version_id_variants() {
    // Fabric
    assert_eq!(
        parse_vanilla_version_id("fabric-loader-0.15.0-1.20.1"),
        (
            "1.20.1".to_string(),
            ModLoaderType::Fabric,
            Some("0.15.0".to_string())
        )
    );
    assert_eq!(
        parse_vanilla_version_id("fabric-loader-1.20.1"),
        ("1.20.1".to_string(), ModLoaderType::Fabric, None)
    );

    // Quilt
    assert_eq!(
        parse_vanilla_version_id("quilt-loader-0.20.0-1.20.1"),
        (
            "1.20.1".to_string(),
            ModLoaderType::Quilt,
            Some("0.20.0".to_string())
        )
    );

    // Forge
    assert_eq!(
        parse_vanilla_version_id("1.20.1-forge-47.2.0"),
        (
            "1.20.1".to_string(),
            ModLoaderType::Forge,
            Some("47.2.0".to_string())
        )
    );
    assert_eq!(
        parse_vanilla_version_id("forge-1.20.1-47.2.0"),
        (
            "1.20.1".to_string(),
            ModLoaderType::Forge,
            Some("47.2.0".to_string())
        )
    );

    // NeoForge
    assert_eq!(
        parse_vanilla_version_id("neoforge-20.4.80"),
        (
            "1.21.1".to_string(),
            ModLoaderType::NeoForge,
            Some("20.4.80".to_string())
        )
    );

    // Vanilla
    assert_eq!(
        parse_vanilla_version_id("1.20.4"),
        ("1.20.4".to_string(), ModLoaderType::Vanilla, None)
    );
}

#[test]
fn test_scan_curseforge_instances_fixture() {
    let temp = tempfile::tempdir().expect("tempdir");
    let inst_dir = temp.path().join("MyCurseInstance");
    fs::create_dir_all(&inst_dir).expect("create_dir");
    fs::create_dir_all(inst_dir.join("mods")).expect("create mods");
    fs::write(inst_dir.join("mods").join("test_mod.jar"), b"jar").expect("write mod");
    fs::create_dir_all(inst_dir.join("saves").join("NewWorld")).expect("create save");
    fs::create_dir_all(inst_dir.join("screenshots")).expect("create screenshots");
    fs::write(
        inst_dir.join("screenshots").join("screen.png"),
        b"screenshot",
    )
    .expect("write screenshot");

    let manifest_json = r#"{
        "name": "My CursePack",
        "gameVersion": "1.20.1",
        "baseModLoader": {
            "name": "fabric-0.15.11",
            "type": 4
        }
    }"#;
    fs::write(inst_dir.join("minecraftinstance.json"), manifest_json).expect("write manifest");

    let instances = scan_curseforge_instances(temp.path());
    assert_eq!(instances.len(), 1);
    let inst = &instances[0];
    assert_eq!(inst.name, "My CursePack");
    assert_eq!(inst.launcher_id, "curseforge");
    assert_eq!(inst.game_version, "1.20.1");
    assert_eq!(inst.loader, ModLoaderType::Fabric);
    assert_eq!(inst.loader_version, Some("0.15.11".to_string()));
    assert_eq!(inst.mods_count, 1);
    assert_eq!(inst.saves_count, 1);
    assert_eq!(inst.screenshots_count, 1);
}

#[test]
fn test_scan_modrinth_instances_fixture() {
    let temp = tempfile::tempdir().expect("tempdir");
    let prof_dir = temp.path().join("MyModrinthProfile");
    fs::create_dir_all(&prof_dir).expect("create_dir");
    fs::create_dir_all(prof_dir.join("mods")).expect("create mods");
    fs::write(prof_dir.join("mods").join("sodium.jar"), b"jar").expect("write mod");

    let profile_json = r#"{
        "name": "Sodium Testing",
        "game_version": "1.20.4",
        "loader": "fabric",
        "loader_version": "0.15.3",
        "icon": "https://example.com/icon.png"
    }"#;
    fs::write(prof_dir.join("profile.json"), profile_json).expect("write profile");

    let instances = scan_modrinth_instances(temp.path());
    assert_eq!(instances.len(), 1);
    let inst = &instances[0];
    assert_eq!(inst.name, "Sodium Testing");
    assert_eq!(inst.launcher_id, "modrinth");
    assert_eq!(inst.game_version, "1.20.4");
    assert_eq!(inst.loader, ModLoaderType::Fabric);
    assert_eq!(inst.loader_version, Some("0.15.3".to_string()));
    assert_eq!(inst.mods_count, 1);
    assert_eq!(inst.saves_count, 0);
}

#[test]
fn test_scan_atlauncher_instances_fixture() {
    let temp = tempfile::tempdir().expect("tempdir");
    let inst_dir = temp.path().join("MyATLauncherInstance");
    fs::create_dir_all(&inst_dir).expect("create_dir");

    let instance_json = r#"{
        "name": "Vanilla Plus",
        "version": "1.19.2",
        "loader": "forge",
        "loaderVersion": "43.3.0"
    }"#;
    fs::write(inst_dir.join("instance.json"), instance_json).expect("write instance.json");

    let instances = scan_atlauncher_instances(temp.path());
    assert_eq!(instances.len(), 1);
    let inst = &instances[0];
    assert_eq!(inst.name, "Vanilla Plus");
    assert_eq!(inst.launcher_id, "atlauncher");
    assert_eq!(inst.game_version, "1.19.2");
    assert_eq!(inst.loader, ModLoaderType::Forge);
    assert_eq!(inst.loader_version, Some("43.3.0".to_string()));
}

#[test]
fn test_scan_vanilla_instances_fixture() {
    let temp = tempfile::tempdir().expect("tempdir");
    let profiles_json = r#"{
        "profiles": {
            "profile1": {
                "name": "Fabric 1.20.1",
                "lastVersionId": "fabric-loader-0.14.22-1.20.1",
                "icon": "Furnace"
            }
        }
    }"#;
    fs::write(temp.path().join("launcher_profiles.json"), profiles_json)
        .expect("write launcher_profiles");

    let instances = scan_vanilla_instances(temp.path());
    assert_eq!(instances.len(), 1);
    let inst = &instances[0];
    assert_eq!(inst.name, "Fabric 1.20.1");
    assert_eq!(inst.launcher_id, "vanilla");
    assert_eq!(inst.game_version, "1.20.1");
    assert_eq!(inst.loader, ModLoaderType::Fabric);
    assert_eq!(inst.loader_version, Some("0.14.22".to_string()));
}

#[test]
fn test_scan_legacy_instances_fixture() {
    let temp = tempfile::tempdir().expect("tempdir");
    let ver_dir = temp.path().join("versions").join("OptiFine_1.20.1");
    fs::create_dir_all(&ver_dir).expect("create_dir");

    let ver_json = r#"{
        "id": "1.20.1-OptiFine_HD_U_I5",
        "jar": "1.20.1",
        "mainClass": "net.minecraft.client.main.Main"
    }"#;
    fs::write(ver_dir.join("OptiFine_1.20.1.json"), ver_json).expect("write ver_json");

    let instances = scan_legacy_instances(temp.path());
    assert_eq!(instances.len(), 1);
    let inst = &instances[0];
    assert_eq!(inst.name, "OptiFine_1.20.1 (LegacyLauncher)");
    assert_eq!(inst.launcher_id, "legacylauncher");
    assert_eq!(inst.game_version, "1.20.1");
    assert_eq!(inst.loader, ModLoaderType::Vanilla);
}

#[test]
fn test_detect_custom_instance_fallback_and_missing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let custom_dir = temp.path().join("MyRawInstance");
    fs::create_dir_all(&custom_dir).expect("create_dir");
    fs::create_dir_all(custom_dir.join("mods")).expect("create mods");
    fs::write(custom_dir.join("mods").join("mod.jar"), b"jar").expect("write mod");

    let inst = detect_custom_instance(&custom_dir.to_string_lossy()).expect("detect success");
    assert_eq!(inst.name, "MyRawInstance");
    assert_eq!(inst.launcher_id, "custom");
    assert_eq!(inst.loader, ModLoaderType::Fabric);
    assert_eq!(inst.mods_count, 1);

    let nonexistent = temp.path().join("does_not_exist");
    assert!(detect_custom_instance(&nonexistent.to_string_lossy()).is_err());
}

#[test]
fn test_copy_instance_contents_full_and_ignored() {
    let temp_src = tempfile::tempdir().expect("src");
    let temp_dst = tempfile::tempdir().expect("dst");
    let src = temp_src.path();
    let dst = temp_dst.path();

    // Create source files
    fs::create_dir_all(src.join("mods")).unwrap();
    fs::write(src.join("mods").join("foo.jar"), b"mod").unwrap();

    fs::create_dir_all(src.join("config")).unwrap();
    fs::write(src.join("config").join("mod.cfg"), b"config").unwrap();

    fs::create_dir_all(src.join("saves").join("world1")).unwrap();
    fs::write(src.join("saves").join("world1").join("level.dat"), b"dat").unwrap();

    fs::create_dir_all(src.join("resourcepacks")).unwrap();
    fs::write(src.join("resourcepacks").join("pack.zip"), b"pack").unwrap();

    fs::write(src.join("options.txt"), b"fov:90").unwrap();
    fs::write(src.join("servers.dat"), b"servers").unwrap();

    fs::create_dir_all(src.join("screenshots")).unwrap();
    fs::write(src.join("screenshots").join("1.png"), b"shot").unwrap();

    // Extra data
    fs::create_dir_all(src.join("schematics")).unwrap();
    fs::write(src.join("schematics").join("house.schem"), b"house").unwrap();

    // Ignored runtime directories
    fs::create_dir_all(src.join("logs")).unwrap();
    fs::write(src.join("logs").join("latest.log"), b"log").unwrap();
    fs::create_dir_all(src.join("assets").join("indexes")).unwrap();
    fs::write(
        src.join("assets").join("indexes").join("1.20.json"),
        b"index",
    )
    .unwrap();

    let options = ImportInstanceOptions {
        source_path: src.to_string_lossy().to_string(),
        launcher_id: "custom".to_string(),
        name: "Test Import".to_string(),
        game_version: "1.20.1".to_string(),
        loader: ModLoaderType::Fabric,
        loader_version: None,
        copy_mods: true,
        copy_configs: true,
        copy_saves: true,
        copy_resource_packs: true,
        copy_options: true,
        copy_servers: true,
        copy_screenshots: true,
        copy_extra_data: true,
    };

    let counts = copy_instance_contents(src, dst, &options).expect("copy success");
    assert_eq!(counts.mods, 1);
    assert_eq!(counts.saves, 1);

    assert!(dst.join("mods").join("foo.jar").exists());
    assert!(dst.join("config").join("mod.cfg").exists());
    assert!(dst.join("saves").join("world1").join("level.dat").exists());
    assert!(dst.join("resourcepacks").join("pack.zip").exists());
    assert!(dst.join("options.txt").exists());
    assert!(dst.join("servers.dat").exists());
    assert!(dst.join("screenshots").join("1.png").exists());
    assert!(dst.join("schematics").join("house.schem").exists());

    // Verify ignored entries were skipped
    assert!(!dst.join("logs").exists());
    assert!(!dst.join("assets").exists());
}

#[test]
fn test_copy_instance_contents_selective() {
    let temp_src = tempfile::tempdir().expect("src");
    let temp_dst = tempfile::tempdir().expect("dst");
    let src = temp_src.path();
    let dst = temp_dst.path();

    fs::create_dir_all(src.join("mods")).unwrap();
    fs::write(src.join("mods").join("foo.jar"), b"mod").unwrap();

    fs::create_dir_all(src.join("saves").join("world1")).unwrap();
    fs::write(src.join("saves").join("world1").join("level.dat"), b"dat").unwrap();

    let options = ImportInstanceOptions {
        source_path: src.to_string_lossy().to_string(),
        launcher_id: "custom".to_string(),
        name: "Selective Import".to_string(),
        game_version: "1.20.1".to_string(),
        loader: ModLoaderType::Vanilla,
        loader_version: None,
        copy_mods: false,
        copy_configs: false,
        copy_saves: false,
        copy_resource_packs: false,
        copy_options: false,
        copy_servers: false,
        copy_screenshots: false,
        copy_extra_data: false,
    };

    let counts = copy_instance_contents(src, dst, &options).expect("copy success");
    assert_eq!(counts.mods, 0);
    assert_eq!(counts.saves, 0);
    assert!(!dst.join("mods").exists());
    assert!(!dst.join("saves").exists());
}

#[test]
fn test_copy_instance_contents_propagates_config_and_save_errors() {
    let temp_src = tempfile::tempdir().expect("src");
    let temp_dst = tempfile::tempdir().expect("dst");
    let src = temp_src.path();
    let dst = temp_dst.path();

    // Create a source config file
    fs::create_dir_all(src.join("config")).unwrap();
    fs::write(src.join("config").join("bad.cfg"), b"cfg data").unwrap();

    // Make the destination config path a file so that creating config directory fails
    fs::write(dst.join("config"), b"blocking file").unwrap();

    let options = ImportInstanceOptions {
        source_path: src.to_string_lossy().to_string(),
        launcher_id: "custom".to_string(),
        name: "Failing Import".to_string(),
        game_version: "1.20.1".to_string(),
        loader: ModLoaderType::Vanilla,
        loader_version: None,
        copy_mods: false,
        copy_configs: true,
        copy_saves: false,
        copy_resource_packs: false,
        copy_options: false,
        copy_servers: false,
        copy_screenshots: false,
        copy_extra_data: false,
    };

    let res = copy_instance_contents(src, dst, &options);
    assert!(res.is_err(), "must propagate config copy failure");
}

#[test]
fn test_execute_import_instance_cleans_up_on_metadata_failure() {
    let temp_src = tempfile::tempdir().expect("src");
    let temp_instances = tempfile::tempdir().expect("instances");
    let src = temp_src.path();
    let instances_dir = temp_instances.path();

    fs::create_dir_all(src.join("mods")).unwrap();
    fs::write(src.join("mods").join("test.jar"), b"jar").unwrap();

    let options = ImportInstanceOptions {
        source_path: src.to_string_lossy().to_string(),
        launcher_id: "custom".to_string(),
        name: "Failing Metadata".to_string(),
        game_version: "1.20.1".to_string(),
        loader: ModLoaderType::Vanilla,
        loader_version: None,
        copy_mods: true,
        copy_configs: false,
        copy_saves: false,
        copy_resource_packs: false,
        copy_options: false,
        copy_servers: false,
        copy_screenshots: false,
        copy_extra_data: false,
    };

    let res = super::execute::execute_import_instance(instances_dir, options, |_stage, _cfg| {
        Err(crate::minecraft::metadata::MetadataError::Conflict(
            "Injected metadata persistence failure".to_string(),
        ))
    });

    assert!(res.is_err());
    // Verify destination was cleaned up and instances_dir has no leftovers
    let entries = fs::read_dir(instances_dir).unwrap().collect::<Vec<_>>();
    assert_eq!(
        entries.len(),
        0,
        "Failed import must leave no staged/published directory"
    );
}

#[test]
fn test_copy_instance_contents_propagates_extra_data_failure() {
    let temp_src = tempfile::tempdir().expect("src");
    let temp_dst = tempfile::tempdir().expect("dst");
    let src = temp_src.path();
    let dst = temp_dst.path();

    // Create an unhandled extra data folder in src
    fs::create_dir_all(src.join("custom_extra")).unwrap();
    fs::write(src.join("custom_extra").join("data.txt"), b"extra").unwrap();

    // Block destination custom_extra by making it a file where a directory is expected
    fs::write(dst.join("custom_extra"), b"blocking file").unwrap();

    let options = ImportInstanceOptions {
        source_path: src.to_string_lossy().to_string(),
        launcher_id: "custom".to_string(),
        name: "Failing Extra Data Import".to_string(),
        game_version: "1.20.1".to_string(),
        loader: ModLoaderType::Vanilla,
        loader_version: None,
        copy_mods: false,
        copy_configs: false,
        copy_saves: false,
        copy_resource_packs: false,
        copy_options: false,
        copy_servers: false,
        copy_screenshots: false,
        copy_extra_data: true,
    };

    let res = copy_instance_contents(src, dst, &options);
    assert!(res.is_err(), "must propagate extra data copy failure");
}
