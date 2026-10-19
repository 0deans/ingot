use super::*;

#[test]
fn configs_export_skips_worlds_and_jars() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().to_path_buf();
    for dir in [
        "world/region",
        "plugins/LuckPerms",
        "logs",
        "config/ftbquests",
        "mods",
        ".ingot/map",
    ] {
        fs::create_dir_all(root.join(dir)).unwrap();
    }
    for file in [
        "server.properties",
        "server.jar",
        "whitelist.json",
        "world/level.dat",
        "world/region/r.0.0.mca",
        "plugins/LuckPerms.jar",
        "plugins/LuckPerms/config.yml",
        "logs/latest.log",
        "config/paper-global.yml",
        "config/ftbquests/chapter.snbt",
        "mods/create.jar",
        ".ingot/plugins.json",
        ".ingot/map/r.0.0.png",
        "quilt-server-launch.jar",
        "forge-26.1.2-64.1.0-shim.jar",
        "run.bat",
        "user_jvm_args.txt",
    ] {
        fs::write(root.join(file), "x").unwrap();
    }
    let list = |mode| {
        let mut out = Vec::new();
        collect(&root, &root, mode, &mut out).unwrap();
        let mut names: Vec<String> = out
            .iter()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .collect();
        names.sort();
        names
    };
    assert_eq!(
        list(ExportMode::Configs),
        [
            ".ingot/plugins.json",
            "config/ftbquests/chapter.snbt",
            "config/paper-global.yml",
            "plugins/LuckPerms/config.yml",
            "server.properties",
            "whitelist.json"
        ]
    );
    let full = list(ExportMode::Full);
    assert!(full.contains(&"world/region/r.0.0.mca".to_string()));
    assert!(full.contains(&"plugins/LuckPerms.jar".to_string()));
    assert!(full.contains(&"mods/create.jar".to_string()));
    assert!(full.contains(&".ingot/plugins.json".to_string()));
    assert!(!full
        .iter()
        .any(|f| f == "server.jar" || f.starts_with("logs/") || f.starts_with(".ingot/map")));
    assert!(!full.iter().any(|f| f.ends_with("-shim.jar")
        || f == "quilt-server-launch.jar"
        || f == "run.bat"
        || f == "user_jvm_args.txt"));
}

pub(super) fn config() -> ServerConfig {
    serde_json::from_value(serde_json::json!({
        "id": "source", "name": "Source", "core": "paper", "gameVersion": "1.20.1",
        "port": 25565, "memoryMinMb": 1024, "memoryMaxMb": 2048, "createdAt": 0
    }))
    .unwrap()
}

pub(super) fn service(root: &Path) -> TransferService {
    let parent = root.join("servers");
    fs::create_dir_all(&parent).unwrap();
    TransferService {
        repository: std::sync::Arc::new(
            crate::minecraft::metadata::MetadataRepository::with_deletion_directory(
                root.join("servers.json"),
                parent.clone(),
                |server: &ServerConfig| &server.id,
            ),
        ),
        servers_dir: parent,
    }
}

fn archive(path: &Path, format: u32) {
    let mut zip = zip::ZipWriter::new(File::create(path).unwrap());
    zip.start_file(MANIFEST, zip::write::SimpleFileOptions::default())
        .unwrap();
    let manifest = Manifest {
        format,
        name: "Imported".into(),
        core: ServerCoreType::Paper,
        game_version: "1.20.1".into(),
        build_number: None,
        memory_min_mb: 1024,
        memory_max_mb: 2048,
        mode: ExportMode::Full,
    };
    zip.write_all(&serde_json::to_vec(&manifest).unwrap())
        .unwrap();
    zip.start_file(
        "server.properties",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(b"server-port=25565\n").unwrap();
    zip.finish().unwrap();
}

#[test]
fn unsupported_format_is_rejected_before_checkpoint_or_publication() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let path = root.path().join("input.zip");
    archive(&path, 9);
    let result = service.import(&path, |_| panic!("invalid archive must not checkpoint"));
    assert!(matches!(
        result,
        Err(TransferError::UnsupportedManifestFormat(9))
    ));
    assert!(service.repository.load().unwrap().is_empty());
    assert_eq!(fs::read_dir(&service.servers_dir).unwrap().count(), 0);
}

#[test]
fn checkpoint_failure_keeps_archive_and_does_not_publish() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let path = root.path().join("input.zip");
    archive(&path, 1);
    let result = service.import(&path, |_| {
        Err(std::io::Error::other("injected checkpoint failure").into())
    });
    assert!(matches!(result, Err(TransferError::Io(_))));
    assert!(path.is_file());
    assert!(service.repository.load().unwrap().is_empty());
    assert_eq!(fs::read_dir(&service.servers_dir).unwrap().count(), 0);
}

#[test]
fn duplicate_metadata_failure_preserves_original_files() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let source = config();
    let original = service.servers_dir.join(&source.id);
    fs::create_dir_all(&original).unwrap();
    fs::write(original.join("world.dat"), b"original world").unwrap();
    // A directory cannot be atomically replaced by the metadata file.
    fs::create_dir(root.path().join("servers.json")).unwrap();
    assert!(service.duplicate(&source, "Copy", None, None).is_err());
    assert_eq!(
        fs::read(original.join("world.dat")).unwrap(),
        b"original world"
    );
}

#[test]
fn duplicate_preserves_settings_and_uses_an_independent_port() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let mut source = config();
    source.jvm_args = Some(vec!["-Dtest=true".into()]);
    fs::write(
        root.path().join("servers.json"),
        serde_json::to_vec(&[source.clone()]).unwrap(),
    )
    .unwrap();
    let original = service.servers_dir.join(&source.id);
    fs::create_dir_all(&original).unwrap();
    fs::write(original.join("world.dat"), b"original world").unwrap();
    let copied = service.duplicate(&source, "Copy", None, None).unwrap();
    assert_ne!(copied.port, source.port);
    assert_eq!(copied.jvm_args, source.jvm_args);
    assert_eq!(
        fs::read(service.servers_dir.join(copied.id).join("world.dat")).unwrap(),
        b"original world"
    );
    assert_eq!(service.repository.load().unwrap().len(), 2);
}
