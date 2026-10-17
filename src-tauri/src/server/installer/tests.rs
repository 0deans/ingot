use super::*;
use serde_json::Value;

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
    assert_eq!(launch_args(&dir, &ServerCoreType::NeoForge), None);
    for v in ["21.1.99", "21.1.100"] {
        let d = dir.join("libraries/net/neoforged/neoforge").join(v);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(name), "").unwrap();
    }
    let expected = format!("@libraries/net/neoforged/neoforge/21.1.100/{name}");
    assert_eq!(
        launch_args(&dir, &ServerCoreType::NeoForge),
        Some(vec![expected])
    );
    assert_eq!(launch_args(&dir, &ServerCoreType::Forge), None);
    uninstall(&dir, &ServerCoreType::NeoForge).unwrap();
    assert_eq!(launch_args(&dir, &ServerCoreType::NeoForge), None);

    std::fs::write(dir.join(QUILT_LAUNCHER), "").unwrap();
    assert_eq!(
        launch_args(&dir, &ServerCoreType::Quilt),
        None,
        "needs Minecraft's server.jar too"
    );
    std::fs::write(dir.join("server.jar"), "").unwrap();
    assert!(launch_args(&dir, &ServerCoreType::Quilt).is_some());
    uninstall(&dir, &ServerCoreType::Quilt).unwrap();
    assert_eq!(launch_args(&dir, &ServerCoreType::Quilt), None);
}
