use super::super::{
    atlauncher::compute_atlauncher_safe_name, curseforge::CurseForgeClient,
    install::ftb_pack::FtbVersionManifest, models::ContentSearchQuery, modrinth::ModrinthClient,
    service::ContentSearchRequest, service::ContentService,
    technic::extract_technic_slug_from_query, AtLauncherClient, FtbClient, TechnicClient,
};
use super::MockServer;
use reqwest::Client;

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

    let client = FtbClient::with_endpoint(Client::new(), &server.base_url);

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

    let client = TechnicClient::with_endpoint(Client::new(), &server.base_url);

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

    let client = AtLauncherClient::with_endpoint(Client::new(), &server.base_url);

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

    let service = ContentService::with_all_providers(
        ModrinthClient::with_endpoint(Client::new(), &server.base_url),
        CurseForgeClient::with_endpoint(Client::new(), &server.base_url, "k"),
        FtbClient::with_endpoint(Client::new(), &server.base_url),
        TechnicClient::with_endpoint(Client::new(), &server.base_url),
        AtLauncherClient::with_endpoint(Client::new(), &server.base_url),
    );

    // FTB search and details
    let ftb_sr = service
        .search_content(ContentSearchRequest {
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
        .search_content(ContentSearchRequest {
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
        .search_content(ContentSearchRequest {
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
        .search_content(ContentSearchRequest {
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
        extract_technic_slug_from_query(
            "https://www.technicpack.net/modpack/shivaxi-rlcraft.1530385"
        ),
        Some("shivaxi-rlcraft")
    );
    assert_eq!(
        extract_technic_slug_from_query(
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

    let client = TechnicClient::with_endpoint(Client::new(), &server.base_url);
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

    let client = FtbClient::with_endpoint(Client::new(), &server.base_url);
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
    let parsed: FtbVersionManifest = serde_json::from_value(manifest_json).unwrap();
    assert_eq!(parsed.name.as_deref(), Some("FTB String Manifest"));
    assert_eq!(parsed.files[0].size, Some(99999));
    let cf = parsed.files[0].curseforge.as_ref().unwrap();
    assert_eq!(cf.project, 459929);
    assert_eq!(cf.file, 4013966);
}

#[tokio::test]
async fn test_atlauncher_packsnew_safe_name_computation_and_configs_json() {
    assert_eq!(
        compute_atlauncher_safe_name("All The Forge 10"),
        "AllTheForge10"
    );
    assert_eq!(
        compute_atlauncher_safe_name("Magicality: The Next Dimension"),
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

    let client = AtLauncherClient::with_endpoint(Client::new(), &server.base_url);
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
