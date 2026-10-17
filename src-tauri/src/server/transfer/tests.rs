use super::*;

#[test]
fn configs_export_skips_worlds_and_jars() {
    let root = std::env::temp_dir().join(format!("ingot-export-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    for dir in [
        "world/region",
        "plugins/LuckPerms",
        "logs",
        "config/ftbquests",
        "mods",
        ".ingot/map",
    ] {
        fs::create_dir_all(root.join(dir)).unwrap();
    }
    for file in [
        "server.properties",
        "server.jar",
        "whitelist.json",
        "world/level.dat",
        "world/region/r.0.0.mca",
        "plugins/LuckPerms.jar",
        "plugins/LuckPerms/config.yml",
        "logs/latest.log",
        "config/paper-global.yml",
        "config/ftbquests/chapter.snbt",
        "mods/create.jar",
        ".ingot/plugins.json",
        ".ingot/map/r.0.0.png",
        "quilt-server-launch.jar",
        "forge-26.1.2-64.1.0-shim.jar",
        "run.bat",
        "user_jvm_args.txt",
    ] {
        fs::write(root.join(file), "x").unwrap();
    }
    let list = |mode| {
        let mut out = Vec::new();
        collect(&root, &root, mode, &mut out).unwrap();
        let mut names: Vec<String> = out
            .iter()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .collect();
        names.sort();
        names
    };
    assert_eq!(
        list(ExportMode::Configs),
        [
            ".ingot/plugins.json",
            "config/ftbquests/chapter.snbt",
            "config/paper-global.yml",
            "plugins/LuckPerms/config.yml",
            "server.properties",
            "whitelist.json"
        ]
    );
    let full = list(ExportMode::Full);
    assert!(full.contains(&"world/region/r.0.0.mca".to_string()));
    assert!(full.contains(&"plugins/LuckPerms.jar".to_string()));
    assert!(full.contains(&"mods/create.jar".to_string()));
    assert!(full.contains(&".ingot/plugins.json".to_string()));
    assert!(!full
        .iter()
        .any(|f| f == "server.jar" || f.starts_with("logs/") || f.starts_with(".ingot/map")));
    assert!(!full.iter().any(|f| f.ends_with("-shim.jar")
        || f == "quilt-server-launch.jar"
        || f == "run.bat"
        || f == "user_jvm_args.txt"));
    let _ = fs::remove_dir_all(&root);
}
