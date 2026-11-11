use super::plan;
use crate::{backup_dir, swap, targets, undo_typed, work_dir, write_journal_typed, Journal};
use std::fs;
use std::path::Path;

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
    write_journal_typed(&root, &journal).unwrap();

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
    let restored = undo_typed(&root, "1.20.2", "vanilla").unwrap();
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
    write_journal_typed(&root, &journal).unwrap();

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
    let restored = undo_typed(&root, "1.20.2", "vanilla").unwrap();
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
    use crate::transaction::{is_contained, strip_contained_root};

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
    use crate::transaction::{is_contained, strip_contained_root};

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
