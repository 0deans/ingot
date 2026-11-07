use super::pack::{prepare_modpack, PrepareModpackParams};
use sha1::Digest;
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
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0; 4096];
        assert!(socket.read(&mut request).await.unwrap() > 0);
        let header = format!(
            "HTTP/1.1 {status} Response\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        socket.write_all(header.as_bytes()).await.unwrap();
        socket.write_all(&body).await.unwrap();
    });
    Server { url, task }
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
        Err(crate::minecraft::content::ContentError::HttpStatus(_))
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
        Err(crate::minecraft::content::ContentError::InvalidFilename(_))
    ));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

fn hex_str(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[tokio::test]
async fn prepare_modpack_ftb_downloads_client_files_and_skips_serveronly() {
    let mod_bytes = b"ftb client mod payload";
    let sha1_hex = hex_str(sha1::Sha1::digest(mod_bytes));
    let server = binary_server(mod_bytes.to_vec(), 200).await;

    let manifest_json = serde_json::json!({
        "id": 42,
        "name": "FTB Test Pack",
        "targets": [
            {"name": "minecraft", "type": "game", "version": "1.20.1"},
            {"name": "fabric", "type": "modloader", "version": "0.15.7"}
        ],
        "files": [
            {
                "name": "client-mod.jar",
                "path": "mods",
                "url": server.url,
                "sha1": sha1_hex,
                "size": mod_bytes.len(),
                "serveronly": false
            },
            {
                "name": "server-only.jar",
                "path": "mods",
                "url": "http://127.0.0.1:0/never-called",
                "serveronly": true
            }
        ]
    });

    let manifest_bytes = serde_json::to_vec(&manifest_json).unwrap();
    let root = tempfile::tempdir().unwrap();

    let prepared = prepare_modpack(PrepareModpackParams {
        client: &reqwest::Client::new(),
        curseforge_base: "unused",
        root: root.path(),
        bytes: manifest_bytes,
        source: "ftb",
        filename: "ftb-42-1.json",
        name: "My FTB Pack",
        manifest_url: None,
        on_progress: None,
    })
    .await
    .unwrap();

    assert_eq!(prepared.name, "My FTB Pack");
    assert_eq!(prepared.game_version, "1.20.1");
    assert_eq!(
        prepared.loader,
        crate::minecraft::instance::ModLoaderType::Fabric
    );
    assert_eq!(prepared.loader_version, Some("0.15.7".to_string()));

    let client_path = prepared.stage.path().join("mods").join("client-mod.jar");
    assert!(client_path.exists());
    assert_eq!(std::fs::read(&client_path).unwrap(), mod_bytes);

    let server_path = prepared.stage.path().join("mods").join("server-only.jar");
    assert!(!server_path.exists());
}

#[tokio::test]
async fn prepare_modpack_technic_direct_zip_extracts_and_detects_loader() {
    let mut zip_writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip_writer
        .start_file("bin/version.json", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip_writer
        .write_all(br#"{"id":"1.12.2-forge-14.23.5.2860"}"#)
        .unwrap();

    zip_writer
        .start_file("mods/example.jar", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip_writer.write_all(b"mod content").unwrap();

    let zip_bytes = zip_writer.finish().unwrap().into_inner();
    let root = tempfile::tempdir().unwrap();

    let prepared = prepare_modpack(PrepareModpackParams {
        client: &reqwest::Client::new(),
        curseforge_base: "unused",
        root: root.path(),
        bytes: zip_bytes,
        source: "technic",
        filename: "bteam.zip",
        name: "Technic BTeam",
        manifest_url: None,
        on_progress: None,
    })
    .await
    .unwrap();

    assert_eq!(prepared.name, "Technic BTeam");
    assert_eq!(prepared.game_version, "1.12.2");
    assert_eq!(
        prepared.loader,
        crate::minecraft::instance::ModLoaderType::Forge
    );
    assert_eq!(prepared.loader_version, Some("14.23.5.2860".to_string()));
    assert!(prepared.stage.path().join("mods/example.jar").exists());
}

#[tokio::test]
async fn prepare_modpack_technic_solder_downloads_and_extracts_modules() {
    let mut zip_writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip_writer
        .start_file(
            "mods/solder-mod.jar",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    zip_writer.write_all(b"solder mod content").unwrap();
    let mod_zip_bytes = zip_writer.finish().unwrap().into_inner();

    let md5_hex = hex_str(md5::Md5::digest(&mod_zip_bytes));
    let server = binary_server(mod_zip_bytes, 200).await;

    let manifest_json = serde_json::json!({
        "minecraft": "1.7.10",
        "mods": [
            {
                "name": "solder-mod",
                "version": "1.0",
                "md5": md5_hex,
                "url": server.url
            }
        ]
    });
    let manifest_bytes = serde_json::to_vec(&manifest_json).unwrap();
    let root = tempfile::tempdir().unwrap();

    let prepared = prepare_modpack(PrepareModpackParams {
        client: &reqwest::Client::new(),
        curseforge_base: "unused",
        root: root.path(),
        bytes: manifest_bytes,
        source: "technic",
        filename: "solder-manifest.json",
        name: "Solder Pack",
        manifest_url: None,
        on_progress: None,
    })
    .await
    .unwrap();

    assert_eq!(prepared.name, "Solder Pack");
    assert_eq!(prepared.game_version, "1.7.10");
    assert_eq!(
        prepared.loader,
        crate::minecraft::instance::ModLoaderType::Forge
    );
    assert!(prepared.stage.path().join("mods/solder-mod.jar").exists());
}

#[tokio::test]
async fn prepare_modpack_atlauncher_manifest_downloads_client_mods() {
    let mod_bytes = b"atlauncher mod bytes";
    let sha1_hex = hex_str(sha1::Sha1::digest(mod_bytes));
    let md5_hex = hex_str(md5::Md5::digest(mod_bytes));
    let server = binary_server(mod_bytes.to_vec(), 200).await;

    let manifest_json = serde_json::json!({
        "minecraft": "1.16.5",
        "loader": {
            "type": "forge",
            "version": "36.2.39"
        },
        "mods": [
            {
                "name": "IronChests",
                "file": "ironchest-1.16.5.jar",
                "url": server.url,
                "sha1": sha1_hex,
                "md5": md5_hex,
                "filesize": mod_bytes.len(),
                "optional": false,
                "selected": true,
                "server": false,
                "client": true,
                "type": "mods"
            },
            {
                "name": "ServerOnlyTool",
                "file": "server.jar",
                "url": "http://127.0.0.1:0/never",
                "optional": false,
                "server": true,
                "client": false
            }
        ]
    });

    let manifest_bytes = serde_json::to_vec(&manifest_json).unwrap();
    let root = tempfile::tempdir().unwrap();

    let prepared = prepare_modpack(PrepareModpackParams {
        client: &reqwest::Client::new(),
        curseforge_base: "unused",
        root: root.path(),
        bytes: manifest_bytes,
        source: "atlauncher",
        filename: "atl-manifest.json",
        name: "Crucial Pack",
        manifest_url: None,
        on_progress: None,
    })
    .await
    .unwrap();

    assert_eq!(prepared.name, "Crucial Pack");
    assert_eq!(prepared.game_version, "1.16.5");
    assert_eq!(
        prepared.loader,
        crate::minecraft::instance::ModLoaderType::Forge
    );
    assert_eq!(prepared.loader_version, Some("36.2.39".to_string()));

    let mod_path = prepared
        .stage
        .path()
        .join("mods")
        .join("ironchest-1.16.5.jar");
    assert!(mod_path.exists());
    assert_eq!(std::fs::read(mod_path).unwrap(), mod_bytes);

    let server_mod_path = prepared.stage.path().join("mods").join("server.jar");
    assert!(!server_mod_path.exists());
}

#[tokio::test]
async fn prepare_modpack_atlauncher_nested_loader_and_relative_server_url_with_configs_zip() {
    let mod_bytes = b"security craft jar bytes";
    let md5_hex = hex_str(md5::Md5::digest(mod_bytes));

    let mut zip_writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip_writer
        .start_file(
            "config/atlauncher-config.txt",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    zip_writer.write_all(b"pack configs").unwrap();
    let configs_zip_bytes = zip_writer.finish().unwrap().into_inner();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    let mod_bytes_clone = mod_bytes.to_vec();
    let task = tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let mut buf = [0u8; 2048];
            let n = socket.read(&mut buf).await.unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]);
            let first_line = req.lines().next().unwrap_or("");
            let path = first_line.split_whitespace().nth(1).unwrap_or("");

            if path.contains("Configs.zip") {
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    configs_zip_bytes.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.write_all(&configs_zip_bytes).await;
            } else if path.contains("SecurityCraft") {
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    mod_bytes_clone.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.write_all(&mod_bytes_clone).await;
            } else {
                let _ = socket
                    .write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    )
                    .await;
            }
        }
    });

    let manifest_json = serde_json::json!({
        "version": "v11.3.0",
        "minecraft": "1.21.1",
        "loader": {
            "type": "neoforge",
            "metadata": {
                "minecraft": "1.21.1",
                "version": "21.1.264",
                "rawVersion": "21.1.264"
            }
        },
        "mods": [
            {
                "name": "Security Craft",
                "version": "1.10.2.1",
                "url": "packs/AllTheForge10/files/[1.21.1] SecurityCraft v1.10.2.1.jar",
                "file": "[1.21.1] SecurityCraft v1.10.2.1.jar",
                "download": "server",
                "md5": md5_hex,
                "filesize": mod_bytes.len(),
                "type": "mods",
                "client": true,
                "optional": false
            }
        ]
    });

    let manifest_bytes = serde_json::to_vec(&manifest_json).unwrap();
    let root = tempfile::tempdir().unwrap();
    let manifest_url = format!("{base_url}/packs/AllTheForge10/versions/v11.3.0/Configs.json");

    let prepared = prepare_modpack(PrepareModpackParams {
        client: &reqwest::Client::new(),
        curseforge_base: "unused",
        root: root.path(),
        bytes: manifest_bytes,
        source: "atlauncher",
        filename: "atlauncher-AllTheForge10-v11.3.0.json",
        name: "All The Forge 10",
        manifest_url: Some(&manifest_url),
        on_progress: None,
    })
    .await
    .unwrap();

    assert_eq!(prepared.game_version, "1.21.1");
    assert_eq!(
        prepared.loader,
        crate::minecraft::instance::ModLoaderType::NeoForge
    );
    assert_eq!(prepared.loader_version, Some("21.1.264".to_string()));

    let mod_path = prepared
        .stage
        .path()
        .join("mods")
        .join("[1.21.1] SecurityCraft v1.10.2.1.jar");
    assert!(mod_path.exists());
    assert_eq!(std::fs::read(mod_path).unwrap(), mod_bytes);

    let config_path = prepared
        .stage
        .path()
        .join("config")
        .join("atlauncher-config.txt");
    assert!(config_path.exists());
    assert_eq!(std::fs::read(config_path).unwrap(), b"pack configs");

    task.abort();
}

#[tokio::test]
async fn prepare_modpack_technic_direct_zip_mc_version_fallback() {
    let mut zip_writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip_writer
        .start_file("mods/sample.jar", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip_writer.write_all(b"sample mod").unwrap();
    let zip_bytes = zip_writer.finish().unwrap().into_inner();

    let root = tempfile::tempdir().unwrap();
    let prepared = prepare_modpack(PrepareModpackParams {
        client: &reqwest::Client::new(),
        curseforge_base: "unused",
        root: root.path(),
        bytes: zip_bytes,
        source: "technic",
        filename: "nanatsu-dark-servidor-2.15b-mc1.7.10.zip",
        name: "Nanatsu Dark",
        manifest_url: None,
        on_progress: None,
    })
    .await
    .unwrap();

    assert_eq!(prepared.name, "Nanatsu Dark");
    assert_eq!(prepared.game_version, "1.7.10");
    assert_eq!(
        prepared.loader,
        crate::minecraft::instance::ModLoaderType::Forge
    );
    assert!(prepared.stage.path().join("mods/sample.jar").exists());
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

#[tokio::test]
async fn prepare_modpack_technic_detects_loader_from_modpack_jar_and_tolerates_crc_mismatch() {
    let mut jar_writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    jar_writer
        .start_file("version.json", zip::write::SimpleFileOptions::default())
        .unwrap();
    jar_writer
        .write_all(br#"{"id": "1.12.2-forge-14.23.5.2860"}"#)
        .unwrap();
    let jar_bytes = jar_writer.finish().unwrap().into_inner();

    let mut zip_writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip_writer
        .start_file("bin/modpack.jar", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip_writer.write_all(&jar_bytes).unwrap();

    let test_file_payload = b"RLCraft mod config content with valid data";
    zip_writer
        .start_file(
            "config/test_config.cfg",
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
    zip_writer.write_all(test_file_payload).unwrap();
    let mut outer_zip_bytes = zip_writer.finish().unwrap().into_inner();

    let target = b"config/test_config.cfg";
    let positions: Vec<usize> = outer_zip_bytes
        .windows(target.len())
        .enumerate()
        .filter_map(|(idx, window)| if window == target { Some(idx) } else { None })
        .collect();
    assert_eq!(positions.len(), 2, "Expected local and central headers");
    let local_crc_pos = positions[0] - 16;
    let central_crc_pos = positions[1] - 30;

    for i in 0..4 {
        outer_zip_bytes[local_crc_pos + i] ^= 0xFF;
        outer_zip_bytes[central_crc_pos + i] ^= 0xFF;
    }

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

    let root = tempfile::tempdir().unwrap();
    let prepared = prepare_modpack(PrepareModpackParams {
        client: &reqwest::Client::new(),
        curseforge_base: "unused",
        root: root.path(),
        bytes: outer_zip_bytes,
        source: "technic",
        filename: "rlcraft-v2.9.3.zip",
        name: "RLCraft",
        manifest_url: None,
        on_progress: Some(&reporter),
    })
    .await
    .unwrap();

    assert_eq!(prepared.name, "RLCraft");
    assert_eq!(prepared.game_version, "1.12.2");
    assert_eq!(
        prepared.loader,
        crate::minecraft::instance::ModLoaderType::Forge
    );
    assert_eq!(prepared.loader_version, Some("14.23.5.2860".to_string()));

    let extracted_config = prepared.stage.path().join("config/test_config.cfg");
    assert!(extracted_config.exists());
    assert_eq!(std::fs::read(&extracted_config).unwrap(), test_file_payload);

    let extracted_jar = prepared.stage.path().join("bin/modpack.jar");
    assert!(extracted_jar.exists());
    assert_eq!(std::fs::read(&extracted_jar).unwrap(), jar_bytes);

    let recorded = reports.lock().unwrap();
    let extract_reports: Vec<_> = recorded
        .iter()
        .filter(|(_, _, _, detail)| detail == "Extracting modpack archive...")
        .collect();
    assert_eq!(extract_reports.len(), 2);
    assert_eq!(extract_reports[0].1, 0);
    assert_eq!(extract_reports[1].1, 1);
}

#[tokio::test]
async fn fetch_with_progress_reports_smooth_byte_progress() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let test_data = vec![0x42u8; 200_000];
    let payload = test_data.clone();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let _ = socket.read(&mut buf).await.unwrap();
        let response_headers = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            payload.len()
        );
        socket.write_all(response_headers.as_bytes()).await.unwrap();
        for chunk in payload.chunks(50_000) {
            socket.write_all(chunk).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    });

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

    let client = reqwest::Client::new();
    let downloaded = super::pack::fetch_with_progress(
        &client,
        &format!("http://{addr}"),
        "Installing Modpack",
        "Downloading modpack archive",
        Some(&reporter),
    )
    .await
    .unwrap();

    assert_eq!(downloaded, test_data);
    server.await.unwrap();

    let recorded = reports.lock().unwrap();
    assert!(!recorded.is_empty());
    assert_eq!(recorded[0].0, "Installing Modpack");
    assert!(recorded
        .iter()
        .any(|(_, cur, tot, _)| *cur == 200_000 && *tot == 200_000));
}
