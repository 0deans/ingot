use super::super::pack::{fetch_with_progress, prepare_modpack, PrepareModpackParams};
use super::binary_server;
use ingot_test_support::{LoopbackServer, ResponseSpec, ScriptedExchange};
use sha1::Digest;
use std::io::{Cursor, Write};

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

    let mod_bytes_clone = mod_bytes.to_vec();
    let server = LoopbackServer::with_handler(move |request| {
        if request.path.contains("Configs.zip") {
            ResponseSpec::bytes(200, configs_zip_bytes.clone())
        } else if request.path.contains("SecurityCraft") {
            ResponseSpec::bytes(200, mod_bytes_clone.clone())
        } else {
            ResponseSpec::status(404)
        }
    })
    .await;
    let base_url = server.base_url().to_string();

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
    let test_data = vec![0x42u8; 200_000];
    let server = LoopbackServer::exact_script(vec![ScriptedExchange::any_path(
        ResponseSpec::bytes(200, test_data.clone())
            .with_chunks(50_000, std::time::Duration::from_millis(5)),
    )])
    .await;

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
    let downloaded = fetch_with_progress(
        &client,
        server.base_url(),
        "Installing Modpack",
        "Downloading modpack archive",
        Some(&reporter),
    )
    .await
    .unwrap();

    assert_eq!(downloaded, test_data);
    let _ = server.finish().await;

    let recorded = reports.lock().unwrap();
    assert!(!recorded.is_empty());
    assert_eq!(recorded[0].0, "Installing Modpack");
    assert!(recorded
        .iter()
        .any(|(_, cur, tot, _)| *cur == 200_000 && *tot == 200_000));
}
