use super::pack::{prepare_modpack, PrepareModpackParams};
use ingot_test_support::{LoopbackServer, ResponseSpec, ScriptedExchange};
use std::io::{Cursor, Write};

#[path = "pack_third_party_tests.rs"]
mod third_party_tests;

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
        Err(crate::content::ContentError::Metadata(
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
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let started_tx = std::sync::Arc::new(std::sync::Mutex::new(Some(started_tx)));
    let server = LoopbackServer::with_handler(move |_| {
        let tx = std::sync::Arc::clone(&started_tx);
        ResponseSpec::stall().with_effect(move |_| {
            if let Some(sender) = tx.lock().unwrap().take() {
                let _ = sender.send(());
            }
        })
    })
    .await;
    let url = server.base_url().to_string();
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
}
use zip::{write::SimpleFileOptions, ZipWriter};

struct Server {
    url: String,
    _server: LoopbackServer,
}

async fn server(status: u16, body: String) -> Server {
    let body = body.into_bytes();
    let server =
        LoopbackServer::with_handler(move |_| ResponseSpec::bytes(status, body.clone())).await;
    Server {
        url: server.base_url().to_string(),
        _server: server,
    }
}

async fn prepare(
    root: &std::path::Path,
    bytes: Vec<u8>,
    endpoint: &str,
    source: &str,
) -> Result<super::pack::PreparedModpack, crate::content::ContentError> {
    prepare_modpack(PrepareModpackParams {
        client: &reqwest::Client::new(),
        curseforge_base: endpoint,
        root,
        bytes,
        source,
        filename: "pack.zip",
        name: "Pack",
        manifest_url: None,
        on_progress: None,
    })
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
        Err(crate::content::ContentError::HttpStatus(_))
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
                Err(crate::content::ContentError::Integrity(_))
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
        Err(crate::content::ContentError::HttpStatus(_))
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
    let result = prepare_modpack(PrepareModpackParams {
        client: &reqwest::Client::new(),
        curseforge_base: "http://127.0.0.1:1",
        root: root.path(),
        bytes,
        source: "modrinth",
        filename: "pack.mrpack",
        name: "Pack",
        manifest_url: None,
        on_progress: None,
    })
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
    let result = prepare_modpack(PrepareModpackParams {
        client: &reqwest::Client::new(),
        curseforge_base: "http://127.0.0.1:1",
        root: root.path(),
        bytes,
        source: "modrinth",
        filename: "pack.mrpack",
        name: "Pack",
        manifest_url: None,
        on_progress: None,
    })
    .await;
    assert!(result.is_err());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

async fn binary_server(body: Vec<u8>, status: u16) -> Server {
    let server = LoopbackServer::exact_script(vec![ScriptedExchange::any_path(
        ResponseSpec::bytes(status, body),
    )])
    .await;
    Server {
        url: server.base_url().to_string(),
        _server: server,
    }
}

#[tokio::test]
async fn modpack_service_publishes_files_and_metadata_without_tauri() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("instances");
    std::fs::create_dir(&root).unwrap();
    let repository = std::sync::Arc::new(
        crate::minecraft::metadata::MetadataRepository::with_deletion_directory(
            directory.path().join("instances.json"),
            root.clone(),
            |record: &crate::minecraft::instance::InstanceConfig| &record.id,
        ),
    );
    let bytes = archive(
        "modrinth.index.json",
        serde_json::json!({"dependencies":{"minecraft":"1.20.1"},"files":[]}),
        Some(("overrides/options.txt", b"prepared options")),
    );
    let server = binary_server(bytes, 200).await;
    let service = super::ModpackInstaller {
        client: reqwest::Client::new(),
        instances: std::sync::Arc::clone(&repository),
        root: root.clone(),
        curseforge_base: server.url.clone(),
        on_progress: None,
    };
    let config = service
        .install("Pack", "modrinth", &server.url, "pack.mrpack")
        .await
        .unwrap();
    assert_eq!(repository.load().unwrap().len(), 1);
    assert_eq!(repository.load().unwrap()[0].id, config.id);
    assert_eq!(
        std::fs::read(root.join(&config.id).join("options.txt")).unwrap(),
        b"prepared options"
    );
    assert_eq!(std::fs::read_dir(root).unwrap().count(), 1);
}

#[tokio::test]
async fn content_file_service_keeps_previous_file_on_http_failure() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("mods")).unwrap();
    std::fs::write(root.path().join("mods/a.jar"), b"old").unwrap();
    let server = binary_server(b"unavailable".to_vec(), 503).await;
    let result = super::install_content_file(
        &reqwest::Client::new(),
        root.path(),
        "mod",
        &server.url,
        "a.jar",
    )
    .await;
    assert!(matches!(
        result,
        Err(crate::content::ContentError::HttpStatus(_))
    ));
    assert_eq!(
        std::fs::read(root.path().join("mods/a.jar")).unwrap(),
        b"old"
    );
}

#[tokio::test]
async fn content_file_service_rejects_nested_filenames_before_download() {
    let root = tempfile::tempdir().unwrap();
    let result = super::install_content_file(
        &reqwest::Client::new(),
        root.path(),
        "mod",
        "invalid-url",
        "../a.jar",
    )
    .await;
    assert!(matches!(
        result,
        Err(crate::content::ContentError::InvalidFilename(_))
    ));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn modpack_preparation_reports_progress_events() {
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
    let bytes = writer.finish().unwrap().into_inner();

    let reports = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let reports_clone = std::sync::Arc::clone(&reports);
    let reporter: crate::minecraft::launcher::progress::ProgressCallback =
        std::sync::Arc::new(move |phase: &str, current: u64, total: u64, detail: &str| {
            reports_clone.lock().unwrap().push((
                phase.to_string(),
                current,
                total,
                detail.to_string(),
            ));
        });

    let prepared = prepare_modpack(PrepareModpackParams {
        client: &reqwest::Client::new(),
        curseforge_base: "unused",
        root: root.path(),
        bytes,
        source: "modrinth",
        filename: "pack.mrpack",
        name: "Pack",
        manifest_url: None,
        on_progress: Some(&reporter),
    })
    .await
    .unwrap();

    assert_eq!(prepared.name, "Pack");
    let recorded = reports.lock().unwrap();
    assert!(!recorded.is_empty());
    assert_eq!(recorded.last().unwrap().0, "Installing Modpack");
    assert_eq!(
        recorded.last().unwrap().3,
        "Finalizing modpack installation..."
    );
}
