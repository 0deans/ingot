use super::pack::prepare_modpack;
use std::io::{Cursor, Write};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn explicit_override_directories_are_extracted_before_publication() {
    let root = tempfile::tempdir().unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(
            "modrinth.index.json",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    writer
        .write_all(br#"{"dependencies":{"minecraft":"1.20.1"},"files":[]}"#)
        .unwrap();
    writer
        .add_directory("overrides/", zip::write::SimpleFileOptions::default())
        .unwrap();
    writer
        .add_directory(
            "overrides/config/",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    writer
        .add_directory(
            "overrides/config/nested/",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    writer
        .start_file(
            "overrides/config/nested/options.txt",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    writer.write_all(b"settings").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    let prepared = prepare(root.path(), bytes, "unused", "modrinth")
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(prepared.stage.path().join("config/nested/options.txt")).unwrap(),
        b"settings"
    );
}

#[test]
fn modpack_publication_preserves_metadata_error_type_and_leaves_working_instance() {
    use crate::minecraft::{
        instance::{prepare_instance, ModLoaderType},
        metadata::{MetadataError, MetadataRepository},
    };
    let root = tempfile::tempdir().unwrap();
    let metadata = root.path().join("instances.json");
    std::fs::write(&metadata, b"corrupt metadata").unwrap();
    let existing = root.path().join("existing");
    std::fs::create_dir(&existing).unwrap();
    std::fs::write(existing.join("options.txt"), b"keep").unwrap();
    let repository = MetadataRepository::new(metadata.clone());
    let stage = tempfile::tempdir_in(root.path()).unwrap();
    std::fs::write(stage.path().join("options.txt"), b"new").unwrap();
    let config = prepare_instance(
        "Imported".into(),
        "1.20.1".into(),
        ModLoaderType::Vanilla,
        None,
    )
    .unwrap();
    let destination = root.path().join(&config.id);
    let result = super::publish_modpack(&repository, root.path(), config, stage.path());
    assert!(matches!(
        result,
        Err(crate::minecraft::content::ContentError::Metadata(
            MetadataError::Decode { .. }
        ))
    ));
    assert!(!destination.exists());
    assert_eq!(std::fs::read(metadata).unwrap(), b"corrupt metadata");
    assert_eq!(
        std::fs::read(existing.join("options.txt")).unwrap(),
        b"keep"
    );
    assert_eq!(
        std::fs::read(stage.path().join("options.txt")).unwrap(),
        b"new"
    );
}

#[test]
fn individual_content_replacement_is_atomic_and_cleans_failed_staging() {
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("mod.jar");
    std::fs::write(&destination, b"old").unwrap();
    super::publish_content_file(&destination, b"new").unwrap();
    assert_eq!(std::fs::read(&destination).unwrap(), b"new");
    let invalid = root.path().join("directory.jar");
    std::fs::create_dir(&invalid).unwrap();
    std::fs::write(invalid.join("existing"), b"keep").unwrap();
    assert!(super::publish_content_file(&invalid, b"replace").is_err());
    assert_eq!(std::fs::read(invalid.join("existing")).unwrap(), b"keep");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 2);
}

#[tokio::test]
async fn cancelled_download_removes_unpublished_stage() {
    let root = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0; 4096];
        assert!(socket.read(&mut request).await.unwrap() > 0);
        started_tx.send(()).unwrap();
        std::future::pending::<()>().await;
    });
    let bytes = archive(
        "modrinth.index.json",
        serde_json::json!({"dependencies":{"minecraft":"1.20.1"},"files":[{"path":"mods/a.jar","downloads":[url]}]}),
        None,
    );
    let mut future = Box::pin(prepare(root.path(), bytes, "unused", "modrinth"));
    tokio::select! {
        result=&mut future => panic!("download completed unexpectedly: {}",result.is_ok()),
        result=tokio::time::timeout(std::time::Duration::from_secs(5),started_rx) => {result.unwrap().unwrap();}
    }
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    drop(future);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    task.abort();
    let _ = task.await;
}
use zip::{write::SimpleFileOptions, ZipWriter};

struct Server {
    url: String,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn server(status: u16, body: String) -> Server {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            assert!(socket.read(&mut request).await.unwrap() > 0);
            let response = format!("HTTP/1.1 {status} Response\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            socket.write_all(response.as_bytes()).await.unwrap();
        }
    });
    Server { url, task }
}

async fn prepare(
    root: &std::path::Path,
    bytes: Vec<u8>,
    endpoint: &str,
    source: &str,
) -> Result<super::pack::PreparedModpack, crate::minecraft::content::ContentError> {
    prepare_modpack(
        &reqwest::Client::new(),
        endpoint,
        root,
        bytes,
        source,
        "pack.zip",
        "Pack",
    )
    .await
}

#[tokio::test]
async fn modrinth_http_failure_keeps_instance_unpublished() {
    let root = tempfile::tempdir().unwrap();
    let server = server(500, "failed".into()).await;
    let bytes = archive(
        "modrinth.index.json",
        serde_json::json!({"dependencies":{"minecraft":"1.20.1"},"files":[{"path":"mods/a.jar","downloads":[server.url]}]}),
        None,
    );
    assert!(matches!(
        prepare(root.path(), bytes, "unused", "modrinth").await,
        Err(crate::minecraft::content::ContentError::HttpStatus(_))
    ));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn modrinth_validates_size_and_hash_before_publishing() {
    let root = tempfile::tempdir().unwrap();
    let server = server(200, "abc".into()).await;
    for (size, hash, valid) in [
        (3, "a9993e364706816aba3e25717850c26c9cd0d89d", true),
        (4, "a9993e364706816aba3e25717850c26c9cd0d89d", false),
        (3, "invalid", false),
    ] {
        let bytes = archive(
            "modrinth.index.json",
            serde_json::json!({"dependencies":{"minecraft":"1.20.1"},"files":[{"path":"mods/a.jar","downloads":[server.url],"fileSize":size,"hashes":{"sha1":hash}}]}),
            None,
        );
        let result = prepare(root.path(), bytes, "unused", "modrinth").await;
        if valid {
            let prepared = result.unwrap();
            assert_eq!(
                std::fs::read(prepared.stage.path().join("mods/a.jar")).unwrap(),
                b"abc"
            );
            assert_eq!(prepared.game_version, "1.20.1");
            assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
            drop(prepared);
        } else {
            assert!(matches!(
                result,
                Err(crate::minecraft::content::ContentError::Integrity(_))
            ));
        }
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
}

#[tokio::test]
async fn curseforge_lookup_failure_cleans_staging() {
    let root = tempfile::tempdir().unwrap();
    let server = server(500, "lookup failed".into()).await;
    let bytes = archive(
        "manifest.json",
        serde_json::json!({"minecraft":{"version":"1.20.1"},"files":[{"projectID":1,"fileID":2}]}),
        None,
    );
    assert!(matches!(
        prepare(root.path(), bytes, &server.url, "curseforge").await,
        Err(crate::minecraft::content::ContentError::HttpStatus(_))
    ));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn curseforge_download_integrity_and_http_failures_are_propagated() {
    let root = tempfile::tempdir().unwrap();
    for (status, size, valid) in [(200, 3, true), (200, 4, false), (500, 3, false)] {
        let download = server(status, "abc".into()).await;
        let api=server(200,serde_json::json!({"data":{"id":2,"fileName":"a.jar","downloadUrl":download.url,"fileLength":size,"hashes":[{"algo":1,"value":"a9993e364706816aba3e25717850c26c9cd0d89d"}]}}).to_string()).await;
        let bytes = archive(
            "manifest.json",
            serde_json::json!({"minecraft":{"version":"1.20.1"},"files":[{"projectID":1,"fileID":2}]}),
            None,
        );
        let result = prepare(root.path(), bytes, &api.url, "curseforge").await;
        if valid {
            assert_eq!(
                std::fs::read(result.unwrap().stage.path().join("mods/a.jar")).unwrap(),
                b"abc"
            );
        } else {
            assert!(result.is_err());
        }
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
}

#[tokio::test]
async fn override_write_failure_cleans_staging() {
    let root = tempfile::tempdir().unwrap();
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, body) in [
        (
            "modrinth.index.json",
            r#"{"dependencies":{"minecraft":"1.20.1"},"files":[]}"#,
        ),
        ("overrides/config", "file blocks directory"),
        ("overrides/config/options.txt", "cannot write"),
    ] {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(body.as_bytes()).unwrap();
    }
    let bytes = writer.finish().unwrap().into_inner();
    assert!(prepare(root.path(), bytes, "unused", "modrinth")
        .await
        .is_err());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

fn archive(
    manifest_name: &str,
    manifest: serde_json::Value,
    extra: Option<(&str, &[u8])>,
) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(manifest_name, SimpleFileOptions::default())
        .unwrap();
    writer.write_all(manifest.to_string().as_bytes()).unwrap();
    if let Some((name, contents)) = extra {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(contents).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[tokio::test]
async fn invalid_override_does_not_publish_or_leave_staging() {
    let root = tempfile::tempdir().unwrap();
    let bytes = archive(
        "modrinth.index.json",
        serde_json::json!({
            "dependencies": {"minecraft": "1.20.1"}, "files": []
        }),
        Some(("overrides/../escape", b"bad")),
    );
    let result = prepare_modpack(
        &reqwest::Client::new(),
        "http://127.0.0.1:1",
        root.path(),
        bytes,
        "modrinth",
        "pack.mrpack",
        "Pack",
    )
    .await;
    assert!(result.is_err());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn missing_required_download_does_not_publish_or_leave_staging() {
    let root = tempfile::tempdir().unwrap();
    let bytes = archive(
        "modrinth.index.json",
        serde_json::json!({
            "dependencies": {"minecraft": "1.20.1"},
            "files": [{"path": "mods/a.jar", "downloads": []}]
        }),
        None,
    );
    let result = prepare_modpack(
        &reqwest::Client::new(),
        "http://127.0.0.1:1",
        root.path(),
        bytes,
        "modrinth",
        "pack.mrpack",
        "Pack",
    )
    .await;
    assert!(result.is_err());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
