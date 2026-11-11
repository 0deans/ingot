use super::{item, plan, write};
use crate::{
    apply_typed_with_commit, backup_dir, detect_incomplete_transaction, journal, read_journal,
    read_journal_state, recover_interrupted_transaction, swap, targets, transaction, undo_typed,
    undo_typed_with_commit, work_dir, write_journal_typed, ContentKind, ItemAction, Journal,
    JournalState, TransactionPhase, VersionChangeError,
};
use std::fs;

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
        || {
            Err::<(), _>(std::io::Error::other(
                "injected metadata failure: disk full",
            ))
        },
    )
    .await;

    match res {
        Err(VersionChangeError::RollbackSucceeded(msg)) => {
            assert!(msg.to_string().contains("injected metadata failure"));
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
            Err::<(), _>(std::io::Error::other("injected metadata failure"))
        },
    )
    .await;

    match res {
        Err(VersionChangeError::RollbackFailed {
            primary,
            restore,
            backup_path: preserved,
            ..
        }) => {
            assert!(primary.to_string().contains("injected metadata failure"));
            assert!(!restore.to_string().is_empty());
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
        Err::<(), _>(std::io::Error::other(
            "failed to commit undo to instance config",
        ))
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
    let undo_retry =
        undo_typed_with_commit(root, "1.21.0", "fabric", |_| Ok::<(), std::io::Error>(()));
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
    fs::write(backup.join(crate::models::JOURNAL), "{ corrupt json").unwrap();

    // 1. Detection reports error (not missing / None)
    let res = detect_incomplete_transaction(root);
    assert!(res.is_err(), "corrupt journal must return error");

    // 2. State inspection reports Corrupt
    let state = crate::read_journal_state(root);
    assert!(matches!(state, crate::JournalState::Corrupt(_)));

    // 3. Backup preservation: corrupt journal prevents destructive apply
    let dummy_target = crate::Target {
        root: root.to_path_buf(),
        core_files: vec![],
        folders: vec![],
        configs: vec![],
        worlds: vec![],
        hangar: std::collections::HashMap::new(),
    };
    let dummy_plan = crate::VersionPlan {
        target_kind: crate::TargetKind::Instance,
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
    let apply_res = rt.block_on(crate::apply_typed(
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
    let res = crate::recover_interrupted_transaction_with_commit(root, |restored| {
        assert_eq!(restored.game_version, "1.20.1");
        assert_eq!(restored.loader, "paper");
        metadata_committed = true;
        Ok::<(), std::io::Error>(())
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

#[test]
fn interrupted_disable_intent_can_be_restored_before_or_after_rename() {
    for rename_completed in [false, true] {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("mods")).unwrap();
        fs::write(root.path().join("mods/a.jar"), b"original").unwrap();
        let mut journal = Journal::default();
        journal.disabled.push("mods/a.jar".into());
        journal::write_journal_typed(root.path(), &journal).unwrap();
        if rename_completed {
            fs::rename(
                root.path().join("mods/a.jar"),
                root.path().join("mods/a.jar.disabled"),
            )
            .unwrap();
        }
        transaction::restore(root.path(), &backup_dir(root.path()), &journal).unwrap();
        assert_eq!(
            fs::read(root.path().join("mods/a.jar")).unwrap(),
            b"original"
        );
        assert!(!root.path().join("mods/a.jar.disabled").exists());
    }
}
