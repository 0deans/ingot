use super::*;
use std::fs;
use std::path::Path;

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

#[test]
fn recovery_rejects_journal_paths_outside_target() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("instance");
    fs::create_dir_all(&root).unwrap();
    let outside = directory.path().join("important.txt");
    fs::write(&outside, b"preserve").unwrap();
    for unsafe_path in [
        "../important.txt",
        "..\\important.txt",
        "/important.txt",
        "C:\\important.txt",
    ] {
        let journal = Journal {
            added: vec![unsafe_path.into()],
            ..Default::default()
        };
        write_journal_typed(&root, &journal).unwrap();
        assert!(matches!(
            read_journal_state(&root),
            JournalState::Corrupt(_)
        ));
        assert!(recover_interrupted_transaction(&root).is_err());
        assert_eq!(fs::read(&outside).unwrap(), b"preserve");
        assert!(backup_dir(&root).exists());
    }
}

#[test]
fn missing_staged_replacement_fails_and_can_restore_original() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    write(root, &[("plugins/old.jar", "original")]);
    let target = targets::server(root, &crate::server::ServerCoreType::Paper).unwrap();
    let plan = plan(vec![item(
        ContentKind::Plugin,
        "plugins",
        Some("old.jar"),
        ItemAction::Update,
        Some("new.jar"),
    )]);
    let backup = backup_dir(root);
    let staging = work_dir(root).join("staging");
    let mut journal = Journal::default();
    assert!(swap(
        &target,
        &backup,
        &staging,
        &plan,
        false,
        &mut journal,
        &|_| {}
    )
    .is_err());
    super::transaction::restore(root, &backup, &journal).unwrap();
    assert_eq!(fs::read(root.join("plugins/old.jar")).unwrap(), b"original");
    assert!(!root.join("plugins/new.jar").exists());
}

fn write(root: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
        fs::write(root.join(path), text).unwrap();
    }
}

/// Swap + undo on real files, without downloads (staging prepared by hand)
#[test]
fn swaps_and_undoes_exactly() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().to_path_buf();
    write(
        &root,
        &[
            ("plugins/keep.jar", "keep"),
            ("plugins/old-1.jar", "old"),
            ("plugins/gone.jar", "gone"),
            ("plugins/Gone/config.yml", "a: 1"),
            ("bukkit.yml", "old"),
            ("server.jar", "paper 1.21.4"),
            ("world/level.dat", "world"),
        ],
    );
    let staging = work_dir(&root).join("staging");
    write(
        &staging,
        &[("plugins/old-2.jar", "new"), ("plugins/dep.jar", "dep")],
    );
    let target = targets::server(&root, &crate::server::config::ServerCoreType::Purpur).unwrap();
    let plan = plan(vec![
        item(
            ContentKind::Plugin,
            "plugins",
            Some("keep.jar"),
            ItemAction::Keep,
            None,
        ),
        item(
            ContentKind::Plugin,
            "plugins",
            Some("old-1.jar"),
            ItemAction::Update,
            Some("old-2.jar"),
        ),
        item(
            ContentKind::Plugin,
            "plugins",
            Some("gone.jar"),
            ItemAction::Disable,
            None,
        ),
        item(
            ContentKind::Plugin,
            "plugins",
            None,
            ItemAction::Add,
            Some("dep.jar"),
        ),
    ]);
    let backup = backup_dir(&root);
    fs::create_dir_all(&backup).unwrap();
    let mut journal = Journal {
        from_game_version: "1.21.4".into(),
        from_loader: "paper".into(),
        from_loader_version: Some("100".into()),
        to_game_version: "26.1".into(),
        to_loader: "purpur".into(),
        ..Default::default()
    };
    swap(
        &target,
        &backup,
        &staging,
        &plan,
        true,
        &mut journal,
        &|_| {},
    )
    .unwrap();
    write_journal(&root, &journal).unwrap();

    let exists = |p: &str| root.join(p).exists();
    assert!(exists("plugins/keep.jar") && exists("plugins/old-2.jar") && exists("plugins/dep.jar"));
    assert!(
        !exists("plugins/old-1.jar")
            && !exists("plugins/gone.jar")
            && exists("plugins/gone.jar.disabled")
    );
    assert!(
        !exists("server.jar"),
        "the jar is downloaded again for the new version"
    );
    let info = last_backup(&root).unwrap().unwrap();
    assert_eq!(
        (
            info.to_game_version.as_str(),
            info.to_loader.as_str(),
            info.worlds_backed_up
        ),
        ("26.1", "purpur", true)
    );

    // The new version starts: new jar, upgraded world and configs
    write(
        &root,
        &[
            ("server.jar", "purpur 26.1"),
            ("world/level.dat", "upgraded"),
            ("bukkit.yml", "new"),
            ("plugins/Gone/config.yml", "a: 2"),
        ],
    );
    assert!(crash::first_start_pending(&root));
    assert!(
        undo(&root, "1.21.4", "paper").is_err(),
        "only undoes from what it changed to"
    );
    let restored = undo(&root, "26.1", "purpur").unwrap();
    assert_eq!(
        (restored.game_version.as_str(), restored.loader.as_str()),
        ("1.21.4", "paper")
    );
    assert_eq!(restored.loader_version.as_deref(), Some("100"));

    let read = |p: &str| fs::read_to_string(root.join(p)).unwrap();
    assert_eq!(read("plugins/old-1.jar"), "old");
    assert_eq!(read("plugins/gone.jar"), "gone");
    assert_eq!(read("server.jar"), "paper 1.21.4");
    assert_eq!(read("world/level.dat"), "world");
    assert_eq!(read("bukkit.yml"), "old");
    assert_eq!(read("plugins/Gone/config.yml"), "a: 1");
    assert!(
        !exists("plugins/old-2.jar")
            && !exists("plugins/dep.jar")
            && !exists("plugins/gone.jar.disabled")
    );
    assert!(last_backup(&root).unwrap().is_none());
    fs::remove_dir_all(&root).unwrap();
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
    write_journal(&root, &journal).unwrap();
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
    undo(&root, "26.1", "").unwrap();
    assert!(root.join("mods/bad.jar").exists());

    // A clean first start ends the watch
    fs::create_dir_all(backup_dir(&root)).unwrap();
    write_journal(&root, &journal).unwrap();
    crash::first_start_ok(&root);
    assert!(!crash::first_start_pending(&root));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn rejects_unsafe_names() {
    assert!(safe_file_name("sodium-0.6.jar", ContentKind::Mod).is_ok());
    assert!(safe_file_name("../evil.jar", ContentKind::Mod).is_err());
    assert!(safe_file_name("C:evil.jar", ContentKind::Mod).is_err());
    assert!(safe_file_name("pack.jar", ContentKind::ResourcePack).is_err());
    assert!(safe_rel("plugins/LuckPerms").is_ok());
    assert!(safe_rel("../x").is_err() && safe_rel("/etc").is_err());

    assert!(matches!(
        safe_file_name_typed("../evil.jar", ContentKind::Mod),
        Err(VersionChangeError::InvalidFileName(_))
    ));
    assert!(matches!(
        safe_rel_typed("../x"),
        Err(VersionChangeError::InvalidPath(_))
    ));
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
async fn metadata_commit_failure_triggers_clean_rollback() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    write(
        root,
        &[("plugins/old-1.jar", "old"), ("server.jar", "paper 1.21.4")],
    );
    let staging = work_dir(root).join("staging");
    write(&staging, &[("plugins/old-2.jar", "new")]);

    let target = targets::server(root, &crate::server::config::ServerCoreType::Purpur).unwrap();
    let plan = plan(vec![item(
        ContentKind::Plugin,
        "plugins",
        Some("old-1.jar"),
        ItemAction::Disable,
        None,
    )]);

    let client = reqwest::Client::new();
    let res = apply_typed_with_commit(
        &client,
        &target,
        &plan,
        false,
        |_| {},
        || Err::<(), _>("injected metadata failure: disk full"),
    )
    .await;

    match res {
        Err(VersionChangeError::RollbackSucceeded(msg)) => {
            assert!(msg.contains("injected metadata failure"));
        }
        other => panic!("expected RollbackSucceeded error, got: {:?}", other),
    }

    // Verify files were completely rolled back
    assert!(root.join("plugins/old-1.jar").exists());
    assert_eq!(
        fs::read_to_string(root.join("plugins/old-1.jar")).unwrap(),
        "old"
    );
    assert!(!root.join("plugins/old-1.jar.disabled").exists());
    assert!(root.join("server.jar").exists());
    assert_eq!(
        fs::read_to_string(root.join("server.jar")).unwrap(),
        "paper 1.21.4"
    );
    // Backup directory should have been removed since rollback succeeded
    assert!(!backup_dir(root).exists());
}

#[tokio::test]
async fn metadata_commit_failure_with_failed_restore_preserves_backup() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    write(
        root,
        &[("plugins/old-1.jar", "old"), ("server.jar", "paper 1.21.4")],
    );

    let target = targets::server(root, &crate::server::config::ServerCoreType::Purpur).unwrap();
    let plan = plan(vec![item(
        ContentKind::Plugin,
        "plugins",
        Some("old-1.jar"),
        ItemAction::Disable,
        None,
    )]);

    let client = reqwest::Client::new();
    let backup_path = backup_dir(root);

    let res = apply_typed_with_commit(
        &client,
        &target,
        &plan,
        false,
        |_| {},
        || {
            // Tamper with the backup files before failing commit, so that restore fails
            let _ = fs::remove_file(backup_path.join("files/server.jar"));
            Err::<(), _>("injected metadata failure")
        },
    )
    .await;

    match res {
        Err(VersionChangeError::RollbackFailed {
            primary,
            restore,
            backup_path: preserved,
        }) => {
            assert!(primary.contains("injected metadata failure"));
            assert!(!restore.is_empty());
            assert_eq!(preserved, backup_path);
        }
        other => panic!("expected RollbackFailed error, got: {:?}", other),
    }

    // Backup directory MUST still exist to preserve remaining recovery data!
    assert!(backup_dir(root).exists());
    let incomplete = detect_incomplete_transaction(root)
        .expect("should not error on valid journal")
        .expect("should detect incomplete transaction");
    assert_eq!(incomplete.phase, TransactionPhase::RollbackFailed);
}

#[test]
fn incomplete_transaction_detection_and_recovery() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    // Setup an interrupted transaction state:
    // "server.jar" was moved into backup, journal saved with Swapping phase
    let backup = backup_dir(root);
    fs::create_dir_all(backup.join("files")).unwrap();
    fs::write(backup.join("files/server.jar"), "paper 1.21.4").unwrap();

    let journal = Journal {
        created_at: 1000,
        from_game_version: "1.21.4".into(),
        from_loader: "paper".into(),
        from_loader_version: None,
        to_game_version: "1.21.5".into(),
        to_loader: "purpur".into(),
        phase: TransactionPhase::Swapping,
        moved: vec![("server.jar".into(), "files/server.jar".into())],
        ..Default::default()
    };
    write_journal_typed(root, &journal).unwrap();

    // 1. Detection works
    let incomplete = detect_incomplete_transaction(root).expect("check should succeed");
    assert!(incomplete.is_some());
    assert_eq!(incomplete.unwrap().phase, TransactionPhase::Swapping);

    // 2. Recovery restores files and cleans up
    recover_interrupted_transaction(root).expect("recovery should succeed");
    assert!(root.join("server.jar").exists());
    assert_eq!(
        fs::read_to_string(root.join("server.jar")).unwrap(),
        "paper 1.21.4"
    );
    assert!(!backup.exists());
    assert!(detect_incomplete_transaction(root).unwrap().is_none());
}

#[test]
fn undo_retry_after_metadata_failure_preserves_restored_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    let backup = backup_dir(root);
    fs::create_dir_all(&backup).unwrap();
    let mut journal = Journal {
        from_game_version: "1.20.1".into(),
        from_loader: "fabric".into(),
        to_game_version: "1.21.0".into(),
        to_loader: "fabric".into(),
        phase: TransactionPhase::RollbackFailed,
        ..Default::default()
    };
    write_journal_typed(root, &journal).unwrap();

    // Cannot undo a RollbackFailed transaction
    assert!(undo_typed(root, "1.21.0", "fabric").is_err());

    // Switch phase to Committed
    journal.phase = TransactionPhase::Committed;
    journal.moved = vec![("mods/test.jar".into(), "files/mods/test.jar".into())];
    fs::create_dir_all(backup.join("files/mods")).unwrap();
    fs::write(backup.join("files/mods/test.jar"), "test-content").unwrap();
    write_journal_typed(root, &journal).unwrap();

    // Undo with failing commit callback (attempt 1)
    let undo_res = undo_typed_with_commit(root, "1.21.0", "fabric", |_| {
        Err::<(), _>("failed to commit undo to instance config")
    });
    assert!(undo_res.is_err());
    // File was restored, but backup must be preserved because metadata commit failed
    assert!(root.join("mods/test.jar").exists());
    assert_eq!(
        fs::read_to_string(root.join("mods/test.jar")).unwrap(),
        "test-content"
    );
    assert!(
        backup.join("files/mods/test.jar").exists(),
        "backup source must NOT be deleted on failed commit"
    );

    // Undo retry with successful commit callback (attempt 2)
    let undo_retry = undo_typed_with_commit(root, "1.21.0", "fabric", |_| Ok::<(), &str>(()));
    assert!(
        undo_retry.is_ok(),
        "retry after metadata failure must succeed"
    );
    assert!(
        root.join("mods/test.jar").exists(),
        "restored file must still be present"
    );
    assert_eq!(
        fs::read_to_string(root.join("mods/test.jar")).unwrap(),
        "test-content"
    );
    // Backup is only deleted once commit callback succeeds!
    assert!(!backup.exists());
}

#[test]
fn corrupt_journal_is_detected_and_blocks_transactions() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    let backup = backup_dir(root);
    fs::create_dir_all(&backup).unwrap();
    fs::write(
        backup.join(crate::version_change::models::JOURNAL),
        "{ corrupt json",
    )
    .unwrap();

    // 1. Detection reports error (not missing / None)
    let res = detect_incomplete_transaction(root);
    assert!(res.is_err(), "corrupt journal must return error");

    // 2. State inspection reports Corrupt
    let state = crate::version_change::read_journal_state(root);
    assert!(matches!(
        state,
        crate::version_change::JournalState::Corrupt(_)
    ));

    // 3. Backup preservation: corrupt journal prevents destructive apply
    let dummy_target = crate::version_change::Target {
        root: root.to_path_buf(),
        core_files: vec![],
        folders: vec![],
        configs: vec![],
        worlds: vec![],
        hangar: std::collections::HashMap::new(),
    };
    let dummy_plan = crate::version_change::VersionPlan {
        target_kind: crate::version_change::TargetKind::Instance,
        target_id: "test".into(),
        from_game_version: "1.21.0".into(),
        from_loader: "vanilla".into(),
        from_loader_version: None,
        to_game_version: "1.21.1".into(),
        to_loader: "vanilla".into(),
        to_loader_version: None,
        downgrade: false,
        items: vec![],
        worlds: vec![],
    };
    let client = reqwest::Client::new();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let apply_res = rt.block_on(crate::version_change::apply_typed(
        &client,
        &dummy_target,
        &dummy_plan,
        false,
        |_| {},
    ));
    assert!(
        apply_res.is_err(),
        "apply must fail when corrupt journal exists"
    );
    assert!(
        backup.exists(),
        "backup directory must not be deleted when corrupt journal exists"
    );
}

#[test]
fn interrupted_transaction_at_mutation_boundary_recovers_idempotently() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    // Simulation: Intent recorded for "server.jar" and "mods/new_mod.jar",
    // but process interrupted before "server.jar" was moved, and after "staging" added a file.
    let backup = backup_dir(root);
    fs::create_dir_all(&backup).unwrap();
    fs::write(root.join("server.jar"), "original server jar").unwrap();
    fs::create_dir_all(root.join("mods")).unwrap();
    fs::write(root.join("mods/added.jar"), "partially added mod").unwrap();
    fs::write(root.join("mods/existing.jar.disabled"), "was disabled").unwrap();

    let journal = Journal {
        created_at: 2000,
        from_game_version: "1.20.1".into(),
        from_loader: "paper".into(),
        from_loader_version: None,
        to_game_version: "1.21.0".into(),
        to_loader: "paper".into(),
        phase: TransactionPhase::Swapping,
        moved: vec![("server.jar".into(), "files/server.jar".into())],
        disabled: vec!["mods/existing.jar".into()],
        added: vec!["mods/added.jar".into()],
        ..Default::default()
    };
    write_journal_typed(root, &journal).unwrap();

    // App-owned recovery with metadata revert
    let mut metadata_committed = false;
    let res =
        crate::version_change::recover_interrupted_transaction_with_commit(root, |restored| {
            assert_eq!(restored.game_version, "1.20.1");
            assert_eq!(restored.loader, "paper");
            metadata_committed = true;
            Ok::<(), &str>(())
        });
    assert!(res.is_ok(), "recovery must succeed: {:?}", res.err());
    assert!(metadata_committed);

    // Assert: original server.jar was preserved (not lost because backup source didn't exist yet)
    assert!(root.join("server.jar").exists());
    assert_eq!(
        fs::read_to_string(root.join("server.jar")).unwrap(),
        "original server jar"
    );
    // Assert: partially added mod was removed
    assert!(!root.join("mods/added.jar").exists());
    // Assert: disabled mod was re-enabled
    assert!(root.join("mods/existing.jar").exists());
    assert!(!root.join("mods/existing.jar.disabled").exists());
    // Assert: backup directory is cleaned up
    assert!(!backup.exists());
}

#[test]
fn test_swap_failed_world_backup_preserves_original_world() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let backup = backup_dir(root);
    fs::create_dir_all(&backup).unwrap();

    // Create an original world
    let world_dir = root.join("world");
    fs::create_dir_all(&world_dir).unwrap();
    fs::write(world_dir.join("level.dat"), "precious world data").unwrap();

    // Make the staging path uncreatable so staging copy fails
    let staging_parent = backup.join(".staging_worlds");
    fs::write(&staging_parent, "blocking file").unwrap();

    let target = Target {
        root: root.to_path_buf(),
        folders: vec![],
        worlds: vec!["world".to_string()],
        configs: vec![],
        core_files: vec![],
        hangar: std::collections::HashMap::new(),
    };
    let plan = VersionPlan {
        target_kind: TargetKind::Instance,
        target_id: "inst-1".into(),
        from_game_version: "1.20.1".into(),
        from_loader: "vanilla".into(),
        from_loader_version: None,
        to_game_version: "1.21".into(),
        to_loader: "vanilla".into(),
        to_loader_version: None,
        downgrade: false,
        items: vec![],
        worlds: vec![],
    };

    let mut journal = Journal::default();
    let res = swap(&target, &backup, root, &plan, true, &mut journal, &|_| {});
    assert!(res.is_err(), "Swap must fail when world backup fails");

    // The journal must NOT list the world
    assert!(!journal.worlds.contains(&"world".to_string()));

    // The original world must remain intact
    assert!(world_dir.join("level.dat").exists());
    assert_eq!(
        fs::read_to_string(world_dir.join("level.dat")).unwrap(),
        "precious world data"
    );

    // Restoring or rolling back must not delete or corrupt the original world
    let restore_res = crate::version_change::transaction::restore(root, &backup, &journal);
    assert!(restore_res.is_ok());
    assert!(world_dir.join("level.dat").exists());
    assert_eq!(
        fs::read_to_string(world_dir.join("level.dat")).unwrap(),
        "precious world data"
    );
}

#[test]
fn test_swap_failed_world_backup_partial_entry_failure_preserves_original_world() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let backup = backup_dir(root);
    fs::create_dir_all(&backup).unwrap();

    // Create an original world with multiple entries
    let world_dir = root.join("world");
    fs::create_dir_all(&world_dir).unwrap();
    fs::write(world_dir.join("level.dat"), "precious world data").unwrap();
    let locked_file = world_dir.join("locked.dat");
    fs::write(&locked_file, "locked world data").unwrap();

    // Lock locked.dat with exclusive sharing so that copy_path fails after copying level.dat
    #[cfg(target_os = "windows")]
    use std::os::windows::fs::OpenOptionsExt;
    #[cfg(target_os = "windows")]
    let lock_handle = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(&locked_file)
        .unwrap();

    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&locked_file, std::fs::Permissions::from_mode(0o000)).unwrap();
    }

    let target = Target {
        root: root.to_path_buf(),
        folders: vec![],
        worlds: vec!["world".to_string()],
        configs: vec![],
        core_files: vec![],
        hangar: std::collections::HashMap::new(),
    };
    let plan = VersionPlan {
        target_kind: TargetKind::Instance,
        target_id: "inst-1".into(),
        from_game_version: "1.20.1".into(),
        from_loader: "vanilla".into(),
        from_loader_version: None,
        to_game_version: "1.21".into(),
        to_loader: "vanilla".into(),
        to_loader_version: None,
        downgrade: false,
        items: vec![],
        worlds: vec![],
    };

    let mut journal = Journal::default();
    let res = swap(&target, &backup, root, &plan, true, &mut journal, &|_| {});
    assert!(res.is_err(), "Swap must fail when an entry copy fails");

    // The journal must NOT list the world as backed up
    assert!(!journal.worlds.contains(&"world".to_string()));

    // The partial staging directory must be cleaned up
    let staging_root = backup.join(".staging_worlds");
    if staging_root.exists() {
        let entries: Vec<_> = fs::read_dir(&staging_root).unwrap().flatten().collect();
        assert!(
            entries.is_empty(),
            "Staging entries must be cleaned up on failure"
        );
    }

    // The original world files must remain completely intact
    assert!(world_dir.join("level.dat").exists());
    assert_eq!(
        fs::read_to_string(world_dir.join("level.dat")).unwrap(),
        "precious world data"
    );

    // Release locks before tempdir cleanup
    #[cfg(target_os = "windows")]
    drop(lock_handle);
    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&locked_file, std::fs::Permissions::from_mode(0o644));
    }
}

#[test]
fn test_swap_and_restore_preserves_contained_symlinks() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("instance");
    fs::create_dir_all(&root).unwrap();

    let world_dir = root.join("saves").join("WorldWithLink");
    fs::create_dir_all(&world_dir).unwrap();
    fs::write(world_dir.join("level.dat"), "contained world data").unwrap();

    let link_target = "level.dat";
    let link_path = world_dir.join("link_to_level.dat");

    let link_res = {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(link_target, &link_path)
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(link_target, &link_path)
        }
    };
    if let Err(e) = link_res {
        eprintln!("Skipping symlink test: symlink creation not permitted: {e}");
        return;
    }

    let mut target = targets::instance(&root, "vanilla", "vanilla");
    target.worlds = vec!["saves/WorldWithLink".into()];
    let backup = backup_dir(&root);
    fs::create_dir_all(&backup).unwrap();
    let staging = work_dir(&root).join("staging");
    fs::create_dir_all(&staging).unwrap();
    let plan = plan(vec![]);
    let mut journal = Journal {
        from_game_version: "1.20.1".into(),
        from_loader: "vanilla".into(),
        to_game_version: "1.20.2".into(),
        to_loader: "vanilla".into(),
        ..Default::default()
    };

    swap(
        &target,
        &backup,
        &staging,
        &plan,
        true,
        &mut journal,
        &|_| {},
    )
    .unwrap();
    write_journal(&root, &journal).unwrap();

    // Verify backup contains the symlink
    let backup_link = backup
        .join("worlds")
        .join("saves/WorldWithLink")
        .join("link_to_level.dat");
    let backup_meta = fs::symlink_metadata(&backup_link).unwrap();
    assert!(backup_meta.file_type().is_symlink());

    // Modify target world before undo to simulate game rewrite
    fs::write(world_dir.join("level.dat"), "modified during play").unwrap();

    // Perform undo
    let restored = undo(&root, "1.20.2", "vanilla").unwrap();
    assert_eq!(restored.game_version, "1.20.1");

    // Restored world must contain the symlink pointing to level.dat
    let restored_link = world_dir.join("link_to_level.dat");
    let restored_meta = fs::symlink_metadata(&restored_link).unwrap();
    assert!(
        restored_meta.file_type().is_symlink(),
        "Restored link must be a symbolic link"
    );
    assert_eq!(
        fs::read_to_string(&restored_link).unwrap(),
        "contained world data",
        "Reading through the restored symlink must return original data"
    );
}

#[test]
fn test_swap_rejects_uncontained_symlinks_before_mutations() {
    let temp = tempfile::tempdir().unwrap();
    let outside_dir = temp.path().join("outside");
    fs::create_dir_all(&outside_dir).unwrap();
    let secret_file = outside_dir.join("secret.txt");
    fs::write(&secret_file, "outside secret").unwrap();

    let root = temp.path().join("instance");
    fs::create_dir_all(&root).unwrap();
    let world_dir = root.join("saves").join("WorldEscape");
    fs::create_dir_all(&world_dir).unwrap();
    fs::write(world_dir.join("level.dat"), "world data").unwrap();

    let escape_link = world_dir.join("escape_link");
    let link_res = {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&secret_file, &escape_link)
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(&secret_file, &escape_link)
        }
    };
    if let Err(e) = link_res {
        eprintln!("Skipping symlink test: symlink creation not permitted: {e}");
        return;
    }

    let mut target = targets::instance(&root, "vanilla", "vanilla");
    target.worlds = vec!["saves/WorldEscape".into()];
    let backup = backup_dir(&root);
    fs::create_dir_all(&backup).unwrap();
    let staging = work_dir(&root).join("staging");
    fs::create_dir_all(&staging).unwrap();
    let plan = plan(vec![]);
    let mut journal = Journal {
        from_game_version: "1.20.1".into(),
        from_loader: "vanilla".into(),
        to_game_version: "1.20.2".into(),
        to_loader: "vanilla".into(),
        ..Default::default()
    };

    let swap_res = swap(
        &target,
        &backup,
        &staging,
        &plan,
        true,
        &mut journal,
        &|_| {},
    );

    assert!(swap_res.is_err(), "Swap must reject uncontained symlink");
    // Original world must be completely untouched
    assert!(world_dir.join("level.dat").exists());
    assert_eq!(
        fs::read_to_string(world_dir.join("level.dat")).unwrap(),
        "world data"
    );
    assert!(fs::symlink_metadata(&escape_link)
        .unwrap()
        .file_type()
        .is_symlink());
}

#[test]
fn test_swap_and_restore_preserves_contained_absolute_symlinks() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("instance");
    fs::create_dir_all(&root).unwrap();
    let world_dir = root.join("saves").join("WorldWithAbsLink");
    fs::create_dir_all(&world_dir).unwrap();
    let target_file = world_dir.join("level.dat");
    fs::write(&target_file, "contained world data").unwrap();

    let link_path = world_dir.join("link_to_level.dat");
    let link_res = {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target_file, &link_path)
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(&target_file, &link_path)
        }
    };
    if let Err(e) = link_res {
        eprintln!("Skipping symlink test: symlink creation not permitted: {e}");
        return;
    }

    let mut target = targets::instance(&root, "vanilla", "vanilla");
    target.worlds = vec!["saves/WorldWithAbsLink".into()];
    let backup = backup_dir(&root);
    fs::create_dir_all(&backup).unwrap();
    let staging = work_dir(&root).join("staging");
    fs::create_dir_all(&staging).unwrap();
    let plan = plan(vec![]);
    let mut journal = Journal {
        from_game_version: "1.20.1".into(),
        from_loader: "vanilla".into(),
        to_game_version: "1.20.2".into(),
        to_loader: "vanilla".into(),
        ..Default::default()
    };

    swap(
        &target,
        &backup,
        &staging,
        &plan,
        true,
        &mut journal,
        &|_| {},
    )
    .unwrap();
    write_journal(&root, &journal).unwrap();

    // Verify backup contains the symlink remapped to point inside backup
    let backup_world = backup.join("worlds").join("saves/WorldWithAbsLink");
    let backup_link = backup_world.join("link_to_level.dat");
    let backup_meta = fs::symlink_metadata(&backup_link).unwrap();
    assert!(backup_meta.file_type().is_symlink());
    let backup_target = fs::read_link(&backup_link).unwrap();
    assert_eq!(backup_target, backup_world.join("level.dat"));

    // Modify target world before undo to simulate game rewrite
    fs::write(world_dir.join("level.dat"), "modified during play").unwrap();

    // Perform undo
    let restored = undo(&root, "1.20.2", "vanilla").unwrap();
    assert_eq!(restored.game_version, "1.20.1");

    // Restored world must contain the symlink remapped back to point to world_dir/level.dat
    let restored_link = world_dir.join("link_to_level.dat");
    let restored_meta = fs::symlink_metadata(&restored_link).unwrap();
    assert!(
        restored_meta.file_type().is_symlink(),
        "Restored link must be a symbolic link"
    );
    let restored_target = fs::read_link(&restored_link).unwrap();
    assert_eq!(restored_target, target_file);
    assert_eq!(
        fs::read_to_string(&restored_link).unwrap(),
        "contained world data",
        "Reading through the restored absolute symlink must return original data"
    );
}

#[test]
#[cfg(windows)]
fn test_is_contained_rejects_unrelated_unicode_paths_without_panicking() {
    use super::transaction::{is_contained, strip_contained_root};

    // The user's exact case: root C:\world (8 bytes) vs C:\世界\x (offset 8 falls within '界')
    let root = Path::new(r"C:\world");
    let unicode_target = Path::new(r"C:\世界\x");

    assert!(!is_contained(
        Path::new(r"C:\world\link"),
        unicode_target,
        root
    ));
    assert_eq!(strip_contained_root(unicode_target, root), None);

    // Contained unicode target should be accepted and stripped properly
    let contained_unicode = Path::new(r"C:\world\世界\level.dat");
    assert!(is_contained(
        Path::new(r"C:\world\link"),
        contained_unicode,
        root
    ));
    #[cfg(windows)]
    assert_eq!(
        strip_contained_root(contained_unicode, root),
        Some(std::path::PathBuf::from(r"世界\level.dat"))
    );
}

#[cfg(unix)]
#[test]
fn unix_containment_handles_unicode_path_components() {
    use super::transaction::{is_contained, strip_contained_root};

    let root = Path::new("/world");
    let link = root.join("link");
    let outside = Path::new("/世界/x");
    assert!(!is_contained(&link, outside, root));
    assert_eq!(strip_contained_root(outside, root), None);

    let inside = root.join("世界/level.dat");
    assert!(is_contained(&link, &inside, root));
    assert_eq!(
        strip_contained_root(&inside, root),
        Some(std::path::PathBuf::from("世界/level.dat"))
    );
    assert!(!is_contained(&link, Path::new("/world-sibling/x"), root));
}

#[test]
fn test_swap_rejects_unrelated_unicode_symlink_without_panicking() {
    let temp = tempfile::tempdir().unwrap();
    // Unrelated Unicode directory outside instance
    let outside_dir = temp.path().join("世界");
    fs::create_dir_all(&outside_dir).unwrap();
    let secret_file = outside_dir.join("secret.txt");
    fs::write(&secret_file, "unicode outside secret").unwrap();

    let root = temp.path().join("instance");
    fs::create_dir_all(&root).unwrap();
    let world_dir = root.join("saves").join("WorldUnicodeEscape");
    fs::create_dir_all(&world_dir).unwrap();
    fs::write(world_dir.join("level.dat"), "world data").unwrap();

    let escape_link = world_dir.join("escape_link");
    let link_res = {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&secret_file, &escape_link)
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(&secret_file, &escape_link)
        }
    };
    if let Err(e) = link_res {
        eprintln!("Skipping symlink test: symlink creation not permitted: {e}");
        return;
    }

    let mut target = targets::instance(&root, "vanilla", "vanilla");
    target.worlds = vec!["saves/WorldUnicodeEscape".into()];
    let backup = backup_dir(&root);
    fs::create_dir_all(&backup).unwrap();
    let staging = work_dir(&root).join("staging");
    fs::create_dir_all(&staging).unwrap();
    let plan = plan(vec![]);
    let mut journal = Journal {
        from_game_version: "1.20.1".into(),
        from_loader: "vanilla".into(),
        to_game_version: "1.20.2".into(),
        to_loader: "vanilla".into(),
        ..Default::default()
    };

    let swap_res = swap(
        &target,
        &backup,
        &staging,
        &plan,
        true,
        &mut journal,
        &|_| {},
    );

    assert!(
        swap_res.is_err(),
        "Swap must reject uncontained Unicode symlink"
    );
    let err_msg = swap_res.err().unwrap().to_string();
    assert!(
        err_msg.contains("escaping root") || err_msg.contains("uncontained link"),
        "Expected uncontained link error, got: {err_msg}"
    );

    // Original world must be completely untouched
    assert!(world_dir.join("level.dat").exists());
    assert_eq!(
        fs::read_to_string(world_dir.join("level.dat")).unwrap(),
        "world data"
    );
    assert!(fs::symlink_metadata(&escape_link)
        .unwrap()
        .file_type()
        .is_symlink());
}

#[test]
fn recovery_restores_missing_plugin_tracking_to_absence() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let target = targets::server(root, &crate::server::ServerCoreType::Purpur).unwrap();
    let mut journal = Journal {
        phase: TransactionPhase::Swapping,
        ..Default::default()
    };
    let backup = backup_dir(root);
    fs::create_dir_all(&backup).unwrap();
    let staging = work_dir(root).join("staging");
    fs::create_dir_all(&staging).unwrap();
    swap(
        &target,
        &backup,
        &staging,
        &plan(Vec::new()),
        false,
        &mut journal,
        &|_| {},
    )
    .unwrap();
    assert!(journal
        .absent_configs
        .contains(&".ingot/plugins.json".to_string()));
    write(root, &[(".ingot/plugins.json", "[]")]);
    transaction::restore(root, &backup, &journal).unwrap();
    assert!(!root.join(".ingot/plugins.json").exists());
    transaction::restore(root, &backup, &journal).unwrap();
}

#[test]
fn legacy_journal_without_absence_list_remains_readable() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let journal = Journal::default();
    let mut json = serde_json::to_value(&journal).unwrap();
    json.as_object_mut().unwrap().remove("absentConfigs");
    fs::create_dir_all(backup_dir(root)).unwrap();
    fs::write(
        backup_dir(root).join("journal.json"),
        serde_json::to_vec(&json).unwrap(),
    )
    .unwrap();
    assert!(read_journal(root).unwrap().absent_configs.is_empty());
}
