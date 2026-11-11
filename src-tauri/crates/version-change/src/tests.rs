use super::*;
use crate::journal::{read_journal, Journal};
use crate::models::backup_dir;
use ingot_artifacts::crash;
use std::fs;
use std::path::Path;

#[path = "swap_tests.rs"]
mod swap_tests;

#[path = "recovery_tests.rs"]
mod recovery_tests;

#[path = "symlink_tests.rs"]
mod symlink_tests;

fn item(
    kind: ContentKind,
    folder: &str,
    file: Option<&str>,
    action: ItemAction,
    target: Option<&str>,
) -> PlanItem {
    PlanItem {
        kind,
        folder: folder.into(),
        file_name: file.map(str::to_string),
        title: file.or(target).unwrap_or_default().to_string(),
        icon_url: None,
        page_url: None,
        source: None,
        project_id: None,
        current_version: None,
        target: target.map(|name| TargetFile {
            version_id: "v".into(),
            version_number: "2".into(),
            file_name: name.into(),
            url: String::new(),
            sha1: None,
            sha256: None,
            bytes: 0.0,
        }),
        status: ItemStatus::Works,
        action,
        actions: vec![action],
        note: None,
    }
}

fn plan(items: Vec<PlanItem>) -> VersionPlan {
    VersionPlan {
        target_kind: TargetKind::Server,
        target_id: "test".into(),
        from_game_version: "1.21.4".into(),
        from_loader: "paper".into(),
        from_loader_version: Some("100".into()),
        to_game_version: "26.1".into(),
        to_loader: "purpur".into(),
        to_loader_version: None,
        downgrade: false,
        items,
        worlds: Vec::new(),
    }
}

fn write(root: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
        fs::write(root.join(path), text).unwrap();
    }
}

#[test]
fn crash_check_and_turning_off_suspects() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().to_path_buf();
    write(
        &root,
        &[
            ("mods/bad.jar", "not really a jar"),
            ("logs/latest.log", "Mixin apply for mod bad failed\n"),
        ],
    );
    fs::create_dir_all(backup_dir(&root)).unwrap();
    let since = std::time::SystemTime::now() - std::time::Duration::from_secs(5);
    assert!(
        crash::analyze(&root, TargetKind::Instance, "i", "Test", &["mods"], since).is_none(),
        "no change applied"
    );

    let journal = Journal {
        from_game_version: "1.21.4".into(),
        to_game_version: "26.1".into(),
        ..Default::default()
    };
    write_journal_typed(&root, &journal).unwrap();
    let report =
        crash::analyze(&root, TargetKind::Instance, "i", "Test", &["mods"], since).unwrap();
    assert_eq!(report.to_game_version, "26.1");
    assert!(report.excerpt.contains("Mixin apply"));
    // Blamed by id; this jar has no metadata, so it isn't matched by name
    assert!(report.suspects.is_empty());

    crash::disable(&root, "mods", "bad.jar").unwrap();
    assert!(root.join("mods/bad.jar.disabled").exists());
    assert!(crash::disable(&root, "../x", "bad.jar").is_err());
    // Undo turns it back on
    undo_typed(&root, "26.1", "").unwrap();
    assert!(root.join("mods/bad.jar").exists());

    // A clean first start ends the watch
    fs::create_dir_all(backup_dir(&root)).unwrap();
    write_journal_typed(&root, &journal).unwrap();
    crash::first_start_ok(&root);
    assert!(!crash::first_start_pending(&root));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn rejects_unsafe_names() {
    assert!(safe_file_name_typed("sodium-0.6.jar", ContentKind::Mod).is_ok());
    assert!(matches!(
        safe_file_name_typed("../evil.jar", ContentKind::Mod),
        Err(VersionChangeError::InvalidFileName(_))
    ));
    assert!(matches!(
        safe_file_name_typed("C:evil.jar", ContentKind::Mod),
        Err(VersionChangeError::InvalidFileName(_))
    ));
    assert!(matches!(
        safe_file_name_typed("pack.jar", ContentKind::ResourcePack),
        Err(VersionChangeError::InvalidFileName(_))
    ));
    assert!(safe_rel_typed("plugins/LuckPerms").is_ok());
    assert!(matches!(
        safe_rel_typed("../x"),
        Err(VersionChangeError::InvalidPath(_))
    ));
    assert!(safe_rel_typed("/etc").is_err());
}

#[test]
fn formats_loader_labels() {
    assert_eq!(loader_label("neoforge"), "NeoForge");
    assert_eq!(loader_label("fabric"), "Fabric");
    assert_eq!(loader_label("paper"), "Paper");
    assert_eq!(loader_label("purpur"), "Purpur");
    assert_eq!(loader_label("forge"), "Forge");
}

#[test]
fn computes_expected_checksums() {
    use super::models::{sha1_hex, sha256_hex};
    let data = b"ingot-test-payload";
    assert_eq!(sha1_hex(data), "59438878c34d31baf3708e8765a61e5883e58c25");
    assert_eq!(
        sha256_hex(data),
        "bb58457d8c3a7d5365067af584ae4bbe34b0625e13f1d8a23e0e910fe77e5615"
    );
}

#[test]
fn journal_roundtrip_and_discard() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    assert!(read_journal(root).is_none());
    assert!(last_backup(root).unwrap().is_none());

    let journal = Journal {
        created_at: 12345678,
        from_game_version: "1.20.1".into(),
        from_loader: "fabric".into(),
        from_loader_version: Some("0.15.0".into()),
        to_game_version: "1.21.0".into(),
        to_loader: "fabric".into(),
        moved: vec![("mods/a.jar".into(), "files/mods/a.jar".into())],
        disabled: vec!["mods/b.jar".into()],
        added: vec!["mods/c.jar".into()],
        configs: vec!["config/a.json".into()],
        absent_configs: Vec::new(),
        worlds: vec!["saves/World".into()],
        phase: TransactionPhase::Committed,
        first_start_checked: false,
    };

    fs::create_dir_all(backup_dir(root)).unwrap();
    write_journal_typed(root, &journal).unwrap();

    let read_back = read_journal(root).unwrap();
    assert_eq!(read_back.from_game_version, "1.20.1");
    assert_eq!(read_back.to_game_version, "1.21.0");
    assert_eq!(read_back.moved.len(), 1);
    assert_eq!(read_back.disabled.len(), 1);
    assert_eq!(read_back.added.len(), 1);
    assert_eq!(read_back.configs.len(), 1);
    assert_eq!(read_back.worlds.len(), 1);

    let backup_info = last_backup(root).unwrap().unwrap();
    assert_eq!(backup_info.from_game_version, "1.20.1");
    assert!(backup_info.worlds_backed_up);

    discard_backup_typed(root).unwrap();
    assert!(read_journal(root).is_none());
    assert!(last_backup(root).unwrap().is_none());
}

#[tokio::test]
async fn planner_propagates_identification_target_dependency_and_metadata_failures() {
    use ingot_artifacts::sources::{SourceClient, SourceError};
    use ingot_test_support::{LoopbackServer, ResponseSpec, ScriptedExchange};
    use serde_json::json;

    struct Fixture {
        server: LoopbackServer,
    }

    impl Fixture {
        async fn new(responses: Vec<(u16, String)>) -> Self {
            let exchanges = responses
                .into_iter()
                .map(|(status, body)| {
                    ScriptedExchange::any_path(ResponseSpec::json(status, body.into_bytes()))
                })
                .collect();
            Self {
                server: LoopbackServer::exact_script(exchanges).await,
            }
        }

        fn sources<'a>(&self, client: &'a reqwest::Client) -> SourceClient<'a> {
            let endpoint = self.server.base_url();
            SourceClient::with_endpoints(client, endpoint, endpoint, endpoint)
        }

        async fn finish(self) {
            let _ = self.server.finish().await;
        }
    }

    fn modrinth_version(id: &str, channel: &str) -> serde_json::Value {
        json!({
            "id": id,
            "project_id": "project",
            "version_number": id,
            "version_type": channel,
            "game_versions": ["1.21"],
            "loaders": ["fabric"],
            "dependencies": [],
            "files": [{
                "filename": "mod.jar",
                "url": "https://cdn.modrinth.com/mod.jar",
                "size": 42,
                "primary": true,
                "hashes": { "sha1": "abc" }
            }]
        })
    }

    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join("mods")).unwrap();
    std::fs::write(directory.path().join("mods/mod.jar"), b"installed").unwrap();
    let hash = ingot_artifacts::models::sha1_hex(b"installed");
    let target = targets::instance(directory.path(), "fabric", "fabric");
    let current = modrinth_version("current", "release");
    let identified = json!({ (hash.clone()): current }).to_string();
    let mut old = modrinth_version("old", "release");
    old["game_versions"] = json!(["1.20"]);
    let mut needs_dependency = modrinth_version("current", "release");
    needs_dependency["dependencies"] =
        json!([{ "dependency_type": "required", "project_id": "dependency", "version_id": null }]);
    let cf_empty = json!({ "data": { "exactMatches": [] } }).to_string();
    let stages = vec![
        vec![(503, "{}".into())],
        vec![(200, "{}".into()), (503, "{}".into())],
        vec![
            (200, json!({ (hash.clone()): old }).to_string()),
            (200, cf_empty.clone()),
            (503, "{}".into()),
        ],
        vec![
            (200, json!({ (hash.clone()): needs_dependency }).to_string()),
            (200, cf_empty.clone()),
            (503, "{}".into()),
        ],
        vec![(200, identified), (200, cf_empty), (503, "{}".into())],
    ];
    let client = reqwest::Client::new();
    for responses in stages {
        let fixture = Fixture::new(responses).await;
        let request = PlanRequest {
            target_kind: TargetKind::Instance,
            target_id: "test".into(),
            from_game_version: "1.20".into(),
            from_loader: "fabric".into(),
            from_loader_version: None,
            to_game_version: "1.21".into(),
            to_loader: "fabric".into(),
            to_loader_version: None,
            downgrade: false,
        };
        let result = planner::check_with_sources(&fixture.sources(&client), &target, request).await;
        assert!(matches!(
            result,
            Err(VersionChangeError::Source(SourceError::HttpStatus { .. }))
        ));
        fixture.finish().await;
    }
}
