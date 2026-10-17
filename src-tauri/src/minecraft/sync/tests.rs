use super::execute::{
    backup_file_if_exists, inspect_shared_status, sync_file_bidirectional, sync_resourcepacks_dir,
};
use super::models::{ConflictResolution, SyncCategory};
use super::policy::{
    decide_bidirectional_sync_action, evaluate_conflict, resolve_effective_sync,
    BidirectionalSyncAction, ConflictCheckInputs, EffectiveSync,
};
use super::service::{
    execute_sync_after_exit, execute_sync_before_launch, export_instance_category_files,
    import_shared_files_to_instance,
};
use crate::minecraft::instance::InstanceConfig;
use crate::settings::SyncSettings;
use std::fs;
use std::time::{Duration, UNIX_EPOCH};
use tempfile::TempDir;

fn create_test_instance_config() -> InstanceConfig {
    InstanceConfig {
        id: "test-instance".to_string(),
        name: "Test Instance".to_string(),
        game_version: "1.20.1".to_string(),
        loader: crate::minecraft::instance::ModLoaderType::Vanilla,
        loader_version: None,
        java_path: None,
        memory_min_mb: None,
        memory_max_mb: None,
        icon: None,
        created_at: 0,
        last_played: None,
        total_play_time_seconds: 0,
        jvm_args: None,
        fullscreen: None,
        window_width: None,
        window_height: None,
        sync_options: None,
        sync_servers: None,
        sync_resource_packs: None,
        sync_command_history: None,
        sync_creative_hotbars: None,
        last_synced_at: None,
    }
}

#[test]
fn test_resolve_effective_sync_precedence() {
    let global = SyncSettings {
        sync_options: true,
        sync_servers: false,
        sync_resource_packs: true,
        sync_command_history: false,
        sync_creative_hotbars: true,
        initialized_categories: vec![],
    };

    let mut instance = create_test_instance_config();
    // Inherit from global
    let eff = resolve_effective_sync(&instance, &global);
    assert!(eff.options);
    assert!(!eff.servers);
    assert!(eff.resource_packs);
    assert!(!eff.command_history);
    assert!(eff.creative_hotbars);
    assert!(eff.has_any_enabled());

    // Per-instance overrides
    instance.sync_options = Some(false);
    instance.sync_servers = Some(true);
    let eff_override = resolve_effective_sync(&instance, &global);
    assert!(!eff_override.options);
    assert!(eff_override.servers);

    // All false
    let all_disabled = EffectiveSync::default();
    assert!(!all_disabled.has_any_enabled());
}

#[test]
fn test_decide_bidirectional_sync_action() {
    let t1 = UNIX_EPOCH + Duration::from_secs(100);
    let t2 = UNIX_EPOCH + Duration::from_secs(200);

    // One side missing
    assert_eq!(
        decide_bidirectional_sync_action(true, false, t1, t1),
        BidirectionalSyncAction::CopySharedToInstance {
            backup_instance: false
        }
    );
    assert_eq!(
        decide_bidirectional_sync_action(false, true, t1, t1),
        BidirectionalSyncAction::CopyInstanceToShared {
            backup_shared: false
        }
    );
    assert_eq!(
        decide_bidirectional_sync_action(false, false, t1, t1),
        BidirectionalSyncAction::DoNothing
    );

    // Both exist - shared newer
    assert_eq!(
        decide_bidirectional_sync_action(true, true, t2, t1),
        BidirectionalSyncAction::CopySharedToInstance {
            backup_instance: true
        }
    );

    // Both exist - instance newer
    assert_eq!(
        decide_bidirectional_sync_action(true, true, t1, t2),
        BidirectionalSyncAction::CopyInstanceToShared {
            backup_shared: true
        }
    );

    // Both exist - equal timestamps
    assert_eq!(
        decide_bidirectional_sync_action(true, true, t1, t1),
        BidirectionalSyncAction::DoNothing
    );
}

#[test]
fn test_evaluate_conflict_rules() {
    let base_inputs = ConflictCheckInputs {
        has_last_synced: false,
        effective: EffectiveSync {
            options: true,
            servers: true,
            resource_packs: false,
            command_history: false,
            creative_hotbars: false,
        },
        has_instance_options: true,
        has_instance_servers: false,
        has_shared_options: true,
        has_shared_servers: false,
    };

    // Options conflict detected
    assert!(evaluate_conflict(&base_inputs));

    // If instance was already synced previously, conflict is skipped
    let mut already_synced = base_inputs.clone();
    already_synced.has_last_synced = true;
    assert!(!evaluate_conflict(&already_synced));

    // If options not enabled, no options conflict
    let mut options_disabled = base_inputs.clone();
    options_disabled.effective.options = false;
    assert!(!evaluate_conflict(&options_disabled));

    // Servers conflict detected when servers exist on both sides
    let mut servers_conflict = base_inputs.clone();
    servers_conflict.effective.options = false;
    servers_conflict.has_instance_servers = true;
    servers_conflict.has_shared_servers = true;
    assert!(evaluate_conflict(&servers_conflict));

    // If shared file doesn't exist, no conflict
    let mut no_shared = base_inputs.clone();
    no_shared.has_shared_options = false;
    assert!(!evaluate_conflict(&no_shared));
}

#[test]
fn test_backup_file_if_exists() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("options.txt");
    fs::write(&file, "fov:70").unwrap();

    backup_file_if_exists(&file).unwrap();

    let backup = temp.path().join("options.txt.bak");
    assert!(backup.exists());
    assert_eq!(fs::read_to_string(&backup).unwrap(), "fov:70");
    assert_eq!(fs::read_to_string(&file).unwrap(), "fov:70");

    // Non-existent file does not fail or create .bak
    let missing = temp.path().join("missing.txt");
    backup_file_if_exists(&missing).unwrap();
    assert!(!temp.path().join("missing.txt.bak").exists());
}

#[test]
fn test_sync_file_bidirectional_missing_on_one_side() {
    let temp = TempDir::new().unwrap();
    let shared = temp.path().join("shared.txt");
    let instance = temp.path().join("instance.txt");

    // Shared exists, instance missing -> copies shared to instance
    fs::write(&shared, "shared_content").unwrap();
    sync_file_bidirectional(&shared, &instance).unwrap();
    assert_eq!(fs::read_to_string(&instance).unwrap(), "shared_content");

    // Instance updated, shared removed -> copies instance to shared
    fs::remove_file(&shared).unwrap();
    fs::write(&instance, "instance_content").unwrap();
    sync_file_bidirectional(&shared, &instance).unwrap();
    assert_eq!(fs::read_to_string(&shared).unwrap(), "instance_content");
}

#[test]
fn test_sync_resourcepacks_dir() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src_packs");
    let dst = temp.path().join("dst_packs");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    // Create a loose pack file and a pack folder in src
    fs::write(src.join("pack1.zip"), "zip_data").unwrap();
    let folder_pack = src.join("folder_pack");
    fs::create_dir_all(&folder_pack).unwrap();
    fs::write(folder_pack.join("pack.mcmeta"), "meta").unwrap();

    // Create an existing pack in dst with different content to ensure it is not overwritten
    fs::write(dst.join("pack1.zip"), "existing_data").unwrap();

    let copied = sync_resourcepacks_dir(&src, &dst).unwrap();
    // Only folder_pack should be copied because pack1.zip already existed in dst
    assert_eq!(copied, 1);
    assert_eq!(
        fs::read_to_string(dst.join("pack1.zip")).unwrap(),
        "existing_data"
    );
    assert!(dst.join("folder_pack").join("pack.mcmeta").exists());
}

#[test]
fn test_execute_sync_before_launch_and_after_exit() {
    let temp = TempDir::new().unwrap();
    let shared = temp.path().join("shared");
    let instance = temp.path().join("instance");
    fs::create_dir_all(&shared).unwrap();
    fs::create_dir_all(&instance).unwrap();

    fs::write(shared.join("options.txt"), "shared_options").unwrap();
    fs::write(shared.join("servers.dat"), "shared_servers").unwrap();

    let eff = EffectiveSync {
        options: true,
        servers: true,
        resource_packs: false,
        command_history: false,
        creative_hotbars: false,
    };

    // Pre-launch sync: copies shared files to instance since instance files were missing
    execute_sync_before_launch(&shared, &instance, &eff).unwrap();
    assert_eq!(
        fs::read_to_string(instance.join("options.txt")).unwrap(),
        "shared_options"
    );
    assert_eq!(
        fs::read_to_string(instance.join("servers.dat")).unwrap(),
        "shared_servers"
    );

    // Modify instance options and run post-exit sync
    fs::write(instance.join("options.txt"), "modified_options").unwrap();
    execute_sync_after_exit(&shared, &instance, &eff).unwrap();
    assert_eq!(
        fs::read_to_string(shared.join("options.txt")).unwrap(),
        "modified_options"
    );
    // Backup of old shared file was created
    assert_eq!(
        fs::read_to_string(shared.join("options.txt.bak")).unwrap(),
        "shared_options"
    );
}

#[test]
fn test_sync_failed_backup_prevents_overwrite() {
    let temp = TempDir::new().unwrap();
    let shared = temp.path().join("shared");
    let instance = temp.path().join("instance");
    fs::create_dir_all(&shared).unwrap();
    fs::create_dir_all(&instance).unwrap();

    let shared_options = shared.join("options.txt");
    let instance_options = instance.join("options.txt");
    fs::write(&instance_options, "original_instance_content").unwrap();
    fs::write(&shared_options, "new_shared_content").unwrap();
    let older = UNIX_EPOCH + Duration::from_secs(100);
    let newer = UNIX_EPOCH + Duration::from_secs(200);
    fs::File::options()
        .write(true)
        .open(&instance_options)
        .unwrap()
        .set_modified(older)
        .unwrap();
    fs::File::options()
        .write(true)
        .open(&shared_options)
        .unwrap()
        .set_modified(newer)
        .unwrap();

    // Make instance options.txt.bak a directory so backup fails
    let instance_bak = instance.join("options.txt.bak");
    fs::create_dir_all(&instance_bak).unwrap();

    // Calling sync_file_bidirectional must fail because backup cannot be written
    let res = sync_file_bidirectional(&shared_options, &instance_options);
    assert!(res.is_err(), "Expected sync to fail because backup failed");

    // Original content must NOT have been overwritten
    assert_eq!(
        fs::read_to_string(&instance_options).unwrap(),
        "original_instance_content"
    );

    // Also test execute_sync_after_exit failure
    let shared_bak = shared.join("options.txt.bak");
    fs::create_dir_all(&shared_bak).unwrap();

    let eff = EffectiveSync {
        options: true,
        servers: false,
        resource_packs: false,
        command_history: false,
        creative_hotbars: false,
    };
    let exit_res = execute_sync_after_exit(&shared, &instance, &eff);
    assert!(
        exit_res.is_err(),
        "Expected post-exit sync to fail because shared backup failed"
    );

    // Shared content must NOT have been overwritten
    assert_eq!(
        fs::read_to_string(&shared_options).unwrap(),
        "new_shared_content"
    );
}

#[test]
fn test_export_instance_category_files() {
    let temp = TempDir::new().unwrap();
    let shared = temp.path().join("shared");
    let instance = temp.path().join("instance");
    fs::create_dir_all(&shared).unwrap();
    fs::create_dir_all(&instance).unwrap();

    fs::write(instance.join("options.txt"), "instance_options").unwrap();
    fs::write(instance.join("servers.dat"), "instance_servers").unwrap();
    fs::write(instance.join("command_history.txt"), "history").unwrap();
    fs::write(instance.join("hotbar.nbt"), "hotbar").unwrap();

    let packs_dir = instance.join("resourcepacks");
    fs::create_dir_all(&packs_dir).unwrap();
    fs::write(packs_dir.join("pack.zip"), "pack").unwrap();

    // Export individual category
    let report_opts =
        export_instance_category_files(&shared, &instance, SyncCategory::Options).unwrap();
    assert!(report_opts.options_synced);
    assert!(!report_opts.servers_synced);
    assert_eq!(
        fs::read_to_string(shared.join("options.txt")).unwrap(),
        "instance_options"
    );

    // Export all
    let report_all = export_instance_category_files(&shared, &instance, SyncCategory::All).unwrap();
    assert!(report_all.options_synced);
    assert!(report_all.servers_synced);
    assert!(report_all.command_history_synced);
    assert!(report_all.creative_hotbars_synced);
    assert_eq!(report_all.resource_packs_count, 1);
}

#[test]
fn test_import_shared_files_to_instance() {
    let temp = TempDir::new().unwrap();
    let shared = temp.path().join("shared");
    let instance = temp.path().join("instance");
    fs::create_dir_all(&shared).unwrap();
    fs::create_dir_all(&instance).unwrap();

    fs::write(shared.join("options.txt"), "shared_options").unwrap();
    fs::write(shared.join("servers.dat"), "shared_servers").unwrap();
    fs::write(instance.join("options.txt"), "old_instance_options").unwrap();

    let report = import_shared_files_to_instance(&shared, &instance).unwrap();
    assert!(report.options_synced);
    assert!(report.servers_synced);
    assert_eq!(
        fs::read_to_string(instance.join("options.txt")).unwrap(),
        "shared_options"
    );
    // Old instance options backed up
    assert_eq!(
        fs::read_to_string(instance.join("options.txt.bak")).unwrap(),
        "old_instance_options"
    );
}

#[test]
fn test_inspect_shared_status() {
    let temp = TempDir::new().unwrap();
    let shared = temp.path();

    let initial = inspect_shared_status(shared).unwrap();
    assert!(!initial.has_options);
    assert!(!initial.has_servers);
    assert!(!initial.has_command_history);
    assert!(!initial.has_creative_hotbars);
    assert_eq!(initial.resource_packs_count, 0);
    assert_eq!(initial.last_synced_timestamp, None);

    fs::write(shared.join("options.txt"), "opt").unwrap();
    let packs = shared.join("resourcepacks");
    fs::create_dir_all(&packs).unwrap();
    fs::write(packs.join("pack1.zip"), "zip").unwrap();
    fs::write(packs.join("pack2.zip"), "zip").unwrap();

    let updated = inspect_shared_status(shared).unwrap();
    assert!(updated.has_options);
    assert!(!updated.has_servers);
    assert_eq!(updated.resource_packs_count, 2);
    assert!(updated.last_synced_timestamp.is_some());
}

#[test]
fn test_category_and_resolution_parsing() {
    assert_eq!(SyncCategory::parse("all"), Some(SyncCategory::All));
    assert_eq!(SyncCategory::parse("options"), Some(SyncCategory::Options));
    assert_eq!(SyncCategory::parse("servers"), Some(SyncCategory::Servers));
    assert_eq!(
        SyncCategory::parse("command_history"),
        Some(SyncCategory::CommandHistory)
    );
    assert_eq!(SyncCategory::parse("hotbar"), Some(SyncCategory::Hotbar));
    assert_eq!(
        SyncCategory::parse("resourcepacks"),
        Some(SyncCategory::ResourcePacks)
    );
    assert_eq!(SyncCategory::parse("unknown"), None);

    assert_eq!(
        ConflictResolution::parse("use_shared"),
        Some(ConflictResolution::UseShared)
    );
    assert_eq!(
        ConflictResolution::parse("use_instance"),
        Some(ConflictResolution::UseInstance)
    );
    assert_eq!(
        ConflictResolution::parse("disable_sync"),
        Some(ConflictResolution::DisableSync)
    );
    assert_eq!(ConflictResolution::parse("invalid"), None);
}

#[test]
fn test_sync_resourcepacks_dir_propagates_copy_failure() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src_packs");
    let dst = temp.path().join("dst_packs");
    fs::create_dir_all(&src).unwrap();
    // dst is a file, so copying a file into dst/pack1.zip fails
    fs::write(&dst, "blocking file").unwrap();

    fs::write(src.join("pack1.zip"), "meta").unwrap();

    let res = sync_resourcepacks_dir(&src, &dst);
    assert!(
        res.is_err(),
        "sync_resourcepacks_dir must propagate copy failures when destination cannot be written"
    );
}

#[test]
fn test_sync_service_resourcepacks_propagates_error() {
    let temp = TempDir::new().unwrap();
    let shared = temp.path().join("shared");
    let inst = temp.path().join("instance");
    fs::create_dir_all(&shared).unwrap();
    fs::create_dir_all(&inst).unwrap();

    let shared_packs = shared.join("resourcepacks");
    fs::create_dir_all(&shared_packs).unwrap();
    fs::write(shared_packs.join("pack.zip"), "data").unwrap();

    // Block destination resourcepacks folder by creating a file instead of a directory
    let inst_packs = inst.join("resourcepacks");
    fs::write(&inst_packs, "blocking file").unwrap();

    let res = import_shared_files_to_instance(&shared, &inst);
    assert!(
        res.is_err(),
        "import_shared_files_to_instance must fail when resourcepack sync fails"
    );
}

#[test]
fn shared_directory_creation_failure_is_not_success() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("resourcepacks"), b"blocking file").unwrap();
    assert!(super::execute::ensure_shared_directories(temp.path()).is_err());
}

#[test]
fn invalid_shared_metadata_is_not_an_empty_status() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join("options.txt")).unwrap();
    assert!(inspect_shared_status(temp.path()).is_err());
}

#[test]
fn resource_pack_source_file_is_not_treated_as_missing() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("instance");
    let shared = temp.path().join("shared");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&shared).unwrap();
    fs::write(source.join("resourcepacks"), b"not a directory").unwrap();
    let result = export_instance_category_files(&shared, &source, SyncCategory::ResourcePacks);
    assert!(matches!(result, Err(super::models::SyncError::Io(_))));
    assert!(!shared.join("resourcepacks/pack.zip").exists());
}

#[cfg(unix)]
#[test]
fn linked_sync_file_is_rejected_without_changing_its_target() {
    let temp = TempDir::new().unwrap();
    let outside = temp.path().join("outside.txt");
    let shared = temp.path().join("shared.txt");
    let instance = temp.path().join("instance.txt");
    fs::write(&outside, b"original").unwrap();
    fs::write(&shared, b"replacement").unwrap();
    std::os::unix::fs::symlink(&outside, &instance).unwrap();
    assert!(sync_file_bidirectional(&shared, &instance).is_err());
    assert_eq!(fs::read(outside).unwrap(), b"original");
}

#[cfg(unix)]
#[test]
fn failed_pack_copy_never_publishes_partial_directory() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    fs::create_dir_all(source.join("pack")).unwrap();
    fs::create_dir_all(&target).unwrap();
    fs::write(source.join("pack/valid.txt"), b"valid").unwrap();
    std::os::unix::fs::symlink("valid.txt", source.join("pack/link.txt")).unwrap();
    assert!(sync_resourcepacks_dir(&source, &target).is_err());
    assert!(!target.join("pack").exists());
    assert_eq!(fs::read_dir(target).unwrap().count(), 0);
    assert_eq!(fs::read(source.join("pack/valid.txt")).unwrap(), b"valid");
}
