use super::providers::{
    parse_forge_promotions, parse_neoforge_versions, parse_quilt_game_versions,
    parse_quilt_installers, InstallerEndpoints, QuiltGameVersion,
};
use super::selection::{
    forge_install_plan, forge_supported, neoforge_game_version, neoforge_install_plan, pick_forge,
    pick_neoforge, quilt_install_plan, select_forge_game_versions, select_neoforge_game_versions,
    select_quilt_game_versions, version_key,
};
use super::*;
use ingot_test_support::{LoopbackServer, ResponseSpec, ScriptedExchange};
use std::collections::BTreeMap;
use std::io;

#[test]
fn maps_neoforge_versions() {
    assert_eq!(neoforge_game_version("21.1.77").as_deref(), Some("1.21.1"));
    assert_eq!(neoforge_game_version("21.0.167").as_deref(), Some("1.21"));
    assert_eq!(
        neoforge_game_version("20.2.3-beta").as_deref(),
        Some("1.20.2")
    );
    assert_eq!(
        neoforge_game_version("26.1.2.111").as_deref(),
        Some("26.1.2")
    );
    assert_eq!(
        neoforge_game_version("26.3.0.23-beta").as_deref(),
        Some("26.3")
    );
    assert_eq!(neoforge_game_version("26.1.0.0-alpha.7+snapshot-3"), None);
    assert_eq!(neoforge_game_version("0.25w14craftmine.3-beta"), None);
}

#[test]
fn picks_newest_stable_neoforge_then_beta() {
    let versions: Vec<String> = [
        "21.1.9",
        "21.1.100",
        "21.1.99",
        "21.1.101-beta",
        "26.3.0.5-beta",
        "26.3.0.23-beta",
    ]
    .map(str::to_string)
    .into();
    assert_eq!(
        pick_neoforge(&versions, "1.21.1").as_deref(),
        Some("21.1.100")
    );
    assert_eq!(
        pick_neoforge(&versions, "26.3").as_deref(),
        Some("26.3.0.23-beta")
    );
    assert_eq!(pick_neoforge(&versions, "1.20.4"), None);
}

#[test]
fn picks_recommended_forge_then_latest() {
    let promos = BTreeMap::from([
        ("1.20.1-latest".to_string(), "47.4.12".to_string()),
        ("1.20.1-recommended".to_string(), "47.4.10".to_string()),
        ("26.3-latest".to_string(), "66.0.5".to_string()),
    ]);
    assert_eq!(
        pick_forge(&promos, "1.20.1").as_deref(),
        Some("1.20.1-47.4.10")
    );
    assert_eq!(pick_forge(&promos, "26.3").as_deref(), Some("26.3-66.0.5"));
    assert_eq!(pick_forge(&promos, "1.19.2"), None);
    assert!(forge_supported("1.17.1") && forge_supported("26.1") && !forge_supported("1.16.5"));
}

#[test]
fn finds_what_to_launch() {
    let fixture = tempfile::tempdir().unwrap();
    let dir = fixture.path().to_path_buf();
    let name = if cfg!(windows) {
        "win_args.txt"
    } else {
        "unix_args.txt"
    };
    assert_eq!(launch_args(&dir, &ServerCoreType::NeoForge).unwrap(), None);
    for v in ["21.1.99", "21.1.100"] {
        let d = dir.join("libraries/net/neoforged/neoforge").join(v);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(name), "").unwrap();
    }
    let expected = format!("@libraries/net/neoforged/neoforge/21.1.100/{name}");
    assert_eq!(
        launch_args(&dir, &ServerCoreType::NeoForge).unwrap(),
        Some(vec![expected])
    );
    assert_eq!(launch_args(&dir, &ServerCoreType::Forge).unwrap(), None);
    uninstall(&dir, &ServerCoreType::NeoForge).unwrap();
    assert_eq!(launch_args(&dir, &ServerCoreType::NeoForge).unwrap(), None);

    std::fs::write(dir.join(QUILT_LAUNCHER), "").unwrap();
    assert_eq!(
        launch_args(&dir, &ServerCoreType::Quilt).unwrap(),
        None,
        "needs Minecraft's server.jar too"
    );
    std::fs::write(dir.join("server.jar"), "").unwrap();
    assert!(launch_args(&dir, &ServerCoreType::Quilt).unwrap().is_some());
    uninstall(&dir, &ServerCoreType::Quilt).unwrap();
    assert_eq!(launch_args(&dir, &ServerCoreType::Quilt).unwrap(), None);
}

#[test]
fn launch_args_distinguishes_not_installed_from_inspection_failure() {
    let fixture = tempfile::tempdir().unwrap();
    let dir = fixture.path();

    // 1. Legitimate absence: missing directory produces Ok(None)
    let absent = launch_args(dir, &ServerCoreType::NeoForge).unwrap();
    assert_eq!(absent, None);

    let quilt_absent = launch_args(dir, &ServerCoreType::Quilt).unwrap();
    assert_eq!(quilt_absent, None);

    // 2. Injected directory read failure produces typed Err(InstallerError::Io)
    let failed_read = launch_args_with(
        dir,
        &ServerCoreType::NeoForge,
        |_| {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "injected read_dir denial",
            ))
        },
        |_| Ok(dir.metadata().unwrap()),
    );
    assert!(matches!(
        failed_read,
        Err(InstallerError::Io(ref e)) if e.kind() == io::ErrorKind::PermissionDenied
    ));

    // 3. Injected directory entry inspection failure produces typed Err(InstallerError::Io)
    let failed_entry = launch_args_with(
        dir,
        &ServerCoreType::NeoForge,
        |_| Ok(vec![Err(io::Error::other("entry failure"))]),
        |_| Ok(dir.metadata().unwrap()),
    );
    assert!(matches!(
        failed_entry,
        Err(InstallerError::Io(ref e)) if e.to_string().contains("entry failure")
    ));

    // 4. Injected Quilt metadata inspection failure produces typed Err(InstallerError::Io)
    let failed_quilt_meta = launch_args_with(
        dir,
        &ServerCoreType::Quilt,
        |_| Ok(Vec::new()),
        |_| {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "unreadable Quilt launcher",
            ))
        },
    );
    assert!(matches!(
        failed_quilt_meta,
        Err(InstallerError::Io(ref e)) if e.kind() == io::ErrorKind::PermissionDenied
    ));
}

#[derive(Debug, thiserror::Error)]
#[error("injected command prep error: {msg}")]
struct InjectedCommandError {
    msg: String,
}

#[tokio::test]
async fn installer_command_preparation_failure_retains_source() {
    let fixture = tempfile::tempdir().unwrap();
    let installer = "test-installer.jar";
    std::fs::write(fixture.path().join(installer), b"downloaded").unwrap();
    let error = execution::run_downloaded_installer(
        fixture.path(),
        &ServerCoreType::NeoForge,
        installer,
        vec!["--installServer".into()],
        |arguments| -> Result<tokio::process::Command, InjectedCommandError> {
            assert_eq!(arguments, ["-jar", installer, "--installServer"]);
            Err(InjectedCommandError {
                msg: "custom command failure".into(),
            })
        },
    )
    .await
    .unwrap_err();
    let InstallerError::Process { source, .. } = error else {
        panic!("expected command preparation error");
    };
    assert_eq!(
        source.downcast_ref::<InjectedCommandError>().unwrap().msg,
        "custom command failure"
    );
    assert!(!fixture.path().join(installer).exists());
}

#[tokio::test]
async fn installer_spawn_failure_retains_source() {
    let fixture = tempfile::tempdir().unwrap();
    let missing = fixture.path().join("missing-java-executable");
    let error = execution::run_downloaded_installer(
        fixture.path(),
        &ServerCoreType::Forge,
        "installer.jar",
        Vec::new(),
        |_| -> Result<Command, io::Error> { Ok(Command::new(&missing)) },
    )
    .await
    .unwrap_err();
    let InstallerError::Process { source, .. } = error else {
        panic!("expected spawn error");
    };
    assert_eq!(
        source.downcast_ref::<io::Error>().unwrap().kind(),
        io::ErrorKind::NotFound
    );
}
#[test]
fn failure_leaves_installation_and_recovery_material_unchanged() {
    let fixture = tempfile::tempdir().unwrap();
    let dir = fixture.path();

    // Put existing server files in place
    let props = dir.join("server.properties");
    std::fs::write(&props, b"motd=existing server\n").unwrap();
    let world_marker = dir.join("world/level.dat");
    std::fs::create_dir_all(world_marker.parent().unwrap()).unwrap();
    std::fs::write(&world_marker, b"world data").unwrap();

    // Trigger an inspection failure
    let result = launch_args_with(
        dir,
        &ServerCoreType::NeoForge,
        |_| {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "unreadable",
            ))
        },
        |_| Ok(props.metadata().unwrap()),
    );
    assert!(result.is_err());

    // Pre-existing files and structure remain intact
    assert_eq!(std::fs::read(&props).unwrap(), b"motd=existing server\n");
    assert_eq!(std::fs::read(&world_marker).unwrap(), b"world data");
}

fn loopback_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap()
}

#[test]
fn parses_neoforge_metadata_xml_and_ignores_unclosed_or_empty_tags() {
    let xml = r#"
        <metadata>
          <versioning>
            <versions>
              <version>21.1.77</version>
              <version>   </version>
              <version> 21.1.100 </version>
              <version>26.3.0.23-beta</version>
              <version>21.1.999
            </versions>
          </versioning>
        </metadata>
    "#;
    assert_eq!(
        parse_neoforge_versions(xml),
        vec![
            "21.1.77".to_string(),
            "21.1.100".to_string(),
            "26.3.0.23-beta".to_string(),
        ]
    );
}

#[test]
fn select_game_versions_filters_and_sorts_newest_first() {
    assert!(version_key("21.1.100") > version_key("21.1.99"));

    let neoforge_versions: Vec<String> = [
        "20.2.3-beta",
        "21.0.167",
        "21.1.77",
        "21.1.100",
        "26.1.2.111",
        "26.3.0.23-beta",
        "26.1.0.0-alpha.7+snapshot-3",
        "0.25w14craftmine.3-beta",
    ]
    .map(str::to_string)
    .into();
    assert_eq!(
        select_neoforge_game_versions(&neoforge_versions),
        vec!["26.3", "26.1.2", "1.21.1", "1.21", "1.20.2"]
    );

    let forge_promos = BTreeMap::from([
        ("1.16.5-latest".to_string(), "36.2.42".to_string()),
        ("1.17.1-latest".to_string(), "37.1.1".to_string()),
        ("1.20.1-latest".to_string(), "47.4.12".to_string()),
        ("1.20.1-recommended".to_string(), "47.4.10".to_string()),
        ("26.3-latest".to_string(), "66.0.5".to_string()),
    ]);
    assert_eq!(
        select_forge_game_versions(&forge_promos),
        vec!["26.3", "1.20.1", "1.17.1"]
    );

    let quilt_versions = vec![
        QuiltGameVersion {
            version: "1.20.6".to_string(),
            stable: true,
        },
        QuiltGameVersion {
            version: "1.21.2-rc1".to_string(),
            stable: false,
        },
        QuiltGameVersion {
            version: "1.21.1".to_string(),
            stable: true,
        },
        QuiltGameVersion {
            version: "1.21.1".to_string(),
            stable: true,
        },
    ];
    assert_eq!(
        select_quilt_game_versions(&quilt_versions),
        vec!["1.21.1", "1.20.6"]
    );
}

#[test]
fn builds_installer_download_plans() {
    let endpoints = InstallerEndpoints::from_base("http://127.0.0.1:4100");
    let neoforge = neoforge_install_plan(&endpoints, "21.1.100");
    assert_eq!(
        neoforge.url,
        "http://127.0.0.1:4100/neoforge/21.1.100/neoforge-21.1.100-installer.jar"
    );
    assert_eq!(neoforge.installer_name, "neoforge-installer.jar");
    assert_eq!(neoforge.args, ["--installServer"]);

    let forge = forge_install_plan(&endpoints, "1.20.1-47.4.10");
    assert_eq!(
        forge.url,
        "http://127.0.0.1:4100/forge/1.20.1-47.4.10/forge-1.20.1-47.4.10-installer.jar"
    );
    assert_eq!(forge.installer_name, "forge-installer.jar");
    assert_eq!(forge.args, ["--installServer"]);

    let quilt_unpinned = quilt_install_plan(
        "http://127.0.0.1:4100/quilt/installer.jar".to_string(),
        "1.21.1",
        None,
    );
    assert_eq!(
        quilt_unpinned.url,
        "http://127.0.0.1:4100/quilt/installer.jar"
    );
    assert_eq!(quilt_unpinned.installer_name, "quilt-installer.jar");
    assert_eq!(
        quilt_unpinned.args,
        [
            "install",
            "server",
            "1.21.1",
            "--download-server",
            "--install-dir=."
        ]
    );

    let quilt_pinned = quilt_install_plan(
        "http://127.0.0.1:4100/quilt/installer.jar".to_string(),
        "1.21.1",
        Some("0.26.4"),
    );
    assert_eq!(
        quilt_pinned.args,
        [
            "install",
            "server",
            "1.21.1",
            "0.26.4",
            "--download-server",
            "--install-dir=."
        ]
    );
}

#[test]
fn launch_args_uses_platform_specific_argument_file_name() {
    let fixture = tempfile::tempdir().unwrap();
    let dir = fixture.path();
    let (active_name, inactive_name) = if cfg!(windows) {
        ("win_args.txt", "unix_args.txt")
    } else {
        ("unix_args.txt", "win_args.txt")
    };

    let active_dir = dir.join("libraries/net/neoforged/neoforge/21.1.99");
    let inactive_dir = dir.join("libraries/net/neoforged/neoforge/21.1.100");
    std::fs::create_dir_all(&active_dir).unwrap();
    std::fs::create_dir_all(&inactive_dir).unwrap();
    std::fs::write(active_dir.join(active_name), "").unwrap();
    std::fs::write(inactive_dir.join(inactive_name), "").unwrap();

    let expected = format!("@libraries/net/neoforged/neoforge/21.1.99/{active_name}");
    assert_eq!(
        launch_args(dir, &ServerCoreType::NeoForge).unwrap(),
        Some(vec![expected])
    );
}

#[test]
fn rejects_malformed_forge_promotions_payloads_as_json_error() {
    assert!(parse_forge_promotions(br#"{"promos":{}}"#)
        .unwrap()
        .is_empty());
    assert_eq!(
        parse_forge_promotions(br#"{"promos":{"1.20.1-latest":"47.4.12"}}"#)
            .unwrap()
            .get("1.20.1-latest")
            .map(String::as_str),
        Some("47.4.12")
    );

    let malformed_cases: &[&[u8]] = &[
        br#"{"promos":"#,
        br#"{"homepage":"https://files.minecraftforge.net"}"#,
        br#"{"promos":[]}"#,
        br#"{"promos":{"1.20.1-recommended":123}}"#,
        br#"{"promos":{"1.20.1-latest":"   "}}"#,
        br#"{"promos":{"":"47.4.10"}}"#,
    ];
    for payload in malformed_cases {
        let err = parse_forge_promotions(payload).unwrap_err();
        assert!(
            matches!(err, InstallerError::Json(_)),
            "expected InstallerError::Json for payload {:?}, got {err:?}",
            String::from_utf8_lossy(payload)
        );
    }
}

#[test]
fn rejects_malformed_quilt_game_versions_as_json_error() {
    let valid = parse_quilt_game_versions(br#"[{"version":"1.21.1","stable":true}]"#).unwrap();
    assert_eq!(valid.len(), 1);
    assert_eq!(valid[0].version, "1.21.1");
    assert!(valid[0].stable);

    let malformed_cases: &[&[u8]] = &[
        br#"[{"version":"#,
        br#"{"versions":[]}"#,
        br#"[{"version":"1.21.1"}]"#,
        br#"[{"stable":true}]"#,
        br#"[{"version":"1.21.1","stable":"true"}]"#,
        br#"[{"version":"   ","stable":true}]"#,
    ];
    for payload in malformed_cases {
        let err = parse_quilt_game_versions(payload).unwrap_err();
        assert!(
            matches!(err, InstallerError::Json(_)),
            "expected InstallerError::Json for payload {:?}, got {err:?}",
            String::from_utf8_lossy(payload)
        );
    }
}

#[test]
fn rejects_malformed_quilt_installers_as_json_error_and_empty_list_as_empty() {
    assert!(parse_quilt_installers(br#"[]"#).unwrap().is_empty());
    let valid =
        parse_quilt_installers(br#"[{"url":"https://maven.quiltmc.org/installer.jar"}]"#).unwrap();
    assert_eq!(valid.len(), 1);
    assert_eq!(valid[0].url, "https://maven.quiltmc.org/installer.jar");

    let malformed_cases: &[&[u8]] = &[
        br#"not json"#,
        br#"[{"version":"0.12.1"}]"#,
        br#"[{"url":42}]"#,
        br#"[{"url":"   "}]"#,
    ];
    for payload in malformed_cases {
        let err = parse_quilt_installers(payload).unwrap_err();
        assert!(
            matches!(err, InstallerError::Json(_)),
            "expected InstallerError::Json for payload {:?}, got {err:?}",
            String::from_utf8_lossy(payload)
        );
    }
}

#[tokio::test]
async fn fetch_game_versions_queries_injected_endpoints_and_parses_all_loaders() {
    let neoforge_xml = r#"
        <metadata><versioning><versions>
            <version>20.2.3-beta</version>
            <version>21.1.100</version>
            <version>26.1.0.0-alpha.7+snapshot-3</version>
            <version>26.3.0.23-beta</version>
        </versions></versioning></metadata>
    "#;
    let forge_json =
        br#"{"promos":{"1.16.5-latest":"36.2.42","1.20.1-latest":"47.4.12","26.3-recommended":"66.0.5"}}"#;
    let quilt_json = br#"[
        {"version":"1.20.6","stable":true},
        {"version":"1.21.2-rc1","stable":false},
        {"version":"1.21.1","stable":true}
    ]"#;

    let server = LoopbackServer::exact_script(vec![
        ScriptedExchange::expect_path(
            "/neoforge/maven-metadata.xml",
            ResponseSpec::bytes(200, neoforge_xml.as_bytes().to_vec()),
        ),
        ScriptedExchange::expect_path(
            "/forge/promotions_slim.json",
            ResponseSpec::json(200, forge_json.to_vec()),
        ),
        ScriptedExchange::expect_path(
            "/quilt/v3/versions/game",
            ResponseSpec::json(200, quilt_json.to_vec()),
        ),
    ])
    .await;
    let endpoints = InstallerEndpoints::from_base(server.base_url());
    let client = loopback_client();

    assert_eq!(
        fetch_game_versions_with_endpoints(&client, &ServerCoreType::NeoForge, &endpoints)
            .await
            .unwrap(),
        vec!["26.3", "1.21.1", "1.20.2"]
    );
    assert_eq!(
        fetch_game_versions_with_endpoints(&client, &ServerCoreType::Forge, &endpoints)
            .await
            .unwrap(),
        vec!["26.3", "1.20.1"]
    );
    assert_eq!(
        fetch_game_versions_with_endpoints(&client, &ServerCoreType::Quilt, &endpoints)
            .await
            .unwrap(),
        vec!["1.21.1", "1.20.6"]
    );

    let captured = server.finish().await;
    assert_eq!(captured.len(), 3);
}

#[tokio::test]
async fn fetch_game_versions_short_circuits_non_installer_core_without_http() {
    let server = LoopbackServer::with_handler(|_| ResponseSpec::status(500)).await;
    let endpoints = InstallerEndpoints::from_base(server.base_url());
    let client = loopback_client();

    let err = fetch_game_versions_with_endpoints(&client, &ServerCoreType::Paper, &endpoints)
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        InstallerError::UnsupportedCore(ServerCoreType::Paper)
    ));
    assert!(server.finish().await.is_empty());
}

#[tokio::test]
async fn fetch_game_versions_maps_http_500_to_network_and_malformed_200_to_json() {
    let server = LoopbackServer::exact_script(vec![
        ScriptedExchange::expect_path("/forge/promotions_slim.json", ResponseSpec::status(500)),
        ScriptedExchange::expect_path(
            "/forge/promotions_slim.json",
            ResponseSpec::json(
                200,
                br#"{"homepage":"https://files.minecraftforge.net"}"#.to_vec(),
            ),
        ),
        ScriptedExchange::expect_path(
            "/quilt/v3/versions/game",
            ResponseSpec::json(200, br#"[{"stable":true}]"#.to_vec()),
        ),
    ])
    .await;
    let endpoints = InstallerEndpoints::from_base(server.base_url());
    let client = loopback_client();

    let http_err = fetch_game_versions_with_endpoints(&client, &ServerCoreType::Forge, &endpoints)
        .await
        .unwrap_err();
    assert!(matches!(http_err, InstallerError::Network(_)));

    let forge_json_err =
        fetch_game_versions_with_endpoints(&client, &ServerCoreType::Forge, &endpoints)
            .await
            .unwrap_err();
    assert!(matches!(forge_json_err, InstallerError::Json(_)));

    let quilt_json_err =
        fetch_game_versions_with_endpoints(&client, &ServerCoreType::Quilt, &endpoints)
            .await
            .unwrap_err();
    assert!(matches!(quilt_json_err, InstallerError::Json(_)));

    let captured = server.finish().await;
    assert_eq!(captured.len(), 3);
}

#[tokio::test]
async fn ensure_installed_short_circuits_existing_install_and_unsupported_forge_without_http() {
    let fixture = tempfile::tempdir().unwrap();
    let dir = fixture.path();
    let props = dir.join("server.properties");
    std::fs::write(&props, b"motd=kept\n").unwrap();

    let server = LoopbackServer::with_handler(|_| ResponseSpec::status(500)).await;
    let endpoints = InstallerEndpoints::from_base(server.base_url());
    let client = loopback_client();

    let forge_err = ensure_installed_with_endpoints(
        &client,
        dir,
        &ServerCoreType::Forge,
        "1.16.5",
        None,
        &endpoints,
        |_| -> Result<Command, InjectedCommandError> {
            panic!("java must not run for unsupported Forge")
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(
        forge_err,
        InstallerError::ForgeVersionUnsupported(ref v) if v == "1.16.5"
    ));

    let args_name = if cfg!(windows) {
        "win_args.txt"
    } else {
        "unix_args.txt"
    };
    let installed_dir = dir.join("libraries/net/neoforged/neoforge/21.1.100");
    std::fs::create_dir_all(&installed_dir).unwrap();
    std::fs::write(installed_dir.join(args_name), "").unwrap();

    let launch = ensure_installed_with_endpoints(
        &client,
        dir,
        &ServerCoreType::NeoForge,
        "1.21.1",
        None,
        &endpoints,
        |_| -> Result<Command, InjectedCommandError> {
            panic!("java must not run when already installed")
        },
    )
    .await
    .unwrap();
    assert_eq!(
        launch,
        vec![format!(
            "@libraries/net/neoforged/neoforge/21.1.100/{args_name}"
        )]
    );

    assert!(server.finish().await.is_empty());
    assert_eq!(std::fs::read(&props).unwrap(), b"motd=kept\n");
}

#[tokio::test]
async fn ensure_installed_resolves_neoforge_stable_download_and_cleans_up_on_java_error() {
    let fixture = tempfile::tempdir().unwrap();
    let dir = fixture.path();
    let props = dir.join("server.properties");
    std::fs::write(&props, b"motd=existing\n").unwrap();

    let server = LoopbackServer::with_handler(|req| match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/neoforge/maven-metadata.xml") => ResponseSpec::bytes(
            200,
            br#"<metadata><versioning><versions><version>21.1.99</version><version>21.1.100</version><version>21.1.101-beta</version></versions></versioning></metadata>"#
                .to_vec(),
        ),
        ("HEAD", "/neoforge/21.1.100/neoforge-21.1.100-installer.jar") => ResponseSpec::status(200),
        ("GET", "/neoforge/21.1.100/neoforge-21.1.100-installer.jar") => {
            ResponseSpec::bytes(200, b"fake-neoforge-jar".to_vec())
        }
        _ => ResponseSpec::status(404),
    })
    .await;
    let endpoints = InstallerEndpoints::from_base(server.base_url());
    let client = loopback_client();

    let err = ensure_installed_with_endpoints(
        &client,
        dir,
        &ServerCoreType::NeoForge,
        "1.21.1",
        None,
        &endpoints,
        |arguments| -> Result<Command, InjectedCommandError> {
            assert_eq!(
                arguments,
                ["-jar", "neoforge-installer.jar", "--installServer"]
            );
            Err(InjectedCommandError {
                msg: "stop after download".into(),
            })
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(
        err,
        InstallerError::Process {
            core: ServerCoreType::NeoForge,
            ..
        }
    ));
    assert!(!dir.join("neoforge-installer.jar").exists());
    assert_eq!(std::fs::read(&props).unwrap(), b"motd=existing\n");

    let captured = server.finish().await;
    let trace: Vec<(&str, &str)> = captured
        .iter()
        .map(|req| (req.method.as_str(), req.path.as_str()))
        .collect();
    assert_eq!(
        trace,
        vec![
            ("GET", "/neoforge/maven-metadata.xml"),
            ("HEAD", "/neoforge/21.1.100/neoforge-21.1.100-installer.jar"),
            ("GET", "/neoforge/21.1.100/neoforge-21.1.100-installer.jar"),
        ]
    );
}

#[tokio::test]
async fn ensure_installed_skips_metadata_when_neoforge_is_pinned() {
    let fixture = tempfile::tempdir().unwrap();
    let dir = fixture.path();

    let server =
        LoopbackServer::with_handler(|req| match (req.method.as_str(), req.path.as_str()) {
            ("HEAD", "/neoforge/21.1.77/neoforge-21.1.77-installer.jar") => {
                ResponseSpec::status(200)
            }
            ("GET", "/neoforge/21.1.77/neoforge-21.1.77-installer.jar") => {
                ResponseSpec::bytes(200, b"pinned-jar".to_vec())
            }
            _ => ResponseSpec::status(500),
        })
        .await;
    let endpoints = InstallerEndpoints::from_base(server.base_url());
    let client = loopback_client();

    let err = ensure_installed_with_endpoints(
        &client,
        dir,
        &ServerCoreType::NeoForge,
        "1.21.1",
        Some("21.1.77"),
        &endpoints,
        |arguments| -> Result<Command, InjectedCommandError> {
            assert_eq!(
                arguments,
                ["-jar", "neoforge-installer.jar", "--installServer"]
            );
            Err(InjectedCommandError {
                msg: "stop after pinned download".into(),
            })
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(err, InstallerError::Process { .. }));
    assert!(!dir.join("neoforge-installer.jar").exists());

    let captured = server.finish().await;
    let trace: Vec<(&str, &str)> = captured
        .iter()
        .map(|req| (req.method.as_str(), req.path.as_str()))
        .collect();
    assert_eq!(
        trace,
        vec![
            ("HEAD", "/neoforge/21.1.77/neoforge-21.1.77-installer.jar"),
            ("GET", "/neoforge/21.1.77/neoforge-21.1.77-installer.jar"),
        ]
    );
}

#[tokio::test]
async fn ensure_installed_resolves_forge_recommended_and_handles_download_failure() {
    let fixture = tempfile::tempdir().unwrap();
    let dir = fixture.path();
    let props = dir.join("server.properties");
    std::fs::write(&props, b"motd=forge\n").unwrap();

    let server =
        LoopbackServer::with_handler(|req| match (req.method.as_str(), req.path.as_str()) {
            ("GET", "/forge/promotions_slim.json") => ResponseSpec::json(
                200,
                br#"{"promos":{"1.20.1-latest":"47.4.12","1.20.1-recommended":"47.4.10"}}"#
                    .to_vec(),
            ),
            _ => ResponseSpec::status(500),
        })
        .await;
    let endpoints = InstallerEndpoints::from_base(server.base_url());
    let client = loopback_client();

    let err = ensure_installed_with_endpoints(
        &client,
        dir,
        &ServerCoreType::Forge,
        "1.20.1",
        None,
        &endpoints,
        |_| -> Result<Command, InjectedCommandError> {
            panic!("java must not run when jar download fails")
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(err, InstallerError::Download(_)));
    assert!(!dir.join("forge-installer.jar").exists());
    assert_eq!(std::fs::read(&props).unwrap(), b"motd=forge\n");

    let captured = server.finish().await;
    let trace: Vec<(&str, &str)> = captured
        .iter()
        .map(|req| (req.method.as_str(), req.path.as_str()))
        .collect();
    assert_eq!(
        trace,
        vec![
            ("GET", "/forge/promotions_slim.json"),
            (
                "HEAD",
                "/forge/1.20.1-47.4.10/forge-1.20.1-47.4.10-installer.jar"
            ),
            (
                "GET",
                "/forge/1.20.1-47.4.10/forge-1.20.1-47.4.10-installer.jar"
            ),
        ]
    );
}

#[tokio::test]
async fn ensure_installed_resolves_quilt_installer_url_and_distinguishes_empty_from_malformed() {
    let fixture = tempfile::tempdir().unwrap();
    let dir = fixture.path();
    let client = loopback_client();

    let jar_server =
        LoopbackServer::with_handler(|req| match (req.method.as_str(), req.path.as_str()) {
            ("HEAD", "/quilt/installer-1.0.jar") => ResponseSpec::status(200),
            ("GET", "/quilt/installer-1.0.jar") => ResponseSpec::bytes(200, b"quilt-jar".to_vec()),
            _ => ResponseSpec::status(404),
        })
        .await;
    let installer_jar_url = jar_server.url("/quilt/installer-1.0.jar");
    let installer_listing = serde_json::json!([{ "url": installer_jar_url }]).to_string();

    let meta_server = LoopbackServer::exact_script(vec![
        ScriptedExchange::expect_path(
            "/quilt/v3/versions/installer",
            ResponseSpec::json(200, installer_listing.into_bytes()),
        ),
        ScriptedExchange::expect_path(
            "/quilt/v3/versions/installer",
            ResponseSpec::json(200, br#"[]"#.to_vec()),
        ),
        ScriptedExchange::expect_path(
            "/quilt/v3/versions/installer",
            ResponseSpec::json(200, br#"[{"version":"0.12.1"}]"#.to_vec()),
        ),
    ])
    .await;
    let endpoints = InstallerEndpoints::from_base(meta_server.base_url());

    let pinned_err = ensure_installed_with_endpoints(
        &client,
        dir,
        &ServerCoreType::Quilt,
        "1.21.1",
        Some("0.26.4"),
        &endpoints,
        |arguments| -> Result<Command, InjectedCommandError> {
            assert_eq!(
                arguments,
                [
                    "-jar",
                    "quilt-installer.jar",
                    "install",
                    "server",
                    "1.21.1",
                    "0.26.4",
                    "--download-server",
                    "--install-dir=.",
                ]
            );
            Err(InjectedCommandError {
                msg: "stop after quilt download".into(),
            })
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(
        pinned_err,
        InstallerError::Process {
            core: ServerCoreType::Quilt,
            ..
        }
    ));
    assert!(!dir.join("quilt-installer.jar").exists());

    let empty_err = ensure_installed_with_endpoints(
        &client,
        dir,
        &ServerCoreType::Quilt,
        "1.21.1",
        None,
        &endpoints,
        |_| -> Result<Command, InjectedCommandError> {
            panic!("java must not run on empty Quilt installer list")
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(empty_err, InstallerError::NoQuiltInstaller));

    let malformed_err = ensure_installed_with_endpoints(
        &client,
        dir,
        &ServerCoreType::Quilt,
        "1.21.1",
        None,
        &endpoints,
        |_| -> Result<Command, InjectedCommandError> {
            panic!("java must not run on malformed Quilt installer JSON")
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(malformed_err, InstallerError::Json(_)));

    assert_eq!(jar_server.finish().await.len(), 2);
    assert_eq!(meta_server.finish().await.len(), 3);
}

#[tokio::test]
async fn ensure_installed_returns_version_unavailable_when_metadata_lacks_game_version() {
    let fixture = tempfile::tempdir().unwrap();
    let dir = fixture.path();
    let server = LoopbackServer::exact_script(vec![ScriptedExchange::expect_path(
        "/forge/promotions_slim.json",
        ResponseSpec::json(200, br#"{"promos":{}}"#.to_vec()),
    )])
    .await;
    let endpoints = InstallerEndpoints::from_base(server.base_url());
    let client = loopback_client();

    let err = ensure_installed_with_endpoints(
        &client,
        dir,
        &ServerCoreType::Forge,
        "1.20.1",
        None,
        &endpoints,
        |_| -> Result<Command, InjectedCommandError> {
            panic!("java must not run when game version is unavailable")
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(
        err,
        InstallerError::VersionUnavailable {
            core: ServerCoreType::Forge,
            ref game_version,
        } if game_version == "1.20.1"
    ));
    assert_eq!(server.finish().await.len(), 1);
}
