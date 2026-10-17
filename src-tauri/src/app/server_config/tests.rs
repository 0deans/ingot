use super::*;
use crate::minecraft::metadata::{MetadataError, MetadataRepository};

#[test]
fn directory_lookup_restores_pending_deletion_before_creating_a_directory() {
    let root = tempfile::tempdir().unwrap();
    let parent = root.path().join("servers");
    let original = parent.join("server");
    fs::create_dir_all(&original).unwrap();
    fs::write(original.join("world.dat"), b"retained world").unwrap();
    let config: ServerConfig = serde_json::from_value(serde_json::json!({
        "id": "server", "name": "Server", "core": "paper", "gameVersion": "1.20.1",
        "port": 25565, "memoryMinMb": 1024, "memoryMaxMb": 2048, "createdAt": 0
    }))
    .unwrap();
    let metadata = root.path().join("servers.json");
    fs::write(&metadata, serde_json::to_vec(&[config]).unwrap()).unwrap();
    let backup = format!(".deleted-{}", Uuid::new_v4());
    let receipt = metadata.with_extension("deletion.json");
    fs::write(
        &receipt,
        serde_json::to_vec(&serde_json::json!({
            "version": 1, "id": "server", "backup": backup,
        }))
        .unwrap(),
    )
    .unwrap();
    fs::rename(&original, parent.join(&backup)).unwrap();
    let repository = MetadataRepository::with_deletion_directory(
        metadata,
        parent.clone(),
        |server: &ServerConfig| &server.id,
    );

    assert_eq!(
        ensure_server_directory(&repository, &parent, "server").unwrap(),
        original
    );
    assert_eq!(
        fs::read(original.join("world.dat")).unwrap(),
        b"retained world"
    );
    assert!(!parent.join(backup).exists());
    assert!(!receipt.exists());
}

#[test]
fn directory_lookup_cannot_create_files_while_recovery_is_blocked() {
    let root = tempfile::tempdir().unwrap();
    let parent = root.path().join("servers");
    let metadata = root.path().join("servers.json");
    let receipt = metadata.with_extension("deletion.json");
    fs::write(&receipt, b"malformed receipt").unwrap();
    let repository = MetadataRepository::with_deletion_directory(
        metadata,
        parent.clone(),
        |server: &ServerConfig| &server.id,
    );

    let result = ensure_server_directory(&repository, &parent, "server");
    assert!(matches!(
        result,
        Err(ConfigError::Metadata(MetadataError::Decode { .. }))
    ));
    assert!(!parent.exists());
    assert_eq!(fs::read(receipt).unwrap(), b"malformed receipt");
}
