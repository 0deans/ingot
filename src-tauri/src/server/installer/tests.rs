use super::*;
use crate::ipc::error::ApiError;
use serde_json::Value;
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
    let promos: serde_json::Map<String, Value> = serde_json::from_str(
        r#"{"1.20.1-latest":"47.4.12","1.20.1-recommended":"47.4.10","26.3-latest":"66.0.5"}"#,
    )
    .unwrap();
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

#[test]
fn relevant_ipc_categories_remain_correct() {
    // 1. Process / Command failure maps to internal
    let process_err = InstallerError::Process {
        core: ServerCoreType::NeoForge,
        source: Box::new(io::Error::new(io::ErrorKind::BrokenPipe, "pipe broken")),
    };
    let api_err = ApiError::from(process_err);
    assert!(matches!(api_err, ApiError::Internal { .. }));

    // 2. Io error maps to storage_failure
    let io_err = InstallerError::Io(io::Error::new(io::ErrorKind::PermissionDenied, "denied"));
    let api_err = ApiError::from(io_err);
    assert!(matches!(api_err, ApiError::StorageFailure { .. }));

    // 3. Not configured / missing version maps to not_found
    let not_configured = InstallerError::NotConfigured(ServerCoreType::Forge);
    let api_err = ApiError::from(not_configured);
    assert!(matches!(api_err, ApiError::NotFound { .. }));

    let version_unavailable = InstallerError::VersionUnavailable {
        core: ServerCoreType::Quilt,
        game_version: "1.99".to_string(),
    };
    let api_err = ApiError::from(version_unavailable);
    assert!(matches!(api_err, ApiError::NotFound { .. }));

    // 4. Unsupported core maps to invalid_input
    let unsupported = InstallerError::UnsupportedCore(ServerCoreType::Vanilla);
    let api_err = ApiError::from(unsupported);
    assert!(matches!(api_err, ApiError::InvalidInput { .. }));
}
