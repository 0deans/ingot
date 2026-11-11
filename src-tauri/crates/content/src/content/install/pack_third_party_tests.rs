use super::super::{
    pack::{prepare_modpack, prepare_modpack_with_cdn, PrepareModpackParams},
    support::fetch_with_progress,
};
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
        ingot_minecraft::instance::ModLoaderType::Fabric
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
        ingot_minecraft::instance::ModLoaderType::Forge
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
        ingot_minecraft::instance::ModLoaderType::Forge
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
        ingot_minecraft::instance::ModLoaderType::Forge
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
        ingot_minecraft::instance::ModLoaderType::NeoForge
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
        ingot_minecraft::instance::ModLoaderType::Forge
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
    let reporter: ingot_minecraft::launcher::progress::ProgressCallback =
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
        ingot_minecraft::instance::ModLoaderType::Forge
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
    let reporter: ingot_minecraft::launcher::progress::ProgressCallback =
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

#[tokio::test]
async fn prepare_modpack_ftb_resolves_curseforge_refs_and_paths() {
    let custom_bytes = b"ftb-cf-custom-file";
    let custom_sha1 = hex_str(sha1::Sha1::digest(custom_bytes));
    let fallback_bytes = b"ftb-cf-fallback-file";
    let fallback_md5 = hex_str(md5::Md5::digest(fallback_bytes));
    let config_bytes = b"ftb-cf-config-file";
    let config_sha1 = hex_str(sha1::Sha1::digest(config_bytes));

    let cdn = LoopbackServer::exact_script(vec![ScriptedExchange::expect_path(
        "/4013/966/upstream-orig.jar",
        ResponseSpec::bytes(200, custom_bytes.to_vec()),
    )])
    .await;

    let download = LoopbackServer::exact_script(vec![
        ScriptedExchange::expect_path(
            "/files/fallback.jar",
            ResponseSpec::bytes(200, fallback_bytes.to_vec()),
        ),
        ScriptedExchange::expect_path(
            "/files/cfg.json",
            ResponseSpec::bytes(200, config_bytes.to_vec()),
        ),
    ])
    .await;

    let fallback_url = download.url("/files/fallback.jar");
    let config_url = download.url("/files/cfg.json");
    let api = LoopbackServer::exact_script(vec![
        ScriptedExchange::expect_path(
            "/mods/101/files/4013966",
            ResponseSpec::json(
                200,
                serde_json::json!({
                    "data": {
                        "id": 4013966,
                        "fileName": "upstream-orig.jar",
                        "downloadUrl": null,
                        "fileLength": custom_bytes.len(),
                        "hashes": [{"algo": 1, "value": custom_sha1}]
                    }
                })
                .to_string(),
            ),
        ),
        ScriptedExchange::expect_path(
            "/mods/102/files/2002",
            ResponseSpec::json(
                200,
                serde_json::json!({
                    "data": {
                        "id": 2002,
                        "fileName": "resolved-default.jar",
                        "downloadUrl": fallback_url,
                        "fileLength": fallback_bytes.len(),
                        "hashes": [{"algo": 2, "value": fallback_md5}]
                    }
                })
                .to_string(),
            ),
        ),
        ScriptedExchange::expect_path(
            "/mods/103/files/2003",
            ResponseSpec::json(
                200,
                serde_json::json!({
                    "data": {
                        "id": 2003,
                        "fileName": "custom-config.json",
                        "downloadUrl": config_url,
                        "fileLength": config_bytes.len(),
                        "hashes": [{"algo": 1, "value": config_sha1}]
                    }
                })
                .to_string(),
            ),
        ),
    ])
    .await;

    let manifest_json = serde_json::json!({
        "name": "FTB CF Pack",
        "targets": [{"name": "minecraft", "type": "game", "version": "1.20.1"}],
        "files": [
            {
                "name": "named-override.jar",
                "path": "./mods/",
                "curseforge": {"project": 101, "file": 4013966}
            },
            {
                "name": "",
                "path": "",
                "curseforge": {"project": 102, "file": 2002}
            },
            {
                "name": "   ",
                "path": "/config/custom",
                "curseforge": {"project": 103, "file": 2003}
            }
        ]
    });

    let root = tempfile::tempdir().unwrap();
    let prepared = prepare_modpack_with_cdn(
        PrepareModpackParams {
            client: &reqwest::Client::new(),
            curseforge_base: api.base_url(),
            root: root.path(),
            bytes: serde_json::to_vec(&manifest_json).unwrap(),
            source: "ftb",
            filename: "ftb-99-1.json",
            name: "",
            manifest_url: None,
            on_progress: None,
        },
        cdn.base_url(),
    )
    .await
    .unwrap();

    assert_eq!(
        std::fs::read(prepared.stage.path().join("mods/named-override.jar")).unwrap(),
        custom_bytes
    );
    assert_eq!(
        std::fs::read(prepared.stage.path().join("mods/resolved-default.jar")).unwrap(),
        fallback_bytes
    );
    assert_eq!(
        std::fs::read(
            prepared
                .stage
                .path()
                .join("config/custom/custom-config.json")
        )
        .unwrap(),
        config_bytes
    );
    drop(prepared);
    let _ = cdn.finish().await;
    let _ = download.finish().await;
    let _ = api.finish().await;
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn prepare_modpack_ftb_rejects_unsafe_paths_and_cf_errors_and_cleans_staging() {
    let root = tempfile::tempdir().unwrap();

    for (path, name) in [
        ("../escape", "mod.jar"),
        ("mods", "..\\escape.jar"),
        ("C:\\Windows", "evil.jar"),
        ("mods", "CON.jar"),
        ("mods", "evil.jar:stream"),
    ] {
        let unused = LoopbackServer::exact_script(vec![]).await;
        let manifest = serde_json::json!({
            "name": "Bad FTB",
            "targets": [{"name": "minecraft", "type": "game", "version": "1.20.1"}],
            "files": [{"name": name, "path": path, "url": unused.base_url()}]
        });
        let err = prepare_modpack(PrepareModpackParams {
            client: &reqwest::Client::new(),
            curseforge_base: "unused",
            root: root.path(),
            bytes: serde_json::to_vec(&manifest).unwrap(),
            source: "ftb",
            filename: "ftb-bad.json",
            name: "",
            manifest_url: None,
            on_progress: None,
        })
        .await;
        assert!(
            matches!(err, Err(crate::content::ContentError::UnsafePath(_))),
            "expected UnsafePath for FTB path `{path}` name `{name}`"
        );
        let _ = unused.finish().await;
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }

    let api_err = LoopbackServer::exact_script(vec![ScriptedExchange::expect_path(
        "/mods/1/files/2",
        ResponseSpec::status(404),
    )])
    .await;
    let manifest = serde_json::json!({
        "name": "FTB CF 404",
        "targets": [{"name": "minecraft", "type": "game", "version": "1.20.1"}],
        "files": [{"name": "mod.jar", "path": "mods", "curseforge": {"project": 1, "file": 2}}]
    });
    let err = prepare_modpack(PrepareModpackParams {
        client: &reqwest::Client::new(),
        curseforge_base: api_err.base_url(),
        root: root.path(),
        bytes: serde_json::to_vec(&manifest).unwrap(),
        source: "ftb",
        filename: "ftb-404.json",
        name: "",
        manifest_url: None,
        on_progress: None,
    })
    .await;
    assert!(matches!(
        err,
        Err(crate::content::ContentError::HttpStatus(_))
    ));
    let _ = api_err.finish().await;
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);

    let never_downloaded = LoopbackServer::exact_script(vec![]).await;
    let cf_traversal_api = LoopbackServer::exact_script(vec![ScriptedExchange::expect_path(
        "/mods/1/files/2",
        ResponseSpec::json(
            200,
            serde_json::json!({
                "data": {
                    "id": 2,
                    "fileName": "..\\escape.jar",
                    "downloadUrl": never_downloaded.base_url(),
                    "fileLength": 3,
                    "hashes": []
                }
            })
            .to_string(),
        ),
    )])
    .await;
    let manifest = serde_json::json!({
        "name": "FTB CF Traversal",
        "targets": [{"name": "minecraft", "type": "game", "version": "1.20.1"}],
        "files": [{"name": "", "path": "", "curseforge": {"project": 1, "file": 2}}]
    });
    let err = prepare_modpack(PrepareModpackParams {
        client: &reqwest::Client::new(),
        curseforge_base: cf_traversal_api.base_url(),
        root: root.path(),
        bytes: serde_json::to_vec(&manifest).unwrap(),
        source: "ftb",
        filename: "ftb-cf-trav.json",
        name: "",
        manifest_url: None,
        on_progress: None,
    })
    .await;
    assert!(matches!(
        err,
        Err(crate::content::ContentError::UnsafePath(_))
    ));
    let _ = never_downloaded.finish().await;
    let _ = cf_traversal_api.finish().await;
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn prepare_modpack_atlauncher_resolves_curseforge_refs_and_rejects_errors() {
    let mod_bytes = b"atl-cf-override-bytes";
    let mod_sha1 = hex_str(sha1::Sha1::digest(mod_bytes));
    let pack_bytes = b"atl-cf-resourcepack-bytes";
    let pack_md5 = hex_str(md5::Md5::digest(pack_bytes));

    let cdn = LoopbackServer::exact_script(vec![ScriptedExchange::expect_path(
        "/4013/966/upstream-mod.jar",
        ResponseSpec::bytes(200, mod_bytes.to_vec()),
    )])
    .await;
    let download = LoopbackServer::exact_script(vec![ScriptedExchange::expect_path(
        "/rp.zip",
        ResponseSpec::bytes(200, pack_bytes.to_vec()),
    )])
    .await;
    let rp_url = download.url("/rp.zip");
    let api = LoopbackServer::exact_script(vec![
        ScriptedExchange::expect_path(
            "/mods/50/files/4013966",
            ResponseSpec::json(
                200,
                serde_json::json!({
                    "data": {
                        "id": 4013966,
                        "fileName": "upstream-mod.jar",
                        "downloadUrl": null,
                        "fileLength": mod_bytes.len(),
                        "hashes": [{"algo": 1, "value": mod_sha1}]
                    }
                })
                .to_string(),
            ),
        ),
        ScriptedExchange::expect_path(
            "/mods/51/files/600",
            ResponseSpec::json(
                200,
                serde_json::json!({
                    "data": {
                        "id": 600,
                        "fileName": "fallback-rp.zip",
                        "downloadUrl": rp_url,
                        "fileLength": pack_bytes.len(),
                        "hashes": [{"algo": 2, "value": pack_md5}]
                    }
                })
                .to_string(),
            ),
        ),
    ])
    .await;

    let manifest_json = serde_json::json!({
        "minecraft": "1.20.1",
        "loader": {"type": "fabric", "version": "0.15.7"},
        "mods": [
            {
                "name": "Mod Override",
                "file": "atl-override.jar",
                "download": "curse",
                "curse_id": 50,
                "curse_file_id": 4013966,
                "type": "mods"
            },
            {
                "name": "Resource Fallback",
                "download": "curseforge",
                "curse_id": 51,
                "curse_file_id": 600,
                "type": "resourcepack"
            }
        ]
    });

    let root = tempfile::tempdir().unwrap();
    let prepared = prepare_modpack_with_cdn(
        PrepareModpackParams {
            client: &reqwest::Client::new(),
            curseforge_base: api.base_url(),
            root: root.path(),
            bytes: serde_json::to_vec(&manifest_json).unwrap(),
            source: "atlauncher",
            filename: "atl-cf.json",
            name: "ATL CF",
            manifest_url: None,
            on_progress: None,
        },
        cdn.base_url(),
    )
    .await
    .unwrap();

    assert_eq!(
        std::fs::read(prepared.stage.path().join("mods/atl-override.jar")).unwrap(),
        mod_bytes
    );
    assert_eq!(
        std::fs::read(prepared.stage.path().join("resourcepacks/fallback-rp.zip")).unwrap(),
        pack_bytes
    );
    drop(prepared);
    let _ = cdn.finish().await;
    let _ = download.finish().await;
    let _ = api.finish().await;
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);

    for bad_file in ["../escape.jar", "..\\escape.jar", "C:\\evil.jar", "CON.jar"] {
        let never_downloaded = LoopbackServer::exact_script(vec![]).await;
        let bad_api = LoopbackServer::exact_script(vec![ScriptedExchange::expect_path(
            "/mods/50/files/60",
            ResponseSpec::json(
                200,
                serde_json::json!({
                    "data": {
                        "id": 60,
                        "fileName": bad_file,
                        "downloadUrl": never_downloaded.base_url(),
                        "fileLength": 3,
                        "hashes": []
                    }
                })
                .to_string(),
            ),
        )])
        .await;
        let bad_manifest = serde_json::json!({
            "minecraft": "1.20.1",
            "mods": [{
                "name": "Bad Mod",
                "download": "curseforge",
                "curse_id": 50,
                "curse_file_id": 60,
                "type": "mods"
            }]
        });
        let err = prepare_modpack(PrepareModpackParams {
            client: &reqwest::Client::new(),
            curseforge_base: bad_api.base_url(),
            root: root.path(),
            bytes: serde_json::to_vec(&bad_manifest).unwrap(),
            source: "atlauncher",
            filename: "atl-bad.json",
            name: "ATL Bad",
            manifest_url: None,
            on_progress: None,
        })
        .await;
        assert!(
            matches!(err, Err(crate::content::ContentError::UnsafePath(_))),
            "expected UnsafePath for ATLauncher file `{bad_file}`"
        );
        let _ = never_downloaded.finish().await;
        let _ = bad_api.finish().await;
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }

    let unsupported_api = LoopbackServer::exact_script(vec![ScriptedExchange::expect_path(
        "/mods/50/files/61",
        ResponseSpec::json(
            200,
            serde_json::json!({
                "data": {
                    "id": 61,
                    "fileName": "ok.jar",
                    "downloadUrl": "http://127.0.0.1:1/unused",
                    "fileLength": 3,
                    "hashes": [{"algo": 99, "value": "abcd"}]
                }
            })
            .to_string(),
        ),
    )])
    .await;
    let bad_algo_manifest = serde_json::json!({
        "minecraft": "1.20.1",
        "mods": [{
            "name": "Bad Algo",
            "download": "curse",
            "curse_id": 50,
            "curse_file_id": 61,
            "type": "mods"
        }]
    });
    let err = prepare_modpack(PrepareModpackParams {
        client: &reqwest::Client::new(),
        curseforge_base: unsupported_api.base_url(),
        root: root.path(),
        bytes: serde_json::to_vec(&bad_algo_manifest).unwrap(),
        source: "atlauncher",
        filename: "atl-bad-algo.json",
        name: "ATL Bad Algo",
        manifest_url: None,
        on_progress: None,
    })
    .await;
    assert!(matches!(
        err,
        Err(crate::content::ContentError::Integrity(_))
    ));
    let _ = unsupported_api.finish().await;
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn prepare_modpack_technic_solder_propagates_http_checksum_zip_and_traversal_errors() {
    let root = tempfile::tempdir().unwrap();

    let http_fail = binary_server(b"bad gateway".to_vec(), 502).await;
    let manifest_http = serde_json::json!({
        "minecraft": "1.12.2",
        "mods": [{"name": "broken", "md5": null, "url": http_fail.url}]
    });
    let err = prepare_modpack(PrepareModpackParams {
        client: &reqwest::Client::new(),
        curseforge_base: "unused",
        root: root.path(),
        bytes: serde_json::to_vec(&manifest_http).unwrap(),
        source: "technic",
        filename: "solder.json",
        name: "Solder",
        manifest_url: None,
        on_progress: None,
    })
    .await;
    assert!(matches!(
        err,
        Err(crate::content::ContentError::HttpStatus(_))
    ));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);

    let md5_fail = binary_server(b"corrupt mod payload".to_vec(), 200).await;
    let manifest_md5 = serde_json::json!({
        "minecraft": "1.12.2",
        "mods": [{"name": "bad-md5", "md5": "00000000000000000000000000000000", "url": md5_fail.url}]
    });
    let err = prepare_modpack(PrepareModpackParams {
        client: &reqwest::Client::new(),
        curseforge_base: "unused",
        root: root.path(),
        bytes: serde_json::to_vec(&manifest_md5).unwrap(),
        source: "technic",
        filename: "solder.json",
        name: "Solder",
        manifest_url: None,
        on_progress: None,
    })
    .await;
    assert!(matches!(
        err,
        Err(crate::content::ContentError::Integrity(_))
    ));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);

    let bad_zip_bytes = b"not a zip archive".to_vec();
    let bad_zip_md5 = hex_str(md5::Md5::digest(&bad_zip_bytes));
    let bad_zip_server = binary_server(bad_zip_bytes, 200).await;
    let manifest_zip = serde_json::json!({
        "minecraft": "1.12.2",
        "mods": [{"name": "bad-zip", "md5": bad_zip_md5, "url": bad_zip_server.url}]
    });
    let err = prepare_modpack(PrepareModpackParams {
        client: &reqwest::Client::new(),
        curseforge_base: "unused",
        root: root.path(),
        bytes: serde_json::to_vec(&manifest_zip).unwrap(),
        source: "technic",
        filename: "solder.json",
        name: "Solder",
        manifest_url: None,
        on_progress: None,
    })
    .await;
    assert!(matches!(err, Err(crate::content::ContentError::Zip(_))));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);

    let mut trav_writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    trav_writer
        .start_file(
            "mods\\..\\escape.jar",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    trav_writer.write_all(b"evil").unwrap();
    let trav_zip = trav_writer.finish().unwrap().into_inner();
    let trav_md5 = hex_str(md5::Md5::digest(&trav_zip));
    let trav_server = binary_server(trav_zip, 200).await;
    let manifest_trav = serde_json::json!({
        "minecraft": "1.12.2",
        "mods": [{"name": "trav", "md5": trav_md5, "url": trav_server.url}]
    });
    let err = prepare_modpack(PrepareModpackParams {
        client: &reqwest::Client::new(),
        curseforge_base: "unused",
        root: root.path(),
        bytes: serde_json::to_vec(&manifest_trav).unwrap(),
        source: "technic",
        filename: "solder.json",
        name: "Solder",
        manifest_url: None,
        on_progress: None,
    })
    .await;
    assert!(matches!(err, Err(crate::content::ContentError::Io(_))));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);

    let interrupted_server = LoopbackServer::exact_script(vec![ScriptedExchange::any_path(
        ResponseSpec::bytes(200, b"partial".to_vec()).with_header("Content-Length", "10000"),
    )])
    .await;
    let manifest_interrupted = serde_json::json!({
        "minecraft": "1.12.2",
        "mods": [{"name": "truncated", "md5": null, "url": interrupted_server.base_url()}]
    });
    let err = prepare_modpack(PrepareModpackParams {
        client: &reqwest::Client::new(),
        curseforge_base: "unused",
        root: root.path(),
        bytes: serde_json::to_vec(&manifest_interrupted).unwrap(),
        source: "technic",
        filename: "solder.json",
        name: "Solder",
        manifest_url: None,
        on_progress: None,
    })
    .await;
    assert!(matches!(
        err,
        Err(crate::content::ContentError::Response(_))
    ));
    let _ = interrupted_server.finish().await;
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
