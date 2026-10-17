use super::{paths::rooted_path, swap::swap};
use crate::version_change::{journal::Journal, models::*};
use std::{collections::HashMap, fs, path::Path};

fn directory_link(source: &Path, destination: &Path) -> bool {
    #[cfg(unix)]
    let result = std::os::unix::fs::symlink(source, destination);
    #[cfg(windows)]
    let result = std::os::windows::fs::symlink_dir(source, destination);
    if let Err(error) = result {
        #[cfg(windows)]
        if error.kind() == std::io::ErrorKind::PermissionDenied {
            eprintln!("Directory link test requires Windows symlink permission: {error}");
            return false;
        }
        panic!("Cannot create fixture link: {error}");
    }
    true
}

#[test]
fn swap_rejects_linked_mods_ancestor_without_touching_external_files() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("instance");
    let external = directory.path().join("external");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&external).unwrap();
    fs::write(external.join("original.jar"), b"preserve").unwrap();
    if !directory_link(&external, &root.join("mods")) {
        return;
    }
    let target = Target {
        root: root.clone(),
        folders: vec![Folder {
            kind: ContentKind::Mod,
            dir: "mods",
            scope: Default::default(),
        }],
        worlds: Vec::new(),
        configs: Vec::new(),
        core_files: Vec::new(),
        hangar: HashMap::new(),
    };
    let plan = VersionPlan {
        target_kind: TargetKind::Instance,
        target_id: "fixture".into(),
        from_game_version: "1.20".into(),
        from_loader: "fabric".into(),
        from_loader_version: None,
        to_game_version: "1.21".into(),
        to_loader: "fabric".into(),
        to_loader_version: None,
        downgrade: false,
        items: vec![PlanItem {
            kind: ContentKind::Mod,
            folder: "mods".into(),
            file_name: Some("original.jar".into()),
            title: "Original mod".into(),
            icon_url: None,
            page_url: None,
            source: None,
            project_id: None,
            current_version: None,
            target: None,
            status: ItemStatus::Works,
            action: ItemAction::Disable,
            actions: vec![ItemAction::Disable],
            note: None,
        }],
        worlds: Vec::new(),
    };
    let mut journal = Journal::default();
    assert!(matches!(
        swap(
            &target,
            &backup_dir(&root),
            &work_dir(&root).join("staging"),
            &plan,
            false,
            &mut journal,
            &|_| {}
        ),
        Err(VersionChangeError::InvalidPath(_))
    ));
    assert_eq!(
        fs::read(external.join("original.jar")).unwrap(),
        b"preserve"
    );
    assert!(journal.moved.is_empty());
}

#[test]
fn rooted_paths_allow_a_leaf_link_but_reject_using_it_as_an_ancestor() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("root");
    let external = directory.path().join("external");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&external).unwrap();
    if !directory_link(&external, &root.join("linked")) {
        return;
    }
    assert_eq!(rooted_path(&root, "linked").unwrap(), root.join("linked"));
    assert!(rooted_path(&root, "linked/child").is_err());
    assert!(rooted_path(&root.join("linked"), "child").is_err());
}

#[test]
fn rooted_paths_allow_trusted_aliases_above_the_transaction_root() {
    let directory = tempfile::tempdir().unwrap();
    let real = directory.path().join("real");
    fs::create_dir_all(real.join("instance")).unwrap();
    let alias = directory.path().join("alias");
    if !directory_link(&real, &alias) {
        return;
    }
    let root = alias.join("instance");
    assert!(rooted_path(&root, "mods/new.jar").is_ok());
    for unsafe_path in [
        "../external",
        "mods/../external",
        "/external",
        "C:\\external",
    ] {
        assert!(rooted_path(&root, unsafe_path).is_err());
    }
}
