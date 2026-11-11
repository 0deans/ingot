use super::{item, plan, write};
use crate::journal::Journal;
use crate::models::{backup_dir, work_dir};
use crate::transaction::swap;
use crate::{
    last_backup, targets, undo_typed, write_journal_typed, ContentKind, ItemAction, Target,
    TargetKind, VersionPlan,
};
use ingot_artifacts::crash;
use std::fs;

#[test]
fn missing_staged_replacement_fails_and_can_restore_original() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    write(root, &[("plugins/old.jar", "original")]);
    let target = targets::server(root, &ingot_server::ServerCoreType::Paper).unwrap();
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
    crate::transaction::restore(root, &backup, &journal).unwrap();
    assert_eq!(fs::read(root.join("plugins/old.jar")).unwrap(), b"original");
    assert!(!root.join("plugins/new.jar").exists());
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
    let target = targets::server(&root, &ingot_server::config::ServerCoreType::Purpur).unwrap();
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
    write_journal_typed(&root, &journal).unwrap();

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
        undo_typed(&root, "1.21.4", "paper").is_err(),
        "only undoes from what it changed to"
    );
    let restored = undo_typed(&root, "26.1", "purpur").unwrap();
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
    let restore_res = crate::transaction::restore(root, &backup, &journal);
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
