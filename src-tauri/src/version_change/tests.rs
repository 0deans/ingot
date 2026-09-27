use super::*;

fn item(kind: ContentKind, folder: &str, file: Option<&str>, action: ItemAction, target: Option<&str>) -> PlanItem {
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

/// Swap + undo on real files, without downloads (staging prepared by hand)
#[test]
fn swaps_and_undoes_exactly() {
    let root = std::env::temp_dir().join(format!("ingot-version-change-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
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
    write(&staging, &[("plugins/old-2.jar", "new"), ("plugins/dep.jar", "dep")]);
    let target = targets::server(&root, &crate::server::config::ServerCoreType::Purpur);
    let plan = plan(vec![
        item(ContentKind::Plugin, "plugins", Some("keep.jar"), ItemAction::Keep, None),
        item(ContentKind::Plugin, "plugins", Some("old-1.jar"), ItemAction::Update, Some("old-2.jar")),
        item(ContentKind::Plugin, "plugins", Some("gone.jar"), ItemAction::Disable, None),
        item(ContentKind::Plugin, "plugins", None, ItemAction::Add, Some("dep.jar")),
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
    swap(&target, &backup, &staging, &plan, true, &mut journal, &|_| {}).unwrap();
    write_journal(&root, &journal).unwrap();

    let exists = |p: &str| root.join(p).exists();
    assert!(exists("plugins/keep.jar") && exists("plugins/old-2.jar") && exists("plugins/dep.jar"));
    assert!(!exists("plugins/old-1.jar") && !exists("plugins/gone.jar") && exists("plugins/gone.jar.disabled"));
    assert!(!exists("server.jar"), "the jar is downloaded again for the new version");
    let info = last_backup(&root).unwrap();
    assert_eq!((info.to_game_version.as_str(), info.to_loader.as_str(), info.worlds_backed_up), ("26.1", "purpur", true));

    // The new version starts: new jar, upgraded world and configs
    write(&root, &[("server.jar", "purpur 26.1"), ("world/level.dat", "upgraded"), ("bukkit.yml", "new"), ("plugins/Gone/config.yml", "a: 2")]);
    assert!(crash::first_start_pending(&root));
    assert!(undo(&root, "1.21.4", "paper").is_err(), "only undoes from what it changed to");
    let restored = undo(&root, "26.1", "purpur").unwrap();
    assert_eq!((restored.game_version.as_str(), restored.loader.as_str()), ("1.21.4", "paper"));
    assert_eq!(restored.loader_version.as_deref(), Some("100"));

    let read = |p: &str| fs::read_to_string(root.join(p)).unwrap();
    assert_eq!(read("plugins/old-1.jar"), "old");
    assert_eq!(read("plugins/gone.jar"), "gone");
    assert_eq!(read("server.jar"), "paper 1.21.4");
    assert_eq!(read("world/level.dat"), "world");
    assert_eq!(read("bukkit.yml"), "old");
    assert_eq!(read("plugins/Gone/config.yml"), "a: 1");
    assert!(!exists("plugins/old-2.jar") && !exists("plugins/dep.jar") && !exists("plugins/gone.jar.disabled"));
    assert!(last_backup(&root).is_none());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn crash_check_and_turning_off_suspects() {
    let root = std::env::temp_dir().join(format!("ingot-version-crash-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    write(&root, &[("mods/bad.jar", "not really a jar"), ("logs/latest.log", "Mixin apply for mod bad failed\n")]);
    fs::create_dir_all(backup_dir(&root)).unwrap();
    let since = std::time::SystemTime::now() - std::time::Duration::from_secs(5);
    assert!(crash::analyze(&root, TargetKind::Instance, "i", "Test", &["mods"], since).is_none(), "no change applied");

    let journal = Journal { from_game_version: "1.21.4".into(), to_game_version: "26.1".into(), ..Default::default() };
    write_journal(&root, &journal).unwrap();
    let report = crash::analyze(&root, TargetKind::Instance, "i", "Test", &["mods"], since).unwrap();
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
}
