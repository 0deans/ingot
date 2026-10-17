use super::duplicate::copy_instance_tree;
use super::*;

fn stored_instance(id: &str) -> InstanceConfig {
    let mut config =
        prepare_instance("Test".into(), "1.20.1".into(), ModLoaderType::Vanilla, None).unwrap();
    config.id = id.into();
    config
}

#[test]
fn invalid_identifiers_cannot_remove_files_outside_instances() {
    use crate::minecraft::metadata::MetadataRepository;
    let root = tempfile::tempdir().unwrap();
    let instances = root.path().join("instances");
    fs::create_dir(&instances).unwrap();
    let outside = root.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("save.dat"), b"keep").unwrap();
    let repository = MetadataRepository::new(root.path().join("instances.json"));
    for id in [
        "../outside",
        "..\\outside",
        ".",
        "",
        "CON",
        "outside:stream",
    ] {
        let result = storage::delete_instance_record(&repository, &instances, id);
        assert!(matches!(result, Err(InstanceError::InvalidId)), "{id}");
    }
    assert_eq!(fs::read(outside.join("save.dat")).unwrap(), b"keep");
    assert!(repository.load().unwrap().is_empty());
}

#[test]
fn required_instance_subdirectory_creation_failure_is_not_success() {
    let root = tempfile::tempdir().unwrap();
    let instance = root.path().join("instance");
    fs::create_dir(&instance).unwrap();
    fs::write(instance.join("config"), b"blocking file").unwrap();
    assert!(matches!(
        ensure_instance_directory(root.path(), "instance"),
        Err(InstanceError::Io(_))
    ));
    assert_eq!(fs::read(instance.join("config")).unwrap(), b"blocking file");
}

#[test]
fn deletion_commits_matching_metadata_and_files_only() {
    use crate::minecraft::metadata::MetadataRepository;
    let root = tempfile::tempdir().unwrap();
    let instances = root.path().join("instances");
    for id in ["delete", "keep"] {
        fs::create_dir_all(instances.join(id)).unwrap();
        fs::write(instances.join(id).join("save.dat"), id).unwrap();
    }
    let repository = MetadataRepository::new(root.path().join("instances.json"));
    repository
        .update(|records| {
            records.extend([stored_instance("delete"), stored_instance("keep")]);
            Ok(())
        })
        .unwrap();
    storage::delete_instance_record(&repository, &instances, "delete").unwrap();
    assert!(!instances.join("delete").exists());
    assert_eq!(fs::read(instances.join("keep/save.dat")).unwrap(), b"keep");
    let records = repository.load().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].id, "keep");
    assert_eq!(fs::read_dir(instances).unwrap().count(), 1);
}

#[test]
fn failed_deletion_metadata_write_restores_original_directory() {
    use crate::minecraft::metadata::{MetadataError, MetadataRepository};
    use std::cell::Cell;
    let root = tempfile::tempdir().unwrap();
    let instances = root.path().join("instances");
    let directory = instances.join("instance");
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("world.dat"), b"original world").unwrap();
    let metadata = root.path().join("instances.json");
    let preserved_metadata = root.path().join("previous.json");
    let repository = MetadataRepository::new(metadata.clone());
    repository
        .update(|records| {
            records.push(stored_instance("instance"));
            Ok(())
        })
        .unwrap();
    let previous = fs::read(&metadata).unwrap();
    let injected = Cell::new(false);
    let result = storage::delete_with_rename(
        &repository,
        &instances,
        "instance",
        |source, destination| {
            fs::rename(source, destination)?;
            if !injected.replace(true) {
                // Block the metadata replacement after its read and directory staging.
                fs::rename(&metadata, &preserved_metadata)?;
                fs::create_dir(&metadata)?;
            }
            Ok(())
        },
    );
    assert!(matches!(
        result,
        Err(InstanceError::Metadata(MetadataError::Io { .. }))
    ));
    assert_eq!(
        fs::read(directory.join("world.dat")).unwrap(),
        b"original world"
    );
    assert_eq!(fs::read_dir(&instances).unwrap().count(), 1);
    assert_eq!(fs::read(&preserved_metadata).unwrap(), previous);
    fs::remove_dir(&metadata).unwrap();
    fs::rename(preserved_metadata, metadata).unwrap();
    assert_eq!(repository.load().unwrap()[0].id, "instance");
}

#[test]
fn failed_duplicate_copy_removes_unique_stage_and_keeps_source_metadata() {
    use crate::minecraft::metadata::MetadataRepository;
    let root = tempfile::tempdir().unwrap();
    let instances = root.path().join("instances");
    fs::create_dir(&instances).unwrap();
    let metadata = root.path().join("instances.json");
    let repository = MetadataRepository::new(metadata.clone());
    repository
        .update(|records| {
            records.push(stored_instance("missing-directory"));
            Ok(())
        })
        .unwrap();
    let previous = fs::read(&metadata).unwrap();
    let result = duplicate::duplicate_instance_record(
        &repository,
        &instances,
        "missing-directory",
        "Copy".into(),
        true,
    );
    assert!(matches!(result, Err(InstanceError::Io(_))));
    assert_eq!(fs::read_dir(instances).unwrap().count(), 0);
    assert_eq!(fs::read(metadata).unwrap(), previous);
}

#[test]
fn copy_leaves_history_and_optionally_worlds_behind() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let src = root.join("src");
    for dir in [
        "mods",
        "config/sodium",
        "saves/World/region",
        "logs",
        "crash-reports",
        "screenshots",
        ".ingot/version-change",
    ] {
        fs::create_dir_all(src.join(dir)).unwrap();
    }
    for file in [
        "options.txt",
        "mods/sodium.jar",
        "config/sodium/options.json",
        "saves/World/level.dat",
        "saves/World/session.lock",
        "saves/World/region/r.0.0.mca",
        "logs/latest.log",
        "crash-reports/crash.txt",
        "screenshots/a.png",
        ".ingot/version-change/journal.json",
    ] {
        fs::write(src.join(file), file).unwrap();
    }

    let with = root.join("with");
    copy_instance_tree(&src, &with, true, true).unwrap();
    for kept in [
        "options.txt",
        "mods/sodium.jar",
        "config/sodium/options.json",
        "saves/World/level.dat",
        "saves/World/region/r.0.0.mca",
    ] {
        assert!(with.join(kept).is_file(), "{kept} should be copied");
    }
    for left in [
        "saves/World/session.lock",
        "logs",
        "crash-reports",
        "screenshots",
        ".ingot",
    ] {
        assert!(!with.join(left).exists(), "{left} should stay behind");
    }

    let without = root.join("without");
    copy_instance_tree(&src, &without, true, false).unwrap();
    assert!(without.join("mods/sodium.jar").is_file());
    assert!(without.join("saves").is_dir());
    assert_eq!(fs::read_dir(without.join("saves")).unwrap().count(), 0);

    let _ = fs::remove_dir_all(root);
}
