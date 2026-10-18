use super::super::super::{
    models::{PluginError, PluginSource, TrackedPlugin},
    tracking::{read_tracking_raw, write_tracking_typed},
};
use super::*;
use crate::server::config::ServerCoreType;
use std::{
    fs,
    path::{Path, PathBuf},
};

fn setup_server(root: &Path) -> (PathBuf, PathBuf) {
    let server_dir = root.to_path_buf();
    let plugins_dir = server_dir.join("plugins");
    fs::create_dir_all(&plugins_dir).unwrap();
    fs::create_dir_all(server_dir.join(".ingot")).unwrap();
    (server_dir, plugins_dir)
}

fn sample_plugin(id: &str, file_name: &str) -> TrackedPlugin {
    TrackedPlugin {
        file_name: file_name.to_string(),
        source: PluginSource::Modrinth,
        project_id: id.to_string(),
        version_id: format!("{id}-v1"),
        version_number: "1.0.0".to_string(),
        title: id.to_string(),
        icon_url: None,
    }
}

#[test]
fn publication_success_updates_tracking_and_cleans_journal() {
    let root = tempfile::tempdir().unwrap();
    let (server_dir, plugins_dir) = setup_server(root.path());

    let stage = plugins_dir.join(".staging.jar");
    fs::write(&stage, b"new-plugin-bytes").unwrap();
    let target = plugins_dir.join("plugin-a.jar");
    let target_tracking = vec![sample_plugin("plugin-a", "plugin-a.jar")];

    publish_plugin_transaction(
        &server_dir,
        &ServerCoreType::Paper,
        &[(stage, target.clone())],
        &[],
        &target_tracking,
    )
    .unwrap();

    assert!(target.exists());
    assert_eq!(fs::read(&target).unwrap(), b"new-plugin-bytes");
    assert!(!journal_path(&server_dir).exists());
    let tracked = read_tracking_raw(&server_dir).unwrap();
    assert_eq!(tracked, target_tracking);
}

#[test]
fn crash_before_tracking_commit_rolls_back_and_removes_journal() {
    let root = tempfile::tempdir().unwrap();
    let (server_dir, plugins_dir) = setup_server(root.path());

    let original = plugins_dir.join("plugin-a.jar");
    fs::write(&original, b"original-bytes").unwrap();
    let prev_tracking = vec![sample_plugin("plugin-a", "plugin-a.jar")];
    write_tracking_typed(&server_dir, &prev_tracking).unwrap();

    // Prepare journal simulating crash before tracking commit:
    // original was moved to backup, new replacement was written, but tracking was not updated.
    let backup_name = format!(".plugin.backup.{}", uuid::Uuid::new_v4());
    let backup = plugins_dir.join(&backup_name);
    fs::rename(&original, &backup).unwrap();

    let replacement = plugins_dir.join("plugin-a.jar");
    fs::write(&replacement, b"replacement-bytes").unwrap();

    let new_tracking = vec![TrackedPlugin {
        version_number: "2.0.0".to_string(),
        ..sample_plugin("plugin-a", "plugin-a.jar")
    }];

    let journal = PluginJournal {
        version: 2,
        phase: PluginJournalPhase::Prepared,
        replacements: vec![PluginFileReplacement {
            stage: "plugins/.plugin.stage.550e8400-e29b-41d4-a716-446655440001".into(),
            target: "plugins/plugin-a.jar".into(),
        }],
        backups: vec![PluginFileBackup {
            original: "plugins/plugin-a.jar".into(),
            backup: format!("plugins/{backup_name}"),
        }],
        previous_tracking: prev_tracking.clone(),
        target_tracking: new_tracking,
    };
    write_journal(&server_dir, &journal).unwrap();

    // Reconstruct / recover
    recover_plugins(&server_dir).unwrap();

    // Verification: replacement deleted, original restored, previous tracking intact, journal deleted
    assert!(original.exists());
    assert_eq!(fs::read(&original).unwrap(), b"original-bytes");
    assert!(!backup.exists());
    assert!(!journal_path(&server_dir).exists());
    assert_eq!(read_tracking_raw(&server_dir).unwrap(), prev_tracking);
}

#[test]
fn crash_after_tracking_commit_finalizes_and_removes_journal() {
    let root = tempfile::tempdir().unwrap();
    let (server_dir, plugins_dir) = setup_server(root.path());

    let backup_name = format!(".plugin.backup.{}", uuid::Uuid::new_v4());
    let backup = plugins_dir.join(&backup_name);
    fs::write(&backup, b"old-backup-bytes").unwrap();

    let target = plugins_dir.join("plugin-a.jar");
    fs::write(&target, b"new-bytes").unwrap();

    let target_tracking = vec![sample_plugin("plugin-a", "plugin-a.jar")];
    // Tracking was committed
    write_tracking_typed(&server_dir, &target_tracking).unwrap();

    let journal = PluginJournal {
        version: 2,
        phase: PluginJournalPhase::Committed,
        replacements: vec![PluginFileReplacement {
            stage: "plugins/.plugin.stage.550e8400-e29b-41d4-a716-446655440001".into(),
            target: "plugins/plugin-a.jar".into(),
        }],
        backups: vec![PluginFileBackup {
            original: "plugins/plugin-a.jar".into(),
            backup: format!("plugins/{backup_name}"),
        }],
        previous_tracking: vec![],
        target_tracking: target_tracking.clone(),
    };
    write_journal(&server_dir, &journal).unwrap();

    // Recover
    recover_plugins(&server_dir).unwrap();

    // Verification: target remains, backup cleaned up, journal deleted
    assert!(target.exists());
    assert_eq!(fs::read(&target).unwrap(), b"new-bytes");
    assert!(!backup.exists());
    assert!(!journal_path(&server_dir).exists());
    assert_eq!(read_tracking_raw(&server_dir).unwrap(), target_tracking);
}

#[test]
fn restoration_is_repeatable_until_acknowledgement() {
    let root = tempfile::tempdir().unwrap();
    let (server_dir, plugins_dir) = setup_server(root.path());

    let original = plugins_dir.join("plugin-a.jar");
    fs::write(&original, b"some-file").unwrap();
    let backup = plugins_dir.join(".plugin.backup.550e8400-e29b-41d4-a716-446655440000");
    fs::write(&backup, b"backup-file").unwrap();

    let journal = PluginJournal {
        version: 2,
        phase: PluginJournalPhase::Prepared,
        replacements: vec![],
        backups: vec![PluginFileBackup {
            original: "plugins/plugin-a.jar".into(),
            backup: "plugins/.plugin.backup.550e8400-e29b-41d4-a716-446655440000".into(),
        }],
        previous_tracking: vec![],
        target_tracking: vec![sample_plugin("plugin-a", "plugin-a.jar")],
    };
    write_journal(&server_dir, &journal).unwrap();

    recover_plugins(&server_dir).unwrap();
    assert_eq!(fs::read(&original).unwrap(), b"backup-file");
    assert!(!backup.exists());
    assert!(!journal_path(&server_dir).exists());
}

#[test]
fn corrupt_journal_blocks_recovery() {
    let root = tempfile::tempdir().unwrap();
    let (server_dir, _) = setup_server(root.path());
    let jpath = journal_path(&server_dir);
    fs::write(&jpath, b"{ not valid json").unwrap();

    let result = recover_plugins(&server_dir);
    assert!(matches!(result, Err(PluginError::Json(_))));
    assert!(jpath.exists());
}

#[test]
fn unknown_journal_version_blocks_recovery() {
    let root = tempfile::tempdir().unwrap();
    let (server_dir, _) = setup_server(root.path());

    let journal = PluginJournal {
        version: 999,
        phase: PluginJournalPhase::Prepared,
        replacements: vec![],
        backups: vec![],
        previous_tracking: vec![],
        target_tracking: vec![],
    };
    write_journal(&server_dir, &journal).unwrap();

    let result = recover_plugins(&server_dir);
    assert!(matches!(
        result,
        Err(PluginError::RecoveryBlocked(
            "Invalid plugin journal version"
        ))
    ));
    assert!(journal_path(&server_dir).exists());
}

#[test]
fn path_traversal_in_journal_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let (server_dir, _) = setup_server(root.path());

    let journal = PluginJournal {
        version: 2,
        phase: PluginJournalPhase::Prepared,
        replacements: vec![PluginFileReplacement {
            stage: "plugins/.plugin.stage.550e8400-e29b-41d4-a716-446655440001".into(),
            target: "../outside.jar".into(),
        }],
        backups: vec![],
        previous_tracking: vec![],
        target_tracking: vec![],
    };
    write_journal(&server_dir, &journal).unwrap();

    let result = recover_plugins(&server_dir);
    assert!(matches!(
        result,
        Err(PluginError::RecoveryBlocked("Unsafe plugin recovery path"))
    ));
}

#[test]
fn repeated_recovery_is_idempotent_and_preserves_unrelated_data() {
    let root = tempfile::tempdir().unwrap();
    let (server_dir, plugins_dir) = setup_server(root.path());

    let unrelated = plugins_dir.join("other-plugin.jar");
    fs::write(&unrelated, b"unrelated-data").unwrap();

    // Call recover on clean server
    recover_plugins(&server_dir).unwrap();
    assert_eq!(fs::read(&unrelated).unwrap(), b"unrelated-data");

    // Call recover again
    recover_plugins(&server_dir).unwrap();
    assert_eq!(fs::read(&unrelated).unwrap(), b"unrelated-data");
}

#[test]
fn external_staging_handled_cleanly_and_caller_cleanup_safe() {
    let root = tempfile::tempdir().unwrap();
    let (server_dir, plugins_dir) = setup_server(root.path());

    let external_temp = tempfile::tempdir().unwrap();
    let external_stage = external_temp.path().join("downloaded.jar");
    fs::write(&external_stage, b"external-plugin-bytes").unwrap();

    let target = plugins_dir.join("plugin-b.jar");
    let target_tracking = vec![sample_plugin("plugin-b", "plugin-b.jar")];

    publish_plugin_transaction(
        &server_dir,
        &ServerCoreType::Paper,
        &[(external_stage.clone(), target.clone())],
        &[],
        &target_tracking,
    )
    .unwrap();

    // Caller drops external_temp
    drop(external_temp);

    // Target is safely installed in server_dir
    assert!(target.exists());
    assert_eq!(fs::read(&target).unwrap(), b"external-plugin-bytes");
}

#[test]
fn prepared_equal_tracking_does_not_delete_untouched_original() {
    let root = tempfile::tempdir().unwrap();
    let (server, plugins) = setup_server(root.path());
    let original = plugins.join("plugin-a.jar");
    fs::write(&original, b"working original").unwrap();
    let tracking = vec![sample_plugin("plugin-a", "plugin-a.jar")];
    write_tracking_typed(&server, &tracking).unwrap();
    let journal = PluginJournal {
        version: 2,
        phase: PluginJournalPhase::Prepared,
        replacements: vec![PluginFileReplacement {
            stage: "plugins/.plugin.stage.550e8400-e29b-41d4-a716-446655440001".into(),
            target: "plugins/plugin-a.jar".into(),
        }],
        backups: vec![PluginFileBackup {
            original: "plugins/plugin-a.jar".into(),
            backup: "plugins/.plugin.backup.550e8400-e29b-41d4-a716-446655440000".into(),
        }],
        previous_tracking: tracking.clone(),
        target_tracking: tracking.clone(),
    };
    write_journal(&server, &journal).unwrap();
    recover_plugins(&server).unwrap();
    assert_eq!(fs::read(original).unwrap(), b"working original");
    assert_eq!(read_tracking_raw(&server).unwrap(), tracking);
}

#[test]
fn interrupted_restoration_can_repeat_without_deleting_original() {
    let root = tempfile::tempdir().unwrap();
    let (server, plugins) = setup_server(root.path());
    let original = plugins.join("plugin-a.jar");
    let backup = plugins.join(".plugin.backup.550e8400-e29b-41d4-a716-446655440000");
    fs::write(&original, b"restored original").unwrap();
    fs::write(&backup, b"restored original").unwrap();
    let journal = PluginJournal {
        version: 2,
        phase: PluginJournalPhase::Prepared,
        replacements: vec![PluginFileReplacement {
            stage: "plugins/.plugin.stage.550e8400-e29b-41d4-a716-446655440001".into(),
            target: "plugins/plugin-a.jar".into(),
        }],
        backups: vec![PluginFileBackup {
            original: "plugins/plugin-a.jar".into(),
            backup: "plugins/.plugin.backup.550e8400-e29b-41d4-a716-446655440000".into(),
        }],
        previous_tracking: vec![],
        target_tracking: vec![],
    };
    write_journal(&server, &journal).unwrap();
    recover_plugins(&server).unwrap();
    recover_plugins(&server).unwrap();
    assert_eq!(fs::read(original).unwrap(), b"restored original");
}

#[test]
fn corrupt_tracking_blocks_recovery_and_preserves_original() {
    let root = tempfile::tempdir().unwrap();
    let (server, plugins) = setup_server(root.path());
    let original = plugins.join("plugin-a.jar");
    fs::write(&original, b"working original").unwrap();
    fs::write(
        super::super::super::tracking::tracking_path(&server),
        b"corrupt",
    )
    .unwrap();
    let stage = plugins.join("stage");
    fs::write(&stage, b"new").unwrap();
    assert!(publish_plugin_transaction(
        &server,
        &ServerCoreType::Paper,
        &[(stage, original.clone())],
        &[],
        &[]
    )
    .is_err());
    assert_eq!(fs::read(original).unwrap(), b"working original");
}

#[test]
fn tracking_reads_surface_recovery_failure_and_preserve_journal() {
    let root = tempfile::tempdir().unwrap();
    let (server, _) = setup_server(root.path());
    fs::write(journal_path(&server), b"corrupt journal").unwrap();
    let result = super::super::super::tracking::read_tracking(&server);
    assert!(matches!(
        result,
        Err(super::super::super::tracking::TrackingError::Recovery(_))
    ));
    assert_eq!(fs::read(journal_path(&server)).unwrap(), b"corrupt journal");
}

#[test]
fn tracking_reader_waits_for_publication_owner() {
    let root = tempfile::tempdir().unwrap();
    let (server, _) = setup_server(root.path());
    let lock = server_lock(&server).unwrap();
    let guard = lock.lock().unwrap();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        started_tx.send(()).unwrap();
        done_tx
            .send(super::super::super::tracking::read_tracking(&server))
            .unwrap();
    });
    started_rx.recv().unwrap();
    assert!(matches!(
        done_rx.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Empty)
    ));
    drop(guard);
    assert!(done_rx.recv().unwrap().unwrap().is_empty());
    reader.join().unwrap();
}

#[test]
fn every_precommit_batch_boundary_restores_all_originals() {
    for completed_steps in 0..=6 {
        let root = tempfile::tempdir().unwrap();
        let (server, plugins) = setup_server(root.path());
        let original_names = ["a.jar", "b.jar", "removed.jar.disabled"];
        for name in original_names {
            fs::write(plugins.join(name), name.as_bytes()).unwrap();
        }
        let previous = vec![sample_plugin("a", "a.jar"), sample_plugin("b", "b.jar")];
        let target = vec![
            sample_plugin("new-a", "a.jar"),
            sample_plugin("new-b", "b.jar"),
        ];
        write_tracking_typed(&server, &previous).unwrap();
        let backups = original_names
            .iter()
            .map(|name| PluginFileBackup {
                original: format!("plugins/{name}"),
                backup: format!("plugins/.plugin.backup.{}", uuid::Uuid::new_v4()),
            })
            .collect::<Vec<_>>();
        let replacements = ["a.jar", "b.jar"]
            .iter()
            .map(|name| {
                let stage = format!(".plugin.stage.{}", uuid::Uuid::new_v4());
                fs::write(plugins.join(&stage), b"new").unwrap();
                PluginFileReplacement {
                    stage: format!("plugins/{stage}"),
                    target: format!("plugins/{name}"),
                }
            })
            .collect::<Vec<_>>();
        let journal = PluginJournal {
            version: 2,
            phase: PluginJournalPhase::Prepared,
            replacements,
            backups,
            previous_tracking: previous.clone(),
            target_tracking: target.clone(),
        };
        write_journal(&server, &journal).unwrap();
        for (index, backup) in journal.backups.iter().enumerate() {
            if completed_steps > index {
                fs::rename(server.join(&backup.original), server.join(&backup.backup)).unwrap();
            }
        }
        for (index, replacement) in journal.replacements.iter().enumerate() {
            if completed_steps > index + 3 {
                fs::rename(
                    server.join(&replacement.stage),
                    server.join(&replacement.target),
                )
                .unwrap();
            }
        }
        if completed_steps == 6 {
            write_tracking_typed(&server, &target).unwrap();
        }
        recover_plugins(&server).unwrap();
        recover_plugins(&server).unwrap();
        for name in original_names {
            assert_eq!(
                fs::read(plugins.join(name)).unwrap(),
                name.as_bytes(),
                "boundary {completed_steps}"
            );
        }
        assert_eq!(read_tracking_raw(&server).unwrap(), previous);
    }
}

#[cfg(unix)]
#[test]
fn linked_metadata_directory_cannot_redirect_recovery_or_tracking() {
    let server = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("plugins.json"), b"[]").unwrap();
    std::os::unix::fs::symlink(outside.path(), server.path().join(".ingot")).unwrap();
    assert!(matches!(
        recover_plugins(server.path()),
        Err(PluginError::RecoveryBlocked(_))
    ));
    assert!(super::super::super::tracking::read_tracking(server.path()).is_err());
    assert_eq!(
        fs::read(outside.path().join("plugins.json")).unwrap(),
        b"[]"
    );
}

#[test]
fn receipt_cannot_remove_an_arbitrary_contained_user_file() {
    let root = tempfile::tempdir().unwrap();
    let (server, _) = setup_server(root.path());
    let user_file = server.join("server.properties");
    fs::write(&user_file, b"keep").unwrap();
    let journal = PluginJournal {
        version: 2,
        phase: PluginJournalPhase::Prepared,
        replacements: vec![PluginFileReplacement {
            stage: "plugins/.plugin.stage.550e8400-e29b-41d4-a716-446655440001".into(),
            target: "server.properties".into(),
        }],
        backups: vec![],
        previous_tracking: vec![],
        target_tracking: vec![],
    };
    write_journal(&server, &journal).unwrap();
    assert!(matches!(
        recover_plugins(&server),
        Err(PluginError::RecoveryBlocked(_))
    ));
    assert_eq!(fs::read(user_file).unwrap(), b"keep");
}

#[test]
fn malformed_journal_tracking_blocks_all_file_mutations() {
    let root = tempfile::tempdir().unwrap();
    let (server, plugins) = setup_server(root.path());
    let target = plugins.join("new.jar");
    fs::write(&target, b"keep until receipt validated").unwrap();
    let journal = PluginJournal {
        version: 2,
        phase: PluginJournalPhase::Prepared,
        replacements: vec![PluginFileReplacement {
            stage: "plugins/.plugin.stage.550e8400-e29b-41d4-a716-446655440001".into(),
            target: "plugins/new.jar".into(),
        }],
        backups: vec![],
        previous_tracking: vec![sample_plugin("", "new.jar")],
        target_tracking: vec![],
    };
    write_journal(&server, &journal).unwrap();
    assert!(matches!(
        recover_plugins(&server),
        Err(PluginError::RecoveryBlocked(_))
    ));
    assert_eq!(fs::read(target).unwrap(), b"keep until receipt validated");
}

#[cfg(unix)]
#[test]
fn linked_plugin_target_blocks_recovery_without_touching_external_file() {
    let root = tempfile::tempdir().unwrap();
    let (server, plugins) = setup_server(root.path());
    let outside = tempfile::tempdir().unwrap();
    let external = outside.path().join("external.jar");
    fs::write(&external, b"external").unwrap();
    std::os::unix::fs::symlink(&external, plugins.join("plugin-a.jar")).unwrap();
    let journal = PluginJournal {
        version: 2,
        phase: PluginJournalPhase::Prepared,
        replacements: vec![PluginFileReplacement {
            stage: "plugins/.plugin.stage.550e8400-e29b-41d4-a716-446655440001".into(),
            target: "plugins/plugin-a.jar".into(),
        }],
        backups: vec![],
        previous_tracking: vec![],
        target_tracking: vec![],
    };
    write_journal(&server, &journal).unwrap();
    assert!(matches!(
        recover_plugins(&server),
        Err(PluginError::RecoveryBlocked(_))
    ));
    assert_eq!(fs::read(external).unwrap(), b"external");
}
