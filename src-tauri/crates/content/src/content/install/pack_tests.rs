use super::curseforge_resolver::resolve_curseforge_file;
use super::pack::{prepare_modpack, prepare_modpack_with_cdn, PrepareModpackParams};
use super::support::fetch_with_progress;
use ingot_test_support::{LoopbackServer, ResponseSpec, ScriptedExchange};
use sha1::Digest;
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
    use ingot_core::metadata::{MetadataError, MetadataRepository};
    use ingot_minecraft::instance::{prepare_instance, ModLoaderType};
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
        ingot_core::metadata::MetadataRepository::with_deletion_directory(
            directory.path().join("instances.json"),
            root.clone(),
            |record: &ingot_minecraft::instance::InstanceConfig| &record.id,
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
    let reporter: ingot_minecraft::launcher::progress::ProgressCallback =
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

fn hex_str(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[tokio::test]
async fn curseforge_resolver_constructs_cdn_fallback_when_download_url_missing_or_empty() {
    let payload = b"cdn-payload";
    let sha1_hex = hex_str(sha1::Sha1::digest(payload));
    let md5_hex = hex_str(md5::Md5::digest(payload));
    let api = LoopbackServer::exact_script(vec![
        ScriptedExchange::expect_path(
            "/mods/10/files/4013966",
            ResponseSpec::json(
                200,
                serde_json::json!({
                    "data": {
                        "id": 4013966,
                        "fileName": "jei-1.20.1.jar",
                        "downloadUrl": null,
                        "fileLength": payload.len(),
                        "hashes": [
                            {"algo": 1, "value": sha1_hex},
                            {"algo": 2, "value": md5_hex}
                        ]
                    }
                })
                .to_string(),
            ),
        ),
        ScriptedExchange::expect_path(
            "/mods/10/files/123",
            ResponseSpec::json(
                200,
                serde_json::json!({
                    "data": {
                        "id": 123,
                        "fileName": "small.jar",
                        "downloadUrl": "",
                        "fileLength": 4,
                        "hashes": []
                    }
                })
                .to_string(),
            ),
        ),
    ])
    .await;

    let client = reqwest::Client::new();
    let large = resolve_curseforge_file(&client, api.base_url(), 10, 4013966)
        .await
        .unwrap();
    assert_eq!(large.file_name, "jei-1.20.1.jar");
    assert_eq!(
        large.download_url,
        "https://edge.forgecdn.net/files/4013/966/jei-1.20.1.jar"
    );
    assert_eq!(large.integrity.size, Some(payload.len() as u64));
    assert_eq!(
        large.integrity.hashes.get("sha1").map(String::as_str),
        Some(sha1_hex.as_str())
    );
    assert_eq!(
        large.integrity.hashes.get("md5").map(String::as_str),
        Some(md5_hex.as_str())
    );

    let small = resolve_curseforge_file(&client, api.base_url(), 10, 123)
        .await
        .unwrap();
    assert_eq!(small.file_name, "small.jar");
    assert_eq!(
        small.download_url,
        "https://edge.forgecdn.net/files/0/123/small.jar"
    );

    let requests = api.finish().await;
    assert_eq!(requests.len(), 2);
    assert!(requests[0].header("x-api-key").is_some());
    assert!(requests[1].header("x-api-key").is_some());
}

#[tokio::test]
async fn curseforge_prepare_uses_cdn_fallback_when_download_url_missing() {
    let root = tempfile::tempdir().unwrap();
    let payload = b"curseforge-cdn-jar";
    let sha1_hex = hex_str(sha1::Sha1::digest(payload));
    let cdn = LoopbackServer::exact_script(vec![ScriptedExchange::expect_path(
        "/4013/966/fallback.jar",
        ResponseSpec::bytes(200, payload.to_vec()),
    )])
    .await;
    let api = LoopbackServer::exact_script(vec![ScriptedExchange::expect_path(
        "/mods/1/files/4013966",
        ResponseSpec::json(
            200,
            serde_json::json!({
                "data": {
                    "id": 4013966,
                    "fileName": "fallback.jar",
                    "downloadUrl": null,
                    "fileLength": payload.len(),
                    "hashes": [{"algo": 1, "value": sha1_hex}]
                }
            })
            .to_string(),
        ),
    )])
    .await;
    let bytes = archive(
        "manifest.json",
        serde_json::json!({"minecraft":{"version":"1.20.1"},"files":[{"projectID":1,"fileID":4013966}]}),
        None,
    );
    let prepared = prepare_modpack_with_cdn(
        PrepareModpackParams {
            client: &reqwest::Client::new(),
            curseforge_base: api.base_url(),
            root: root.path(),
            bytes,
            source: "curseforge",
            filename: "pack.zip",
            name: "Pack",
            manifest_url: None,
            on_progress: None,
        },
        cdn.base_url(),
    )
    .await
    .unwrap();
    assert_eq!(
        std::fs::read(prepared.stage.path().join("mods/fallback.jar")).unwrap(),
        payload
    );
    drop(prepared);
    let _ = cdn.finish().await;
    let _ = api.finish().await;
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn curseforge_validates_md5_and_rejects_unsupported_hash_algorithms() {
    let root = tempfile::tempdir().unwrap();
    let payload = b"md5-checked-mod";
    let valid_md5 = hex_str(md5::Md5::digest(payload));

    for (expected_md5, should_succeed) in [
        (valid_md5.as_str(), true),
        ("00000000000000000000000000000000", false),
    ] {
        let download = server(200, String::from_utf8_lossy(payload).into_owned()).await;
        let api = server(
            200,
            serde_json::json!({
                "data": {
                    "id": 2,
                    "fileName": "md5-mod.jar",
                    "downloadUrl": download.url,
                    "fileLength": payload.len(),
                    "hashes": [{"algo": 2, "value": expected_md5}]
                }
            })
            .to_string(),
        )
        .await;
        let bytes = archive(
            "manifest.json",
            serde_json::json!({"minecraft":{"version":"1.20.1"},"files":[{"projectID":1,"fileID":2}]}),
            None,
        );
        let result = prepare(root.path(), bytes, &api.url, "curseforge").await;
        if should_succeed {
            let prepared = result.unwrap();
            assert_eq!(
                std::fs::read(prepared.stage.path().join("mods/md5-mod.jar")).unwrap(),
                payload
            );
            drop(prepared);
        } else {
            assert!(matches!(
                result,
                Err(crate::content::ContentError::Integrity(_))
            ));
        }
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }

    let never_downloaded = LoopbackServer::exact_script(vec![]).await;
    let api = LoopbackServer::exact_script(vec![ScriptedExchange::expect_path(
        "/mods/1/files/2",
        ResponseSpec::json(
            200,
            serde_json::json!({
                "data": {
                    "id": 2,
                    "fileName": "bad-algo.jar",
                    "downloadUrl": never_downloaded.base_url(),
                    "fileLength": 3,
                    "hashes": [{"algo": 99, "value": "deadbeef"}]
                }
            })
            .to_string(),
        ),
    )])
    .await;
    let bytes = archive(
        "manifest.json",
        serde_json::json!({"minecraft":{"version":"1.20.1"},"files":[{"projectID":1,"fileID":2}]}),
        None,
    );
    let unsupported_cf = prepare(root.path(), bytes, api.base_url(), "curseforge").await;
    assert!(matches!(
        unsupported_cf,
        Err(crate::content::ContentError::Integrity(_))
    ));
    let _ = never_downloaded.finish().await;
    let _ = api.finish().await;
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);

    let mr_download = server(200, "abc".into()).await;
    let mr_bytes = archive(
        "modrinth.index.json",
        serde_json::json!({
            "dependencies":{"minecraft":"1.20.1"},
            "files":[{"path":"mods/a.jar","downloads":[mr_download.url],"hashes":{"sha384":"deadbeef"}}]
        }),
        None,
    );
    let unsupported_mr = prepare(root.path(), mr_bytes, "unused", "modrinth").await;
    assert!(matches!(
        unsupported_mr,
        Err(crate::content::ContentError::Integrity(_))
    ));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn modpack_install_publication_and_preparation_failures_clean_staging_and_preserve_existing()
{
    use ingot_core::metadata::{MetadataError, MetadataRepository};

    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("instances");
    std::fs::create_dir(&root).unwrap();
    let existing = root.join("existing");
    std::fs::create_dir(&existing).unwrap();
    std::fs::write(existing.join("options.txt"), b"keep").unwrap();
    let metadata = directory.path().join("instances.json");
    std::fs::write(&metadata, b"corrupt metadata").unwrap();
    let repository = std::sync::Arc::new(MetadataRepository::with_deletion_directory(
        metadata.clone(),
        root.clone(),
        |record: &ingot_minecraft::instance::InstanceConfig| &record.id,
    ));

    let pack_bytes = archive(
        "modrinth.index.json",
        serde_json::json!({"dependencies":{"minecraft":"1.20.1"},"files":[]}),
        Some(("overrides/options.txt", b"new options")),
    );
    let archive_server = binary_server(pack_bytes, 200).await;
    let installer = super::ModpackInstaller {
        client: reqwest::Client::new(),
        instances: std::sync::Arc::clone(&repository),
        root: root.clone(),
        curseforge_base: "http://127.0.0.1:1".into(),
        on_progress: None,
    };
    let pub_err = installer
        .install("NewPack", "modrinth", &archive_server.url, "pack.mrpack")
        .await;
    assert!(matches!(
        pub_err,
        Err(crate::content::ContentError::Metadata(
            MetadataError::Decode { .. }
        ))
    ));
    assert_eq!(std::fs::read(&metadata).unwrap(), b"corrupt metadata");
    assert_eq!(
        std::fs::read(existing.join("options.txt")).unwrap(),
        b"keep"
    );
    let entries: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(entries, vec!["existing".to_string()]);

    let valid_metadata = directory.path().join("valid-instances.json");
    let valid_repo = std::sync::Arc::new(MetadataRepository::with_deletion_directory(
        valid_metadata.clone(),
        root.clone(),
        |record: &ingot_minecraft::instance::InstanceConfig| &record.id,
    ));
    let failing_mod_server = server(500, "mod failed".into()).await;
    let failing_pack_bytes = archive(
        "modrinth.index.json",
        serde_json::json!({
            "dependencies":{"minecraft":"1.20.1"},
            "files":[{"path":"mods/a.jar","downloads":[failing_mod_server.url]}]
        }),
        None,
    );
    let failing_archive_server = binary_server(failing_pack_bytes, 200).await;
    let valid_installer = super::ModpackInstaller {
        client: reqwest::Client::new(),
        instances: std::sync::Arc::clone(&valid_repo),
        root: root.clone(),
        curseforge_base: "http://127.0.0.1:1".into(),
        on_progress: None,
    };
    let prep_err = valid_installer
        .install(
            "FailingPack",
            "modrinth",
            &failing_archive_server.url,
            "pack.mrpack",
        )
        .await;
    assert!(matches!(
        prep_err,
        Err(crate::content::ContentError::HttpStatus(_))
    ));
    assert!(valid_repo.load().unwrap().is_empty());
    let entries: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(entries, vec!["existing".to_string()]);

    let conflict_root = tempfile::tempdir().unwrap();
    let conflict_meta = conflict_root.path().join("instances.json");
    let conflict_repo = MetadataRepository::with_deletion_directory(
        conflict_meta,
        conflict_root.path().to_path_buf(),
        |record: &ingot_minecraft::instance::InstanceConfig| &record.id,
    );
    let existing_cfg = ingot_minecraft::instance::prepare_instance(
        "ConflictPack".into(),
        "1.20.1".into(),
        ingot_minecraft::instance::ModLoaderType::Vanilla,
        None,
    )
    .unwrap();
    let first_stage = tempfile::tempdir_in(conflict_root.path()).unwrap();
    std::fs::write(first_stage.path().join("options.txt"), b"original").unwrap();
    super::publish_modpack(
        &conflict_repo,
        conflict_root.path(),
        existing_cfg.clone(),
        first_stage.path(),
    )
    .unwrap();
    let dup_stage = tempfile::tempdir_in(conflict_root.path()).unwrap();
    std::fs::write(dup_stage.path().join("options.txt"), b"duplicate").unwrap();
    let conflict_err = super::publish_modpack(
        &conflict_repo,
        conflict_root.path(),
        existing_cfg.clone(),
        dup_stage.path(),
    );
    assert!(matches!(
        conflict_err,
        Err(crate::content::ContentError::Metadata(
            MetadataError::Conflict(_)
        ))
    ));
    assert_eq!(
        std::fs::read(
            conflict_root
                .path()
                .join(&existing_cfg.id)
                .join("options.txt")
        )
        .unwrap(),
        b"original"
    );
    assert_eq!(conflict_repo.load().unwrap().len(), 1);
}

#[tokio::test]
async fn contained_path_traversal_and_windows_forms_are_rejected_across_pack_paths() {
    let root = tempfile::tempdir().unwrap();

    for unsafe_path in [
        "../escape.jar",
        "mods/../../escape.jar",
        "mods\\..\\escape.jar",
        "C:\\Windows\\System32\\evil.dll",
        "D:evil.jar",
        "\\Windows\\System32\\evil.dll",
        "mods/CON.jar",
        "mods/evil.jar:stream",
        "mods/evil.jar.",
        "mods/evil.jar ",
    ] {
        let unused_server = LoopbackServer::exact_script(vec![]).await;
        let bytes = archive(
            "modrinth.index.json",
            serde_json::json!({
                "dependencies":{"minecraft":"1.20.1"},
                "files":[{"path":unsafe_path,"downloads":[unused_server.base_url()]}]
            }),
            None,
        );
        let result = prepare(root.path(), bytes, "unused", "modrinth").await;
        assert!(
            matches!(result, Err(crate::content::ContentError::UnsafePath(_))),
            "expected UnsafePath for Modrinth path `{unsafe_path}`"
        );
        let _ = unused_server.finish().await;
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }

    for unsafe_overrides in ["../escape", "..\\escape", "C:\\escape"] {
        let bytes = archive(
            "manifest.json",
            serde_json::json!({"minecraft":{"version":"1.20.1"},"overrides":unsafe_overrides,"files":[]}),
            None,
        );
        let result = prepare(root.path(), bytes, "unused", "curseforge").await;
        assert!(
            matches!(result, Err(crate::content::ContentError::UnsafePath(_))),
            "expected UnsafePath for CurseForge overrides `{unsafe_overrides}`"
        );
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }

    for unsafe_filename in [
        "../escape.jar",
        "sub/escape.jar",
        "sub\\escape.jar",
        "..\\escape.jar",
        "C:\\escape.jar",
        "CON.jar",
        "evil.jar.",
    ] {
        let never_downloaded = LoopbackServer::exact_script(vec![]).await;
        let api = LoopbackServer::exact_script(vec![ScriptedExchange::expect_path(
            "/mods/1/files/2",
            ResponseSpec::json(
                200,
                serde_json::json!({
                    "data": {
                        "id": 2,
                        "fileName": unsafe_filename,
                        "downloadUrl": never_downloaded.base_url(),
                        "fileLength": 3,
                        "hashes": []
                    }
                })
                .to_string(),
            ),
        )])
        .await;
        let bytes = archive(
            "manifest.json",
            serde_json::json!({"minecraft":{"version":"1.20.1"},"files":[{"projectID":1,"fileID":2}]}),
            None,
        );
        let result = prepare(root.path(), bytes, api.base_url(), "curseforge").await;
        assert!(
            matches!(
                result,
                Err(crate::content::ContentError::InvalidFilename(_))
            ),
            "expected InvalidFilename for CurseForge fileName `{unsafe_filename}`"
        );
        let _ = never_downloaded.finish().await;
        let _ = api.finish().await;
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }

    for unsafe_entry in [
        "overrides/..\\escape.txt",
        "overrides/C:\\escape.txt",
        "overrides/CON.txt",
        "overrides/file.txt:stream",
        "overrides/trailing.",
    ] {
        let bytes = archive(
            "modrinth.index.json",
            serde_json::json!({"dependencies":{"minecraft":"1.20.1"},"files":[]}),
            Some((unsafe_entry, b"bad")),
        );
        let result = prepare(root.path(), bytes, "unused", "modrinth").await;
        assert!(
            matches!(result, Err(crate::content::ContentError::Io(_))),
            "expected Io for zip override entry `{unsafe_entry}`"
        );
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }

    for bad_name in [
        "..\\a.jar",
        "C:\\a.jar",
        "CON.jar",
        "a.jar:stream",
        "a.jar.",
    ] {
        let never_downloaded = LoopbackServer::exact_script(vec![]).await;
        let result = super::install_content_file(
            &reqwest::Client::new(),
            root.path(),
            "mod",
            never_downloaded.base_url(),
            bad_name,
        )
        .await;
        assert!(
            matches!(
                result,
                Err(crate::content::ContentError::InvalidFilename(_))
            ),
            "expected InvalidFilename for content file `{bad_name}`"
        );
        let _ = never_downloaded.finish().await;
    }
}

#[tokio::test]
async fn interrupted_downloads_fail_and_clean_staging() {
    let client = reqwest::Client::new();

    let closed_fetch = LoopbackServer::exact_script(vec![ScriptedExchange::any_path(
        ResponseSpec::close_connection(),
    )])
    .await;
    let err = fetch_with_progress(&client, closed_fetch.base_url(), "Phase", "Label", None).await;
    assert!(matches!(
        err,
        Err(crate::content::ContentError::Transport(_))
    ));
    let _ = closed_fetch.finish().await;

    let truncated_fetch = LoopbackServer::exact_script(vec![ScriptedExchange::any_path(
        ResponseSpec::bytes(200, b"partial".to_vec()).with_header("Content-Length", "10000"),
    )])
    .await;
    let err =
        fetch_with_progress(&client, truncated_fetch.base_url(), "Phase", "Label", None).await;
    assert!(matches!(
        err,
        Err(crate::content::ContentError::Response(_))
    ));
    let _ = truncated_fetch.finish().await;

    let root = tempfile::tempdir().unwrap();
    let closed_mod = LoopbackServer::exact_script(vec![ScriptedExchange::any_path(
        ResponseSpec::close_connection(),
    )])
    .await;
    let bytes = archive(
        "modrinth.index.json",
        serde_json::json!({
            "dependencies":{"minecraft":"1.20.1"},
            "files":[{"path":"mods/a.jar","downloads":[closed_mod.base_url()]}]
        }),
        None,
    );
    let err = prepare(root.path(), bytes, "unused", "modrinth").await;
    assert!(matches!(
        err,
        Err(crate::content::ContentError::Transport(_))
    ));
    let _ = closed_mod.finish().await;
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);

    let truncated_mod = LoopbackServer::exact_script(vec![ScriptedExchange::any_path(
        ResponseSpec::bytes(200, b"partial".to_vec()).with_header("Content-Length", "10000"),
    )])
    .await;
    let bytes = archive(
        "modrinth.index.json",
        serde_json::json!({
            "dependencies":{"minecraft":"1.20.1"},
            "files":[{"path":"mods/a.jar","downloads":[truncated_mod.base_url()]}]
        }),
        None,
    );
    let err = prepare(root.path(), bytes, "unused", "modrinth").await;
    assert!(matches!(
        err,
        Err(crate::content::ContentError::Response(_))
    ));
    let _ = truncated_mod.finish().await;
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);

    let install_dir = tempfile::tempdir().unwrap();
    let install_root = install_dir.path().join("instances");
    std::fs::create_dir(&install_root).unwrap();
    let repo = std::sync::Arc::new(
        ingot_core::metadata::MetadataRepository::with_deletion_directory(
            install_dir.path().join("instances.json"),
            install_root.clone(),
            |record: &ingot_minecraft::instance::InstanceConfig| &record.id,
        ),
    );
    let truncated_pack = LoopbackServer::exact_script(vec![ScriptedExchange::any_path(
        ResponseSpec::bytes(200, b"partial-zip".to_vec()).with_header("Content-Length", "10000"),
    )])
    .await;
    let installer = super::ModpackInstaller {
        client: reqwest::Client::new(),
        instances: std::sync::Arc::clone(&repo),
        root: install_root.clone(),
        curseforge_base: "http://127.0.0.1:1".into(),
        on_progress: None,
    };
    let install_err = installer
        .install(
            "InterruptedPack",
            "modrinth",
            truncated_pack.base_url(),
            "pack.mrpack",
        )
        .await;
    assert!(matches!(
        install_err,
        Err(crate::content::ContentError::Response(_))
    ));
    let _ = truncated_pack.finish().await;
    assert_eq!(std::fs::read_dir(&install_root).unwrap().count(), 0);
    assert!(repo.load().unwrap().is_empty());
}
