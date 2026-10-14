use super::metadata::{read_jar_metadata, safe_jar_name};
use super::models::{platform, PluginSource, PluginVersion, TrackedPlugin};
use super::providers::pick_version;
use super::service::{search, versions};
use super::tracking::{read_tracking, write_tracking};
use crate::server::config::ServerCoreType;

#[test]
fn jar_names_are_restricted() {
    assert!(safe_jar_name("EssentialsX-2.21.jar").is_ok());
    assert!(safe_jar_name("EssentialsX-2.21.jar.disabled").is_ok());
    assert!(safe_jar_name("../evil.jar").is_err());
    assert!(safe_jar_name("sub/dir.jar").is_err());
    assert!(safe_jar_name("config.yml").is_err());
    assert!(safe_jar_name(".hidden.jar").is_err());
}

#[test]
fn prefers_stable_versions() {
    let v = |id: &str, channel: &str, url: bool| PluginVersion {
        id: id.into(),
        version_number: id.into(),
        channel: channel.into(),
        game_versions: vec![],
        date: String::new(),
        downloads: 0,
        file_name: format!("{id}.jar"),
        download_url: url.then(|| "https://x".into()),
        external_url: None,
        size: 0,
        dependencies: vec![],
    };
    let list = [
        v("3-beta", "beta", true),
        v("2", "release", false),
        v("1", "release", true),
    ];
    assert_eq!(pick_version(&list).map(|v| v.id.as_str()), Some("1"));
    assert_eq!(
        pick_version(&list[..1]).map(|v| v.id.as_str()),
        Some("3-beta")
    );
}

#[test]
fn reads_mod_metadata() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let jar = |name: &str, files: &[(&str, &str)]| {
        let path = dir.join(name);
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        for (file, text) in files {
            zip.start_file(*file, zip::write::SimpleFileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut zip, text.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        read_jar_metadata(&path).unwrap()
    };

    let neoforge = jar(
        "jei.jar",
        &[
            (
                "META-INF/neoforge.mods.toml",
                "modLoader=\"javafml\"\nauthors=\"mezz, Ingot\"\n[[mods]]\nmodId=\"jei\"\ndisplayName=\"Just Enough Items\"\nversion=\"${file.jarVersion}\"\ndescription='''\nShows recipes\n'''\n",
            ),
            (
                "META-INF/MANIFEST.MF",
                "Manifest-Version: 1.0\nImplementation-Version: 19.21.0\n",
            ),
        ],
    );
    assert_eq!(neoforge.0, "Just Enough Items");
    assert_eq!(neoforge.1.as_deref(), Some("19.21.0"));
    assert_eq!(neoforge.2.as_deref(), Some("Shows recipes"));
    assert_eq!(neoforge.3, vec!["mezz", "Ingot"]);

    let forge = jar(
        "forge.jar",
        &[(
            "META-INF/mods.toml",
            "[[mods]]\nmodId=\"create\"\nversion=\"6.0.6\"\n",
        )],
    );
    assert_eq!(
        (forge.0.as_str(), forge.1.as_deref()),
        ("create", Some("6.0.6"))
    );

    let quilt = jar(
        "quilt.jar",
        &[(
            "quilt.mod.json",
            r#"{"quilt_loader":{"id":"qsl","version":"1.0.0","metadata":{"name":"Quilt Standard Libraries","contributors":{"QuiltMC":"Owner"}}}}"#,
        )],
    );
    assert_eq!(
        (quilt.0.as_str(), quilt.1.as_deref()),
        ("Quilt Standard Libraries", Some("1.0.0"))
    );
    assert_eq!(quilt.3, vec!["QuiltMC"]);
}

#[test]
fn test_platform_loader_mappings() {
    let paper = platform(&ServerCoreType::Paper).unwrap();
    assert_eq!(paper.folder, "plugins");
    assert!(paper.hangar);

    let fabric = platform(&ServerCoreType::Fabric).unwrap();
    assert_eq!(fabric.folder, "mods");
    assert!(!fabric.hangar);

    assert!(platform(&ServerCoreType::Vanilla).is_none());
    assert!(platform(&ServerCoreType::Bedrock).is_none());
}

#[test]
fn test_tracking_persistence() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    assert!(read_tracking(dir).is_empty());

    let tracked = vec![TrackedPlugin {
        file_name: "WorldEdit.jar".to_string(),
        source: PluginSource::Modrinth,
        project_id: "worldedit-1".to_string(),
        version_id: "ver-1".to_string(),
        version_number: "7.2.15".to_string(),
        title: "WorldEdit".to_string(),
        icon_url: Some("https://example.com/icon.png".to_string()),
    }];
    write_tracking(dir, &tracked);

    let read_back = read_tracking(dir);
    assert_eq!(read_back.len(), 1);
    assert_eq!(read_back[0].file_name, "WorldEdit.jar");
    assert_eq!(read_back[0].title, "WorldEdit");
}

#[test]
fn tracking_safe_replacement_and_persistence() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    let tracked = vec![TrackedPlugin {
        file_name: "PluginA.jar".to_string(),
        source: PluginSource::Modrinth,
        project_id: "mod-a".to_string(),
        version_id: "v1".to_string(),
        version_number: "1.0.0".to_string(),
        title: "Plugin A".to_string(),
        icon_url: None,
    }];
    super::tracking::write_tracking_typed(dir, &tracked).expect("initial write should succeed");

    // Overwrite safely
    let mut updated = tracked.clone();
    updated.push(TrackedPlugin {
        file_name: "PluginB.jar".to_string(),
        source: PluginSource::Hangar,
        project_id: "mod-b".to_string(),
        version_id: "v2".to_string(),
        version_number: "2.0.0".to_string(),
        title: "Plugin B".to_string(),
        icon_url: None,
    });
    super::tracking::write_tracking_typed(dir, &updated).expect("safe replacement write should succeed");

    let read_back = read_tracking(dir);
    assert_eq!(read_back.len(), 2);
    assert_eq!(read_back[0].title, "Plugin A");
    assert_eq!(read_back[1].title, "Plugin B");
}

#[test]
fn test_publish_installed_plugin_tracking_failure_restores_backup() {
    let tmp = tempfile::tempdir().unwrap();
    let plugins_dir = tmp.path().join("plugins");
    std::fs::create_dir_all(&plugins_dir).unwrap();

    let target_dest = plugins_dir.join("ViaVersion.jar");
    std::fs::write(&target_dest, "version 1 contents").unwrap();

    let staging_file = plugins_dir.join(".staging.jar");
    std::fs::write(&staging_file, "version 2 new contents").unwrap();

    // Directly call the production publication function with a failing tracking commit
    let res = super::service::publish_installed_plugin(
        &staging_file,
        &target_dest,
        "ViaVersion",
        || Err(super::models::PluginError::Other("disk full".into())),
        &[],
    );

    // Verify it returns PublicationFailed
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(matches!(err, super::models::PluginError::PublicationFailed { .. }));
    assert!(err.to_string().contains("Previous version was restored"));

    // Verify that target_dest has been safely restored with version 1 contents
    assert!(target_dest.exists());
    assert_eq!(std::fs::read_to_string(&target_dest).unwrap(), "version 1 contents");
}

#[test]
fn test_publish_installed_plugin_rollback_failure_retains_and_reports_backup() {
    let tmp = tempfile::tempdir().unwrap();
    let plugins_dir = tmp.path().join("plugins");
    std::fs::create_dir_all(&plugins_dir).unwrap();

    let target_dest = plugins_dir.join("ViaVersion.jar");
    std::fs::write(&target_dest, "version 1 contents").unwrap();

    let staging_file = plugins_dir.join(".staging.jar");
    std::fs::write(&staging_file, "version 2 new contents").unwrap();

    let lock_holder = std::sync::Arc::new(std::sync::Mutex::new(None));
    let lock_clone = lock_holder.clone();

    // Call production publication function where commit_tracking fails AND locks target_dest
    // so that std::fs::remove_file(&target_dest) in rollback fails
    let res = super::service::publish_installed_plugin(
        &staging_file,
        &target_dest,
        "ViaVersion",
        || {
            // Lock target_dest so removal fails during rollback
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::fs::OpenOptionsExt;
                let locked = std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .share_mode(0)
                    .open(&target_dest)
                    .unwrap();
                *lock_clone.lock().unwrap() = Some(locked);
                // Deliberately fail tracking
                Err(super::models::PluginError::Other("tracking disk full".into()))
            }
            #[cfg(not(target_os = "windows"))]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&plugins_dir, std::fs::Permissions::from_mode(0o555));
                Err(super::models::PluginError::Other("tracking disk full".into()))
            }
        },
        &[],
    );

    #[cfg(target_os = "windows")]
    drop(lock_holder);

    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&plugins_dir, std::fs::Permissions::from_mode(0o755));
    }

    assert!(res.is_err());
    let err = res.unwrap_err();
    // Must report PublicationRollbackFailed, identify failed rollback, and report retained backup
    assert!(matches!(err, super::models::PluginError::PublicationRollbackFailed { .. }));
    let err_msg = err.to_string();
    assert!(err_msg.contains("Rollback failed"));
    assert!(err_msg.contains("Previous version is retained at"));

    // Verify backup file was retained on disk
    let backups: Vec<_> = std::fs::read_dir(&plugins_dir)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(".ViaVersion.jar.backup"))
        .collect();
    assert_eq!(backups.len(), 1, "Backup must be retained when rollback fails");
    assert_eq!(
        std::fs::read_to_string(backups[0].path()).unwrap(),
        "version 1 contents"
    );
}

/// Hits the real APIs: cargo test -- --ignored plugins_live
#[tokio::test]
#[ignore]
async fn plugins_live() {
    for source in [PluginSource::Modrinth, PluginSource::Hangar] {
        let res = search(
            &ServerCoreType::Paper,
            "26.2",
            source,
            "luckperms",
            "",
            true,
            0,
            5,
        )
        .await
        .unwrap();
        println!(
            "{source:?}: {} results, first = {:?}",
            res.total,
            res.items.first().map(|p| &p.title)
        );
        let first = &res.items[0];
        let vs = versions(&ServerCoreType::Paper, "26.2", source, &first.id, true)
            .await
            .unwrap();
        println!(
            "  {} versions, best = {:?}",
            vs.len(),
            pick_version(&vs).map(|v| (&v.version_number, &v.file_name))
        );
    }
}
