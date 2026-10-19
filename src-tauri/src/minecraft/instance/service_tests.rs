use super::*;
use crate::minecraft::metadata::MetadataRepository;

fn repository(root: &Path) -> MetadataRepository<InstanceConfig> {
    MetadataRepository::with_deletion_directory(
        root.join("instances.json"),
        root.join("instances"),
        |record: &InstanceConfig| &record.id,
    )
}

fn create(repository: &MetadataRepository<InstanceConfig>, parent: &Path) -> InstanceConfig {
    create_instance(
        repository,
        parent,
        "Original".into(),
        "1.21".into(),
        ModLoaderType::Fabric,
        Some("0.16".into()),
    )
    .unwrap()
}

#[test]
fn creation_publishes_matching_metadata_and_required_directories() {
    let root = tempfile::tempdir().unwrap();
    let repository = repository(root.path());
    let parent = root.path().join("instances");
    let instance = create(&repository, &parent);

    let records = load_instances(&repository).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].id, instance.id);
    assert_eq!(records[0].loader, ModLoaderType::Fabric);
    assert_eq!(records[0].loader_version.as_deref(), Some("0.16"));
    for name in ["mods", "config", "saves", "resourcepacks"] {
        assert!(parent.join(&instance.id).join(name).is_dir());
    }
    assert_eq!(fs::read_dir(parent).unwrap().count(), 1);
}

#[test]
fn invalid_creation_does_not_create_or_publish_anything() {
    let root = tempfile::tempdir().unwrap();
    let repository = repository(root.path());
    let parent = root.path().join("instances");
    let result = create_instance(
        &repository,
        &parent,
        "  ".into(),
        "1.21".into(),
        ModLoaderType::Vanilla,
        None,
    );

    assert!(matches!(result, Err(InstanceError::EmptyName)));
    assert!(!parent.exists());
    assert!(!root.path().join("instances.json").exists());
}

#[test]
fn corrupt_metadata_blocks_creation_before_staging() {
    let root = tempfile::tempdir().unwrap();
    let repository = repository(root.path());
    let metadata = root.path().join("instances.json");
    fs::write(&metadata, b"corrupt metadata").unwrap();
    let parent = root.path().join("instances");
    let result = create_instance(
        &repository,
        &parent,
        "Test".into(),
        "1.21".into(),
        ModLoaderType::Vanilla,
        None,
    );

    assert!(matches!(result, Err(InstanceError::Metadata(_))));
    assert_eq!(fs::read(metadata).unwrap(), b"corrupt metadata");
    assert!(!parent.exists());
}

#[test]
fn directory_failure_keeps_original_io_error_and_existing_data() {
    let root = tempfile::tempdir().unwrap();
    let repository = repository(root.path());
    let parent = root.path().join("instances");
    fs::write(&parent, b"keep existing file").unwrap();
    let result = create_instance(
        &repository,
        &parent,
        "Test".into(),
        "1.21".into(),
        ModLoaderType::Vanilla,
        None,
    );

    let Err(InstanceError::Io(source)) = result else {
        panic!("expected filesystem failure")
    };
    assert!(source.raw_os_error().is_some());
    assert_eq!(fs::read(parent).unwrap(), b"keep existing file");
    assert!(load_instances(&repository).unwrap().is_empty());
}

#[test]
fn patches_and_playtime_preserve_fresh_metadata_and_saturate() {
    let root = tempfile::tempdir().unwrap();
    let repository = repository(root.path());
    let parent = root.path().join("instances");
    let instance = create(&repository, &parent);
    patch_instance(&repository, &instance.id, |record| {
        record.name = "Renamed".into();
    })
    .unwrap();
    patch_instance(&repository, &instance.id, |record| {
        record.total_play_time_seconds = u64::MAX - 1;
    })
    .unwrap();
    update_last_played(&repository, &instance.id, 30).unwrap();

    let records = load_instances(&repository).unwrap();
    assert_eq!(records[0].name, "Renamed");
    assert_eq!(records[0].total_play_time_seconds, u64::MAX);
    assert!(records[0].last_played.is_some());
    let before = fs::read(root.path().join("instances.json")).unwrap();
    assert!(matches!(
        update_last_played(&repository, "missing", 10),
        Err(InstanceError::Metadata(_))
    ));
    assert_eq!(
        fs::read(root.path().join("instances.json")).unwrap(),
        before
    );
}

#[test]
fn duplication_publishes_independent_files_and_resets_history() {
    let root = tempfile::tempdir().unwrap();
    let repository = repository(root.path());
    let parent = root.path().join("instances");
    let original = create(&repository, &parent);
    fs::write(parent.join(&original.id).join("mods/example.jar"), b"mod").unwrap();
    fs::write(parent.join(&original.id).join("saves/world.dat"), b"world").unwrap();
    update_last_played(&repository, &original.id, 30).unwrap();

    let copy =
        duplicate_instance(&repository, &parent, &original.id, "Copy".into(), false).unwrap();
    assert_ne!(copy.id, original.id);
    assert_eq!(copy.total_play_time_seconds, 0);
    assert!(copy.last_played.is_none());
    assert_eq!(
        fs::read(parent.join(&copy.id).join("mods/example.jar")).unwrap(),
        b"mod"
    );
    assert!(!parent.join(&copy.id).join("saves/world.dat").exists());
    assert_eq!(
        fs::read(parent.join(&original.id).join("saves/world.dat")).unwrap(),
        b"world"
    );
    assert_eq!(load_instances(&repository).unwrap().len(), 2);
}
