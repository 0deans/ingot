//! Tests for Minecraft content searching, details retrieval, and installation.
use super::{
    curseforge::CurseForgeClient,
    get_content_details,
    install::{
        construct_curseforge_cdn_url, extract_zip_overrides, parse_curseforge_loader,
        parse_modrinth_loader, target_subfolder_for_project_type, CurseForgeManifestModLoader,
    },
    models::ContentSearchQuery,
    modrinth::ModrinthClient,
    parse_content_source_and_id, search_content,
};
use crate::minecraft::instance::ModLoaderType;
use reqwest::Client;
use std::{
    collections::HashMap,
    io::{Cursor, Write},
    sync::Arc,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use zip::{write::SimpleFileOptions, ZipWriter};

#[test]
fn test_target_subfolder_for_project_type() {
    assert_eq!(target_subfolder_for_project_type("mod"), "mods");
    assert_eq!(target_subfolder_for_project_type("MOD"), "mods");
    assert_eq!(
        target_subfolder_for_project_type("resourcepack"),
        "resourcepacks"
    );
    assert_eq!(target_subfolder_for_project_type("shader"), "shaderpacks");
    assert_eq!(target_subfolder_for_project_type("other"), "mods");
}

#[test]
fn test_parse_modrinth_loader() {
    let mut deps = HashMap::new();
    deps.insert("fabric-loader".to_string(), "0.15.0".to_string());
    assert_eq!(
        parse_modrinth_loader(&deps),
        (ModLoaderType::Fabric, Some("0.15.0".to_string()))
    );

    let mut deps = HashMap::new();
    deps.insert("forge".to_string(), "47.2.0".to_string());
    assert_eq!(
        parse_modrinth_loader(&deps),
        (ModLoaderType::Forge, Some("47.2.0".to_string()))
    );

    let mut deps = HashMap::new();
    deps.insert("neoforge".to_string(), "20.4.0".to_string());
    assert_eq!(
        parse_modrinth_loader(&deps),
        (ModLoaderType::NeoForge, Some("20.4.0".to_string()))
    );

    let mut deps = HashMap::new();
    deps.insert("quilt-loader".to_string(), "0.20.0".to_string());
    assert_eq!(
        parse_modrinth_loader(&deps),
        (ModLoaderType::Quilt, Some("0.20.0".to_string()))
    );

    let empty = HashMap::new();
    assert_eq!(
        parse_modrinth_loader(&empty),
        (ModLoaderType::Vanilla, None)
    );
}

#[test]
fn test_parse_curseforge_loader() {
    let loaders = vec![CurseForgeManifestModLoader {
        id: "fabric-0.14.22".to_string(),
    }];
    assert_eq!(
        parse_curseforge_loader(&loaders),
        (ModLoaderType::Fabric, Some("0.14.22".to_string()))
    );

    let loaders = vec![CurseForgeManifestModLoader {
        id: "forge-40.1.0".to_string(),
    }];
    assert_eq!(
        parse_curseforge_loader(&loaders),
        (ModLoaderType::Forge, Some("40.1.0".to_string()))
    );

    let loaders = vec![CurseForgeManifestModLoader {
        id: "neoforge-20.2.1".to_string(),
    }];
    assert_eq!(
        parse_curseforge_loader(&loaders),
        (ModLoaderType::NeoForge, Some("20.2.1".to_string()))
    );

    let loaders = vec![CurseForgeManifestModLoader {
        id: "quilt-0.19.0".to_string(),
    }];
    assert_eq!(
        parse_curseforge_loader(&loaders),
        (ModLoaderType::Quilt, Some("0.19.0".to_string()))
    );

    let empty: Vec<CurseForgeManifestModLoader> = Vec::new();
    assert_eq!(
        parse_curseforge_loader(&empty),
        (ModLoaderType::Vanilla, None)
    );
}

#[test]
fn test_construct_curseforge_cdn_url() {
    assert_eq!(
        construct_curseforge_cdn_url(123, "test.jar"),
        "https://edge.forgecdn.net/files/0/123/test.jar"
    );
    assert_eq!(
        construct_curseforge_cdn_url(1234567, "test.jar"),
        "https://edge.forgecdn.net/files/1234/567/test.jar"
    );
}

#[test]
fn test_parse_content_source_and_id() {
    assert_eq!(
        parse_content_source_and_id("modrinth", "mr:sodium"),
        ("modrinth", "sodium")
    );
    assert_eq!(
        parse_content_source_and_id("modrinth", "cf:310341"),
        ("curseforge", "310341")
    );
    assert_eq!(
        parse_content_source_and_id("", "modrinth:fabric-api"),
        ("modrinth", "fabric-api")
    );
    assert_eq!(
        parse_content_source_and_id("", "curseforge:238222"),
        ("curseforge", "238222")
    );
    assert_eq!(
        parse_content_source_and_id("modrinth", "iris"),
        ("modrinth", "iris")
    );
}

#[test]
fn test_extract_zip_overrides() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let target = temp_dir.path().to_path_buf();

    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut zip = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default();

        zip.start_file("overrides/config/test.cfg", options)
            .expect("start_file");
        zip.write_all(b"key=value").expect("write");

        zip.start_file("client-overrides/options.txt", options)
            .expect("start_file");
        zip.write_all(b"fov:70").expect("write");

        zip.start_file("ignored/file.txt", options)
            .expect("start_file");
        zip.write_all(b"ignore").expect("write");

        zip.start_file("overrides/../escape.txt", options)
            .expect("start_file");
        zip.write_all(b"escape").expect("write");

        zip.start_file("overrides/../../escape_parent.txt", options)
            .expect("start_file");
        zip.write_all(b"escape").expect("write");

        zip.start_file("overrides/stream.txt:stream", options)
            .expect("start_file");
        zip.write_all(b"escape").expect("write");

        zip.start_file("overrides/trailing_dot.", options)
            .expect("start_file");
        zip.write_all(b"escape").expect("write");

        zip.finish().expect("finish");
    }

    let cursor = Cursor::new(buf);
    let mut zip_reader = zip::ZipArchive::new(cursor).expect("zip archive");

    let res = extract_zip_overrides(
        &mut zip_reader,
        &["overrides/", "client-overrides/"],
        &target,
    );
    assert!(res.is_ok());

    assert!(target.join("config/test.cfg").exists());
    assert_eq!(
        std::fs::read_to_string(target.join("config/test.cfg")).unwrap(),
        "key=value"
    );

    assert!(target.join("options.txt").exists());
    assert_eq!(
        std::fs::read_to_string(target.join("options.txt")).unwrap(),
        "fov:70"
    );

    assert!(!target.join("ignored/file.txt").exists());
    assert!(!target.join("escape.txt").exists());
    assert!(!target.join("../escape.txt").exists());
    assert!(!target.join("escape_parent.txt").exists());
    assert!(!temp_dir.path().join("escape_parent.txt").exists());
    assert!(!target.join("stream.txt:stream").exists());
    assert!(!target.join("trailing_dot.").exists());
}

struct MockServer {
    base_url: String,
    _handle: tokio::task::JoinHandle<()>,
}

impl MockServer {
    async fn start<F>(handler: F) -> Self
    where
        F: Fn(&str, &str) -> (u16, String) + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{addr}");
        let handler = Arc::new(handler);

        let handle = tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let handler = Arc::clone(&handler);

                tokio::spawn(async move {
                    let mut buffer = [0u8; 4096];
                    let count = match stream.read(&mut buffer).await {
                        Ok(c) if c > 0 => c,
                        _ => return,
                    };
                    let req_str = String::from_utf8_lossy(&buffer[..count]);
                    let mut lines = req_str.lines();
                    let req_line = lines.next().unwrap_or("");
                    let mut parts = req_line.split_whitespace();
                    let method = parts.next().unwrap_or("GET");
                    let path = parts.next().unwrap_or("/");

                    let (status, resp_body) = handler(method, path);
                    let response = format!(
                        "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        resp_body.len(),
                        resp_body
                    );

                    let _ = stream.write_all(response.as_bytes()).await;
                    let _ = stream.shutdown().await;
                });
            }
        });

        Self {
            base_url,
            _handle: handle,
        }
    }
}

#[tokio::test]
async fn test_modrinth_client_search_loopback() {
    let server = MockServer::start(|_method, path| {
        if path.contains("/search") {
            (
                200,
                r#"{
                    "hits": [
                        {
                            "project_id": "AANobbMI",
                            "project_type": "mod",
                            "slug": "sodium",
                            "author": "jellysquid3",
                            "title": "Sodium",
                            "description": "Modern rendering engine",
                            "categories": ["optimization"],
                            "downloads": 10000000,
                            "follows": 50000,
                            "icon_url": "https://example.com/icon.png"
                        }
                    ],
                    "total_hits": 1,
                    "offset": 0,
                    "limit": 10
                }"#
                .to_string(),
            )
        } else {
            (404, "Not Found".to_string())
        }
    })
    .await;

    let client = ModrinthClient::with_endpoint(Client::new(), &server.base_url);
    let query = ContentSearchQuery {
        project_type: "mod",
        query: Some("sodium"),
        game_version: Some("1.20.1"),
        loader: Some("fabric"),
        sort: Some("downloads"),
        offset: 0,
        limit: 10,
    };

    let result = client.search(&query).await.expect("search succeeded");
    assert_eq!(result.total_hits, 1);
    assert_eq!(result.items.len(), 1);
    let item = &result.items[0];
    assert_eq!(item.id, "mr:AANobbMI");
    assert_eq!(item.slug, "sodium");
    assert_eq!(item.downloads, 10000000);
}

#[tokio::test]
async fn test_modrinth_client_get_details_loopback() {
    let server = MockServer::start(|_method, path| {
        if path.ends_with("/version") {
            (
                200,
                r#"[
                    {
                        "id": "v123",
                        "name": "Sodium 0.5.8",
                        "version_number": "0.5.8",
                        "game_versions": ["1.20.1"],
                        "loaders": ["fabric"],
                        "version_type": "release",
                        "date_published": "2024-01-01T00:00:00Z",
                        "downloads": 100000,
                        "files": [
                            {
                                "url": "https://example.com/sodium-0.5.8.jar",
                                "filename": "sodium-0.5.8.jar",
                                "primary": true,
                                "size": 123456
                            }
                        ]
                    }
                ]"#
                .to_string(),
            )
        } else if path.contains("/project/sodium") {
            (
                200,
                r#"{
                    "id": "AANobbMI",
                    "slug": "sodium",
                    "project_type": "mod",
                    "title": "Sodium",
                    "description": "Modern rendering engine",
                    "body": "Detailed description here",
                    "downloads": 10000000,
                    "followers": 50000,
                    "categories": ["optimization"],
                    "game_versions": ["1.20.1"],
                    "loaders": ["fabric"]
                }"#
                .to_string(),
            )
        } else {
            (404, "Not Found".to_string())
        }
    })
    .await;

    let client = ModrinthClient::with_endpoint(Client::new(), &server.base_url);
    let details = client
        .get_details("sodium")
        .await
        .expect("get_details succeeded");
    assert_eq!(details.id, "mr:AANobbMI");
    assert_eq!(details.title, "Sodium");
    assert_eq!(details.versions.len(), 1);
    assert_eq!(details.versions[0].filename, "sodium-0.5.8.jar");
}

#[tokio::test]
async fn test_curseforge_client_search_loopback() {
    let server = MockServer::start(|_method, path| {
        if path.contains("/mods/search") {
            (
                200,
                r#"{
                    "data": [
                        {
                            "id": 238222,
                            "name": "Just Enough Items (JEI)",
                            "slug": "jei",
                            "summary": "View Items and Recipes",
                            "downloadCount": 200000000,
                            "classId": 6
                        }
                    ],
                    "pagination": {
                        "totalCount": 1,
                        "index": 0,
                        "pageSize": 10
                    }
                }"#
                .to_string(),
            )
        } else {
            (404, "Not Found".to_string())
        }
    })
    .await;

    let client = CurseForgeClient::with_endpoint(Client::new(), &server.base_url, "test-api-key");
    let query = ContentSearchQuery {
        project_type: "mod",
        query: Some("jei"),
        game_version: Some("1.20.1"),
        loader: Some("forge"),
        sort: Some("downloads"),
        offset: 0,
        limit: 10,
    };

    let result = client
        .search(&query)
        .await
        .expect("curseforge search succeeded");
    assert_eq!(result.total_hits, 1);
    assert_eq!(result.items.len(), 1);
    let item = &result.items[0];
    assert_eq!(item.id, "cf:238222");
    assert_eq!(item.slug, "jei");
    assert_eq!(item.downloads, 200000000);
}

#[tokio::test]
async fn test_curseforge_client_get_details_loopback() {
    let server = MockServer::start(|_method, path| {
        if path.contains("/description") {
            (200, r#"{ "data": "<p>JEI description</p>" }"#.to_string())
        } else if path.contains("/files") {
            (
                200,
                r#"{
                    "data": [
                        {
                            "id": 99999,
                            "displayName": "jei-1.20.1-15.0.0.jar",
                            "fileName": "jei-1.20.1-15.0.0.jar",
                            "releaseType": 1,
                            "downloadUrl": "https://example.com/jei.jar",
                            "gameVersions": ["1.20.1", "Forge"]
                        }
                    ]
                }"#
                .to_string(),
            )
        } else if path.contains("/mods/238222") {
            (
                200,
                r#"{
                    "data": {
                        "id": 238222,
                        "name": "Just Enough Items (JEI)",
                        "slug": "jei",
                        "summary": "View Items and Recipes",
                        "classId": 6,
                        "downloadCount": 200000000.0,
                        "authors": [{ "name": "mezz" }]
                    }
                }"#
                .to_string(),
            )
        } else {
            (404, "Not Found".to_string())
        }
    })
    .await;

    let client = CurseForgeClient::with_endpoint(Client::new(), &server.base_url, "test-api-key");
    let details = client
        .get_details("238222")
        .await
        .expect("get_details succeeded");
    assert_eq!(details.id, "238222");
    assert_eq!(details.title, "Just Enough Items (JEI)");
    assert_eq!(details.author, "mezz");
    assert_eq!(details.versions.len(), 1);
    assert_eq!(details.versions[0].filename, "jei-1.20.1-15.0.0.jar");
}

#[tokio::test]
async fn test_curseforge_client_get_file_loopback() {
    let server = MockServer::start(|_method, path| {
        if path.contains("/mods/238222/files/12345") {
            (
                200,
                r#"{
                    "data": {
                        "id": 12345,
                        "displayName": "jei-1.20.1.jar",
                        "fileName": "jei-1.20.1.jar",
                        "releaseType": 1,
                        "downloadUrl": "https://example.com/jei.jar"
                    }
                }"#
                .to_string(),
            )
        } else {
            (404, "Not Found".to_string())
        }
    })
    .await;

    let client = CurseForgeClient::with_endpoint(Client::new(), &server.base_url, "test-api-key");
    let file = client
        .get_file(238222, 12345)
        .await
        .expect("get_file succeeded");
    assert_eq!(file.id, 12345);
    assert_eq!(file.file_name, "jei-1.20.1.jar");
}

#[tokio::test]
#[ignore = "requires the public Modrinth API; run explicitly as a live smoke test"]
async fn test_search_modrinth_mods() {
    let res = search_content(
        "modrinth".into(),
        "mod".into(),
        Some("sodium".into()),
        Some("1.20.1".into()),
        Some("fabric".into()),
        Some("downloads".into()),
        0,
        5,
    )
    .await;

    assert!(res.is_ok(), "search_content failed: {:?}", res.err());
    let result = res.unwrap();
    assert!(!result.items.is_empty(), "expected items from Modrinth");
    assert_eq!(result.items[0].source, "modrinth");
}

#[tokio::test]
#[ignore = "requires the public CurseForge API and API key; run explicitly as a live smoke test"]
async fn test_search_curseforge_mods() {
    let res = search_content(
        "curseforge".into(),
        "mod".into(),
        Some("jei".into()),
        Some("1.20.1".into()),
        Some("forge".into()),
        Some("downloads".into()),
        0,
        5,
    )
    .await;

    assert!(res.is_ok(), "CurseForge search failed: {:?}", res.err());
    let result = res.unwrap();
    assert!(!result.items.is_empty(), "expected items from CurseForge");
    assert_eq!(result.items[0].source, "curseforge");
}

#[tokio::test]
#[ignore = "requires the public Modrinth API; run explicitly as a live smoke test"]
async fn test_get_modrinth_details() {
    let res = get_content_details("modrinth", "mr:sodium").await;
    assert!(res.is_ok(), "get_modrinth_details failed: {:?}", res.err());
    let details = res.unwrap();
    assert_eq!(details.slug, "sodium");
    assert!(!details.versions.is_empty(), "expected versions for Sodium");
}

#[tokio::test]
#[ignore = "requires the public CurseForge API and API key; run explicitly as a live smoke test"]
async fn test_search_curseforge_bedrock_addons() {
    let res = search_content(
        "curseforge".into(),
        "addon".into(),
        None,
        None,
        Some("bedrock".into()),
        Some("downloads".into()),
        0,
        5,
    )
    .await;

    assert!(
        res.is_ok(),
        "CurseForge Bedrock search failed: {:?}",
        res.err()
    );
    let result = res.unwrap();
    assert!(
        !result.items.is_empty(),
        "expected items from CurseForge Bedrock"
    );
    assert_eq!(result.items[0].source, "curseforge");
}
