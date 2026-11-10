//! Tests for Minecraft content searching, details retrieval, and installation.
use super::{
    curseforge::CurseForgeClient,
    install::{
        construct_curseforge_cdn_url, extract_zip_overrides, parse_curseforge_loader,
        parse_modrinth_loader, target_subfolder_for_project_type, CurseForgeManifestModLoader,
    },
    models::ContentSearchQuery,
    modrinth::ModrinthClient,
    parse_content_source_and_id,
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
    assert_eq!(parse_content_source_and_id("", "ftb:1234"), ("ftb", "1234"));
    assert_eq!(
        parse_content_source_and_id("", "technic:attack-of-the-bteam"),
        ("technic", "attack-of-the-bteam")
    );
    assert_eq!(
        parse_content_source_and_id("", "atlauncher:crucial-journey"),
        ("atlauncher", "crucial-journey")
    );
    assert_eq!(
        parse_content_source_and_id("", "atl:crucial-journey"),
        ("atlauncher", "crucial-journey")
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
    let res = super::service::ContentService::new(Client::new())
        .search_content(crate::minecraft::content::service::ContentSearchRequest {
            source: "modrinth".into(),
            project_type: "mod".into(),
            query: Some("sodium".into()),
            game_version: Some("1.20.1".into()),
            loader: Some("fabric".into()),
            sort: Some("downloads".into()),
            page: 0,
            page_size: 5,
        })
        .await;

    assert!(res.is_ok(), "search_content failed: {:?}", res.err());
    let result = res.unwrap();
    assert!(!result.items.is_empty(), "expected items from Modrinth");
    assert_eq!(result.items[0].source, "modrinth");
}

#[tokio::test]
#[ignore = "requires the public CurseForge API and API key; run explicitly as a live smoke test"]
async fn test_search_curseforge_mods() {
    let res = super::service::ContentService::new(Client::new())
        .search_content(crate::minecraft::content::service::ContentSearchRequest {
            source: "curseforge".into(),
            project_type: "mod".into(),
            query: Some("jei".into()),
            game_version: Some("1.20.1".into()),
            loader: Some("forge".into()),
            sort: Some("downloads".into()),
            page: 0,
            page_size: 5,
        })
        .await;

    assert!(res.is_ok(), "CurseForge search failed: {:?}", res.err());
    let result = res.unwrap();
    assert!(!result.items.is_empty(), "expected items from CurseForge");
    assert_eq!(result.items[0].source, "curseforge");
}

#[tokio::test]
#[ignore = "requires the public Modrinth API; run explicitly as a live smoke test"]
async fn test_get_modrinth_details() {
    let res = super::service::ContentService::new(Client::new())
        .get_content_details("modrinth", "mr:sodium")
        .await;
    assert!(res.is_ok(), "get_modrinth_details failed: {:?}", res.err());
    let details = res.unwrap();
    assert_eq!(details.slug, "sodium");
    assert!(!details.versions.is_empty(), "expected versions for Sodium");
}

#[tokio::test]
#[ignore = "requires the public CurseForge API and API key; run explicitly as a live smoke test"]
async fn test_search_curseforge_bedrock_addons() {
    let res = super::service::ContentService::new(Client::new())
        .search_content(crate::minecraft::content::service::ContentSearchRequest {
            source: "curseforge".into(),
            project_type: "addon".into(),
            query: None,
            game_version: None,
            loader: Some("bedrock".into()),
            sort: Some("downloads".into()),
            page: 0,
            page_size: 5,
        })
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

#[tokio::test]
async fn combined_search_propagates_one_or_both_provider_failures() {
    for (mr_status, cf_status) in [(503, 503), (200, 503), (503, 200), (200, 200)] {
        let server = MockServer::start(move |_, path| {
            if path.starts_with("/search") {
                (
                    mr_status,
                    r#"{"hits":[],"total_hits":0,"offset":0,"limit":10}"#.into(),
                )
            } else {
                (
                    cf_status,
                    r#"{"data":[],"pagination":{"totalCount":0,"index":0,"pageSize":10}}"#.into(),
                )
            }
        })
        .await;
        let service = super::service::ContentService::with_providers(
            ModrinthClient::with_endpoint(Client::new(), &server.base_url),
            CurseForgeClient::with_endpoint(Client::new(), &server.base_url, "fixture-key"),
        );
        let result = service
            .search_content(crate::minecraft::content::service::ContentSearchRequest {
                source: "all".into(),
                project_type: "mod".into(),
                query: None,
                game_version: None,
                loader: None,
                sort: None,
                page: 0,
                page_size: 10,
            })
            .await;
        match (mr_status, cf_status) {
            (503, 503) => assert!(matches!(
                result,
                Err(super::ContentError::ProvidersUnavailable { .. })
            )),
            (200, 503) => assert!(matches!(
                result,
                Err(super::ContentError::Provider {
                    provider: "curseforge",
                    ..
                })
            )),
            (503, 200) => assert!(matches!(
                result,
                Err(super::ContentError::Provider {
                    provider: "modrinth",
                    ..
                })
            )),
            _ => assert!(result.unwrap().items.is_empty()),
        }
    }
}

#[tokio::test]
async fn required_version_responses_cannot_be_replaced_with_empty_versions() {
    for (status, body) in [(503, "unavailable"), (200, "invalid json")] {
        let server = MockServer::start(move |_, path| {
            if path.ends_with("/version") || path.contains("/files") {
                (status, body.into())
            } else if path.starts_with("/project/") {
                (200, r#"{"id":"a","slug":"a","project_type":"mod","title":"A","description":"A","downloads":1}"#.into())
            } else {
                (200, r#"{"data":{"id":1,"name":"A","slug":"a","summary":"A"}}"#.into())
            }
        }).await;
        let mr = ModrinthClient::with_endpoint(Client::new(), &server.base_url);
        let cf = CurseForgeClient::with_endpoint(Client::new(), &server.base_url, "fixture-key");
        for result in [mr.get_details("a").await, cf.get_details("1").await] {
            if status == 503 {
                assert!(matches!(result, Err(super::ContentError::HttpStatus(_))));
            } else {
                assert!(matches!(result, Err(super::ContentError::Response(_))));
            }
        }
    }
}

#[tokio::test]
async fn pagination_overflow_is_rejected_before_provider_requests() {
    let server =
        MockServer::start(|_, _| panic!("invalid pagination must not request providers")).await;
    let service = super::service::ContentService::with_providers(
        ModrinthClient::with_endpoint(Client::new(), &server.base_url),
        CurseForgeClient::with_endpoint(Client::new(), &server.base_url, "fixture-key"),
    );
    let result = service
        .search_content(crate::minecraft::content::service::ContentSearchRequest {
            source: "all".into(),
            project_type: "mod".into(),
            query: None,
            game_version: None,
            loader: None,
            sort: None,
            page: u32::MAX,
            page_size: 2,
        })
        .await;
    assert!(matches!(
        result,
        Err(super::ContentError::InvalidPagination)
    ));
}

#[tokio::test]
async fn test_ftb_client_search_and_details_loopback() {
    let server = MockServer::start(|_method, path| {
        if path.contains("/modpack/search/50") {
            (200, r#"{"status":"success","packs":[101]}"#.to_string())
        } else if path.contains("/modpack/101") {
            (
                200,
                r#"{
                    "id": 101,
                    "name": "FTB Stoneblock 3",
                    "slug": "ftb-stoneblock-3",
                    "synopsis": "Stoneblock is back!",
                    "description": "Full description of Stoneblock 3",
                    "art": [{"type": "square", "url": "https://example.com/sb3.png"}],
                    "authors": [{"name": "FTB Team"}],
                    "installs": 500000,
                    "plays": 1200000,
                    "tags": [{"name": "Quest"}],
                    "targets": [
                        {"name": "minecraft", "type": "game", "version": "1.18.2"},
                        {"name": "forge", "type": "modloader", "version": "40.2.0"}
                    ],
                    "versions": [
                        {
                            "id": 1001,
                            "name": "1.0.0",
                            "type": "release",
                            "updated": 1660000000,
                            "targets": [
                                {"name": "minecraft", "type": "game", "version": "1.18.2"},
                                {"name": "forge", "type": "modloader", "version": "40.2.0"}
                            ]
                        }
                    ]
                }"#
                .to_string(),
            )
        } else {
            (404, "Not Found".to_string())
        }
    })
    .await;

    let client = super::FtbClient::with_endpoint(Client::new(), &server.base_url);

    // Search
    let query = ContentSearchQuery {
        project_type: "modpack",
        query: Some("stoneblock"),
        game_version: Some("1.18.2"),
        loader: Some("forge"),
        sort: None,
        offset: 0,
        limit: 10,
    };
    let search_res = client.search(&query).await.unwrap();
    assert_eq!(search_res.total_hits, 1);
    assert_eq!(search_res.items.len(), 1);
    assert_eq!(search_res.items[0].id, "ftb:101");
    assert_eq!(search_res.items[0].title, "FTB Stoneblock 3");
    assert_eq!(search_res.items[0].source, "ftb");
    assert_eq!(search_res.items[0].downloads, 500000);

    // Details
    let details = client.get_details("101").await.unwrap();
    assert_eq!(details.id, "ftb:101");
    assert_eq!(details.slug, "ftb-stoneblock-3");
    assert_eq!(details.body, "Full description of Stoneblock 3");
    assert_eq!(details.versions.len(), 1);
    assert_eq!(
        details.versions[0].download_url,
        format!("{}/modpack/101/1001", server.base_url)
    );
    assert_eq!(details.versions[0].filename, "ftb-101-1001.json");
}

#[tokio::test]
async fn test_technic_client_search_and_details_loopback() {
    let server = MockServer::start(|_method, path| {
        if path.contains("/search") {
            (
                200,
                r#"{
                    "modpacks": [
                        {
                            "name": "attack-of-the-bteam",
                            "display_name": "Attack of the B-Team",
                            "user": "technic",
                            "description": "Crazy mad science modpack",
                            "downloads": 2000000,
                            "runs": 5000000,
                            "icon": {"url": "https://example.com/bteam.png"}
                        }
                    ]
                }"#
                .to_string(),
            )
        } else if path.contains("/modpack/attack-of-the-bteam") {
            (
                200,
                r#"{
                    "name": "attack-of-the-bteam",
                    "display_name": "Attack of the B-Team",
                    "user": "technic",
                    "description": "Crazy mad science modpack",
                    "overview": "Long overview text",
                    "url": "https://example.com/bteam.zip",
                    "version": "1.0.14",
                    "minecraft": "1.6.4",
                    "downloads": 2000000,
                    "runs": 5000000
                }"#
                .to_string(),
            )
        } else {
            (404, "Not Found".to_string())
        }
    })
    .await;

    let client = super::TechnicClient::with_endpoint(Client::new(), &server.base_url);

    // Keyword search
    let query = ContentSearchQuery {
        project_type: "modpack",
        query: Some("bteam"),
        game_version: None,
        loader: None,
        sort: None,
        offset: 0,
        limit: 10,
    };
    let res = client.search(&query).await.unwrap();
    assert_eq!(res.items.len(), 1);
    assert_eq!(res.items[0].id, "technic:attack-of-the-bteam");
    assert_eq!(res.items[0].source, "technic");

    // Direct URL search
    let url_query = ContentSearchQuery {
        project_type: "modpack",
        query: Some("https://www.technicpack.net/modpack/attack-of-the-bteam"),
        game_version: None,
        loader: None,
        sort: None,
        offset: 0,
        limit: 10,
    };
    let url_res = client.search(&url_query).await.unwrap();
    assert_eq!(url_res.items.len(), 1);
    assert_eq!(url_res.items[0].slug, "attack-of-the-bteam");

    // Details
    let details = client.get_details("attack-of-the-bteam").await.unwrap();
    assert_eq!(details.id, "technic:attack-of-the-bteam");
    assert_eq!(details.versions.len(), 1);
    assert_eq!(
        details.versions[0].download_url,
        "https://example.com/bteam.zip"
    );
    assert_eq!(
        details.versions[0].filename,
        "attack-of-the-bteam-1.0.14-mc1.6.4.zip"
    );
}

#[tokio::test]
async fn test_atlauncher_client_search_and_details_loopback() {
    let server = MockServer::start(|_method, path| {
        if path.contains("/packs/simple") || path.contains("/packs") {
            (
                200,
                r#"[
                    {
                        "id": 1,
                        "name": "Crucial 2",
                        "safeName": "Crucial2",
                        "description": "A vanilla-plus experience",
                        "downloads": 150000,
                        "categories": ["Vanilla+"],
                        "versions": [
                            {
                                "version": "2.1.0",
                                "minecraft": "1.16.5",
                                "published": 1620000000
                            }
                        ]
                    }
                ]"#
                .to_string(),
            )
        } else if path.contains("/pack/Crucial2") {
            (
                200,
                r#"{
                    "id": 1,
                    "name": "Crucial 2",
                    "safeName": "Crucial2",
                    "description": "A vanilla-plus experience",
                    "body": "Detailed info about Crucial 2",
                    "downloads": 150000,
                    "versions": [
                        {
                            "version": "2.1.0",
                            "minecraft": "1.16.5",
                            "loader": "Forge",
                            "published": 1620000000
                        }
                    ]
                }"#
                .to_string(),
            )
        } else {
            (404, "Not Found".to_string())
        }
    })
    .await;

    let client = super::AtLauncherClient::with_endpoint(Client::new(), &server.base_url);

    // Search
    let query = ContentSearchQuery {
        project_type: "modpack",
        query: Some("crucial"),
        game_version: Some("1.16.5"),
        loader: None,
        sort: None,
        offset: 0,
        limit: 10,
    };
    let res = client.search(&query).await.unwrap();
    assert_eq!(res.items.len(), 1);
    assert_eq!(res.items[0].id, "atlauncher:Crucial2");
    assert_eq!(res.items[0].source, "atlauncher");

    // Details
    let details = client.get_details("Crucial2").await.unwrap();
    assert_eq!(details.id, "atlauncher:Crucial2");
    assert_eq!(details.versions.len(), 1);
    assert_eq!(
        details.versions[0].filename,
        "atlauncher-Crucial2-2.1.0.json"
    );
    assert_eq!(
        details.versions[0].download_url,
        format!("{}/pack/Crucial2/version/2.1.0", server.base_url)
    );
}

#[tokio::test]
async fn test_content_service_routing_ftb_technic_atlauncher() {
    let server = MockServer::start(|_method, path| {
        if path.contains("/modpack/search/50") {
            (200, r#"{"packs":[1]}"#.to_string())
        } else if path.contains("/modpack/1") {
            (
                200,
                r#"{"id":1,"name":"FTB Pack","versions":[]}"#.to_string(),
            )
        } else if path.contains("/search") {
            (
                200,
                r#"{"modpacks":[{"name":"technic-pack","display_name":"Technic Pack"}]}"#
                    .to_string(),
            )
        } else if path.contains("/modpack/technic-pack") {
            (
                200,
                r#"{"name":"technic-pack","display_name":"Technic Pack","version":"1.0","url":"https://example.com/technic.zip"}"#
                    .to_string(),
            )
        } else if path.contains("/packs/simple") {
            (
                200,
                r#"[{"name":"ATL Pack","safeName":"ATLPack","versions":[]}]"#.to_string(),
            )
        } else if path.contains("/pack/ATLPack") {
            (
                200,
                r#"{"name":"ATL Pack","safeName":"ATLPack","versions":[]}"#.to_string(),
            )
        } else {
            (404, "Not Found".to_string())
        }
    })
    .await;

    let service = super::service::ContentService::with_all_providers(
        ModrinthClient::with_endpoint(Client::new(), &server.base_url),
        CurseForgeClient::with_endpoint(Client::new(), &server.base_url, "k"),
        super::FtbClient::with_endpoint(Client::new(), &server.base_url),
        super::TechnicClient::with_endpoint(Client::new(), &server.base_url),
        super::AtLauncherClient::with_endpoint(Client::new(), &server.base_url),
    );

    // FTB search and details
    let ftb_sr = service
        .search_content(super::service::ContentSearchRequest {
            source: "ftb".into(),
            project_type: "modpack".into(),
            query: Some("pack".into()),
            game_version: None,
            loader: None,
            sort: None,
            page: 0,
            page_size: 10,
        })
        .await
        .unwrap();
    assert_eq!(ftb_sr.items.len(), 1);
    assert_eq!(ftb_sr.items[0].id, "ftb:1");

    let ftb_det = service.get_content_details("ftb", "1").await.unwrap();
    assert_eq!(ftb_det.id, "ftb:1");

    // Technic search and details
    let tech_sr = service
        .search_content(super::service::ContentSearchRequest {
            source: "technic".into(),
            project_type: "modpack".into(),
            query: Some("pack".into()),
            game_version: None,
            loader: None,
            sort: None,
            page: 0,
            page_size: 10,
        })
        .await
        .unwrap();
    assert_eq!(tech_sr.items.len(), 1);
    assert_eq!(tech_sr.items[0].id, "technic:technic-pack");

    let tech_det = service
        .get_content_details("technic", "technic-pack")
        .await
        .unwrap();
    assert_eq!(tech_det.id, "technic:technic-pack");

    // Technic URL routing when source is "all"
    let tech_url_sr = service
        .search_content(super::service::ContentSearchRequest {
            source: "all".into(),
            project_type: "modpack".into(),
            query: Some("https://www.technicpack.net/modpack/technic-pack".into()),
            game_version: None,
            loader: None,
            sort: None,
            page: 0,
            page_size: 10,
        })
        .await
        .unwrap();
    assert_eq!(tech_url_sr.items.len(), 1);
    assert_eq!(tech_url_sr.items[0].id, "technic:technic-pack");

    // ATLauncher search and details
    let atl_sr = service
        .search_content(super::service::ContentSearchRequest {
            source: "atlauncher".into(),
            project_type: "modpack".into(),
            query: Some("ATL".into()),
            game_version: None,
            loader: None,
            sort: None,
            page: 0,
            page_size: 10,
        })
        .await
        .unwrap();
    assert_eq!(atl_sr.items.len(), 1);
    assert_eq!(atl_sr.items[0].id, "atlauncher:ATLPack");

    let atl_det = service.get_content_details("atl", "ATLPack").await.unwrap();
    assert_eq!(atl_det.id, "atlauncher:ATLPack");
}

#[tokio::test]
async fn test_technic_search_slug_and_camelcase_fields_and_slug_extraction() {
    assert_eq!(
        super::technic::extract_technic_slug_from_query(
            "https://www.technicpack.net/modpack/shivaxi-rlcraft.1530385"
        ),
        Some("shivaxi-rlcraft")
    );
    assert_eq!(
        super::technic::extract_technic_slug_from_query(
            "https://www.technicpack.net/modpack/nanatsu-dark-servidor.1389259?build=ingot"
        ),
        Some("nanatsu-dark-servidor")
    );

    let server = MockServer::start(|_method, path| {
        if path.contains("/trending") {
            (
                200,
                r#"{
                    "modpacks": [
                        {
                            "id": "1530385",
                            "name": "RLCraft",
                            "slug": "shivaxi-rlcraft",
                            "url": "https://www.technicpack.net/modpack/shivaxi-rlcraft.1530385",
                            "iconUrl": "https://cdn.technicpack.net/platform2/pack-icons/1530385.png"
                        }
                    ]
                }"#
                .to_string(),
            )
        } else if path.contains("/modpack/shivaxi-rlcraft") {
            (
                200,
                r#"{
                    "id": 1530385,
                    "name": "shivaxi-rlcraft",
                    "displayName": "RLCraft",
                    "user": "Shivaxi",
                    "description": "Real life craft modpack",
                    "minecraft": "1.12.2",
                    "installs": 2500000,
                    "runs": 6000000,
                    "version": "2.9.3",
                    "url": "https://example.com/rlcraft.zip"
                }"#
                .to_string(),
            )
        } else {
            (404, "Not Found".to_string())
        }
    })
    .await;

    let client = super::TechnicClient::with_endpoint(Client::new(), &server.base_url);
    let query = ContentSearchQuery {
        project_type: "modpack",
        query: None,
        game_version: None,
        loader: None,
        sort: None,
        offset: 0,
        limit: 10,
    };
    let res = client.search(&query).await.unwrap();
    assert_eq!(res.items.len(), 1);
    let item = &res.items[0];
    assert_eq!(item.id, "technic:shivaxi-rlcraft");
    assert_eq!(item.slug, "shivaxi-rlcraft");
    assert_eq!(item.title, "RLCraft");
    assert_eq!(item.author, "Shivaxi");
    assert_eq!(item.downloads, 2500000);
    assert_eq!(item.latest_version, Some("2.9.3".to_string()));
    assert_eq!(item.game_versions, vec!["1.12.2".to_string()]);
    assert_eq!(
        item.icon_url,
        Some("https://cdn.technicpack.net/platform2/pack-icons/1530385.png".to_string())
    );

    let details = client.get_details("shivaxi-rlcraft").await.unwrap();
    assert_eq!(details.id, "technic:shivaxi-rlcraft");
    assert_eq!(details.versions.len(), 1);
    assert_eq!(
        details.versions[0].filename,
        "shivaxi-rlcraft-2.9.3-mc1.12.2.zip"
    );
}

#[tokio::test]
async fn test_ftb_string_number_deserialization_and_concurrent_hydration() {
    let server = MockServer::start(|_method, path| {
        if path.contains("/popular/installs/50") {
            (200, r#"{"packs":["1001","1002"],"total":2}"#.to_string())
        } else if path.contains("/modpack/1001") {
            (
                200,
                r#"{
                    "id": "1001",
                    "name": "FTB String IDs Pack",
                    "synopsis": "Pack with string numbers",
                    "installs": "45000",
                    "plays": "120000",
                    "versions": [
                        {
                            "id": "2001",
                            "name": "1.0.0",
                            "updated": "1670000000",
                            "targets": [
                                {"name": "minecraft", "type": "game", "version": "1.20.1"},
                                {"name": "forge", "type": "modloader", "version": "47.2.0"}
                            ]
                        }
                    ]
                }"#
                .to_string(),
            )
        } else if path.contains("/modpack/1002") {
            (
                200,
                r#"{
                    "id": 1002,
                    "name": "FTB Numeric IDs Pack",
                    "synopsis": "Pack with numeric numbers",
                    "installs": 10000,
                    "plays": 25000,
                    "versions": []
                }"#
                .to_string(),
            )
        } else {
            (404, "Not Found".to_string())
        }
    })
    .await;

    let client = super::FtbClient::with_endpoint(Client::new(), &server.base_url);
    let query = ContentSearchQuery {
        project_type: "modpack",
        query: None,
        game_version: None,
        loader: None,
        sort: None,
        offset: 0,
        limit: 10,
    };
    let res = client.search(&query).await.unwrap();
    assert_eq!(res.items.len(), 2);
    assert_eq!(res.items[0].id, "ftb:1001");
    assert_eq!(res.items[0].downloads, 45000);
    assert_eq!(res.items[1].id, "ftb:1002");
    assert_eq!(res.items[1].downloads, 10000);

    let manifest_json = serde_json::json!({
        "id": "1001",
        "name": "FTB String Manifest",
        "targets": [
            {"name": "minecraft", "type": "game", "version": "1.20.1"}
        ],
        "files": [
            {
                "id": "5001",
                "name": "cf-mod.jar",
                "size": "99999",
                "curseforge": {
                    "project": "459929",
                    "file": "4013966"
                }
            }
        ]
    });
    let parsed: super::install::ftb_pack::FtbVersionManifest =
        serde_json::from_value(manifest_json).unwrap();
    assert_eq!(parsed.id, Some(1001));
    assert_eq!(parsed.files[0].id, Some(5001));
    assert_eq!(parsed.files[0].size, Some(99999));
    let cf = parsed.files[0].curseforge.as_ref().unwrap();
    assert_eq!(cf.project, 459929);
    assert_eq!(cf.file, 4013966);
}

#[tokio::test]
async fn test_atlauncher_packsnew_safe_name_computation_and_configs_json() {
    assert_eq!(
        super::atlauncher::compute_atlauncher_safe_name("All The Forge 10"),
        "AllTheForge10"
    );
    assert_eq!(
        super::atlauncher::compute_atlauncher_safe_name("Magicality: The Next Dimension"),
        "MagicalityTheNextDimension"
    );

    let server = MockServer::start(|_method, path| {
        if path.contains("/launcher/json/packsnew.json") {
            (
                200,
                r#"[
                    {
                        "id": 597,
                        "position": 2,
                        "name": "All The Forge 10",
                        "type": "public",
                        "versions": [
                            {
                                "version": "v11.3.0",
                                "minecraft": "1.21.1",
                                "hasLoader": true
                            }
                        ],
                        "description": "A big forge pack",
                        "supportURL": "https://github.com/example/support"
                    },
                    {
                        "id": 999,
                        "position": 1,
                        "name": "Top Ranked Pack",
                        "type": "public",
                        "versions": [
                            {
                                "version": "1.0.0",
                                "minecraft": "1.20.1"
                            }
                        ],
                        "description": "Top pack"
                    },
                    {
                        "id": 888,
                        "position": 3,
                        "name": "Private Hidden Pack",
                        "type": "private",
                        "versions": [
                            {
                                "version": "1.0.0",
                                "minecraft": "1.20.1"
                            }
                        ]
                    }
                ]"#
                .to_string(),
            )
        } else {
            (404, "Not Found".to_string())
        }
    })
    .await;

    let client = super::AtLauncherClient::with_endpoint(Client::new(), &server.base_url);
    let query = ContentSearchQuery {
        project_type: "modpack",
        query: None,
        game_version: None,
        loader: None,
        sort: Some("downloads"),
        offset: 0,
        limit: 10,
    };
    let res = client.search(&query).await.unwrap();
    // Private pack must be filtered out
    assert_eq!(res.items.len(), 2);
    // Position 1 should come before Position 2 when downloads is None
    assert_eq!(res.items[0].title, "Top Ranked Pack");
    assert_eq!(res.items[1].title, "All The Forge 10");
    assert_eq!(res.items[1].id, "atlauncher:AllTheForge10");
    assert_eq!(res.items[1].slug, "AllTheForge10");
    assert_eq!(
        res.items[1].icon_url,
        Some(format!(
            "{}/launcher/images/alltheforge10.png",
            server.base_url
        ))
    );

    // Get details via fallback to packsnew.json (since /pack/AllTheForge10 returns 404)
    let details = client.get_details("AllTheForge10").await.unwrap();
    assert_eq!(details.id, "atlauncher:AllTheForge10");
    assert_eq!(details.title, "All The Forge 10");
    assert_eq!(details.versions.len(), 1);
    assert_eq!(
        details.versions[0].download_url,
        format!(
            "{}/packs/AllTheForge10/versions/v11.3.0/Configs.json",
            server.base_url
        )
    );
    assert_eq!(
        details.versions[0].filename,
        "atlauncher-AllTheForge10-v11.3.0.json"
    );
    assert_eq!(
        details.issues_url,
        Some("https://github.com/example/support".to_string())
    );
}
