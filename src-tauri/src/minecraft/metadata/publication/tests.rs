use super::super::{MetadataError, MetadataRepository};
use super::PublicationReceipt;
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
struct Record {
    id: String,
    val: u32,
}

fn create_repository(root: &Path) -> (MetadataRepository<Record>, PathBuf, PathBuf) {
    let metadata_path = root.join("records.json");
    let directory = root.join("items");
    fs::create_dir_all(&directory).unwrap();
    let repository = MetadataRepository::with_deletion_directory(
        metadata_path.clone(),
        directory.clone(),
        |r: &Record| &r.id,
    );
    (repository, metadata_path, directory)
}

#[test]
fn successful_publication_commits_and_acknowledges() {
    let root = tempfile::tempdir().unwrap();
    let (repo, meta_path, dir) = create_repository(root.path());
    let staging = root.path().join("staging");
    fs::create_dir_all(&staging).unwrap();
    fs::write(staging.join("file.txt"), b"data").unwrap();
    let dest = dir.join("item1");

    repo.publish(
        &staging,
        &dest,
        Record {
            id: "item1".into(),
            val: 1,
        },
        |_, _| Ok(()),
    )
    .unwrap();

    assert!(dest.join("file.txt").exists());
    assert!(!staging.exists());
    assert!(!meta_path.with_extension("publication.json").exists());
    let loaded = repo.load().unwrap();
    assert_eq!(
        loaded,
        vec![Record {
            id: "item1".into(),
            val: 1
        }]
    );
}

#[test]
fn crash_before_metadata_commit_rolls_back_and_acknowledges() {
    let root = tempfile::tempdir().unwrap();
    let (_repo, meta_path, dir) = create_repository(root.path());
    let dest = dir.join("item1");
    fs::create_dir_all(&dest).unwrap();
    fs::write(dest.join("uncommitted.txt"), b"bad").unwrap();

    // Simulate crash before metadata commit: receipt exists, dest exists, records does not contain item1
    let receipt = PublicationReceipt {
        version: 1,
        id: "item1".into(),
        staged: format!(".publishing-{}", uuid::Uuid::new_v4()),
    };
    fs::write(
        meta_path.with_extension("publication.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();

    // Reconstruct new repository
    let new_repo = MetadataRepository::with_deletion_directory(
        meta_path.clone(),
        dir.clone(),
        |r: &Record| &r.id,
    );
    let loaded = new_repo.load().unwrap();
    assert!(loaded.is_empty());
    assert!(!dest.exists());
    assert!(!meta_path.with_extension("publication.json").exists());
}

#[test]
fn crash_after_metadata_commit_finalizes_and_acknowledges() {
    let root = tempfile::tempdir().unwrap();
    let (_repo, meta_path, dir) = create_repository(root.path());
    let dest = dir.join("item1");
    fs::create_dir_all(&dest).unwrap();
    fs::write(dest.join("committed.txt"), b"good").unwrap();

    // Write committed record in metadata
    fs::write(
        &meta_path,
        serde_json::to_vec(&vec![Record {
            id: "item1".into(),
            val: 10,
        }])
        .unwrap(),
    )
    .unwrap();

    // Simulate unacknowledged receipt after crash
    let receipt = PublicationReceipt {
        version: 1,
        id: "item1".into(),
        staged: format!(".publishing-{}", uuid::Uuid::new_v4()),
    };
    fs::write(
        meta_path.with_extension("publication.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();

    // Reconstruct new repository
    let new_repo = MetadataRepository::with_deletion_directory(
        meta_path.clone(),
        dir.clone(),
        |r: &Record| &r.id,
    );
    let loaded = new_repo.load().unwrap();
    assert_eq!(
        loaded,
        vec![Record {
            id: "item1".into(),
            val: 10
        }]
    );
    assert!(dest.join("committed.txt").exists());
    assert!(!meta_path.with_extension("publication.json").exists());
}

#[test]
fn caller_owned_tempdir_cleanup_cannot_destroy_durable_intent_material() {
    let root = tempfile::tempdir().unwrap();
    let (repo, meta_path, dir) = create_repository(root.path());
    let staging_temp = tempfile::tempdir_in(root.path()).unwrap();
    let staging_path = staging_temp.path().to_path_buf();
    fs::write(staging_path.join("data.txt"), b"important").unwrap();
    let dest = dir.join("item1");

    // Inject metadata write failure
    let result = repo.publish_with(
        &staging_path,
        &dest,
        Record {
            id: "item1".into(),
            val: 1,
        },
        |_, _| Ok(()),
        |_| Err(MetadataError::LockPoisoned),
    );
    assert!(result.is_err());

    // When caller-owned tempdir is closed or dropped, destination was already safely rolled back
    let _ = staging_temp.close();
    assert!(!dest.exists());
    assert!(!meta_path.with_extension("publication.json").exists());
}

#[test]
fn failed_rollback_retains_receipt_and_material_with_visible_errors() {
    let root = tempfile::tempdir().unwrap();
    let (repo, meta_path, dir) = create_repository(root.path());
    let staging = root.path().join("staging");
    fs::create_dir_all(&staging).unwrap();
    let data_file = staging.join("data.txt");
    fs::write(&data_file, b"content").unwrap();
    let dest = dir.join("item1");

    let calls = std::sync::atomic::AtomicUsize::new(0);
    let result = repo.publish_with_effects(
        &staging,
        &dest,
        Record {
            id: "item1".into(),
            val: 1,
        },
        |_, _| Ok(()),
        |_| Err(MetadataError::Conflict("write failed".into())),
        move |src, dst| {
            let n = calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n < 2 {
                fs::rename(src, dst)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "rollback rename failed",
                ))
            }
        },
        |_path| {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "rollback cleanup failed",
            ))
        },
        |_path| Ok(()),
    );

    assert!(matches!(result, Err(MetadataError::Rollback { .. })));
    assert!(meta_path.with_extension("publication.json").exists());
    assert!(dest.exists());

    // Reconstruct new repository: normal recovery with remove_dir_all succeeds idempotently
    let new_repo = MetadataRepository::with_deletion_directory(
        meta_path.clone(),
        dir.clone(),
        |r: &Record| &r.id,
    );
    let recovered = new_repo.load().unwrap();
    assert!(recovered.is_empty());
    assert!(!dest.exists());
    assert!(!meta_path.with_extension("publication.json").exists());
}

#[test]
fn failed_acknowledgement_leaves_receipt_for_idempotent_recovery() {
    let root = tempfile::tempdir().unwrap();
    let (repo, meta_path, dir) = create_repository(root.path());
    let staging = root.path().join("staging");
    fs::create_dir_all(&staging).unwrap();
    fs::write(staging.join("data.txt"), b"content").unwrap();
    let dest = dir.join("item1");

    let repo_meta = meta_path.clone();
    let result = repo.publish_with_effects(
        &staging,
        &dest,
        Record {
            id: "item1".into(),
            val: 1,
        },
        |_, _| Ok(()),
        move |records| {
            let bytes = serde_json::to_vec_pretty(records).unwrap();
            fs::write(&repo_meta, bytes).map_err(|e| MetadataError::Io {
                operation: "write",
                path: repo_meta.clone(),
                source: e,
            })
        },
        |src, dst| fs::rename(src, dst),
        |path| fs::remove_dir_all(path),
        |_path| {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "ack failed",
            ))
        },
    );

    assert!(result.is_err());
    assert!(meta_path.with_extension("publication.json").exists());

    // Next access via reconstructed repository recovers and acknowledges idempotently
    let new_repo = MetadataRepository::with_deletion_directory(
        meta_path.clone(),
        dir.clone(),
        |r: &Record| &r.id,
    );
    let loaded = new_repo.load().unwrap();
    assert_eq!(
        loaded,
        vec![Record {
            id: "item1".into(),
            val: 1
        }]
    );
    assert!(!meta_path.with_extension("publication.json").exists());
}

#[test]
fn corrupt_or_unknown_receipt_blocks_recovery_without_destructive_guessing() {
    let root = tempfile::tempdir().unwrap();
    let (_repo, meta_path, dir) = create_repository(root.path());
    let receipt_path = meta_path.with_extension("publication.json");

    // Case 1: Corrupt JSON
    fs::write(&receipt_path, b"{corrupt json").unwrap();
    let repo1 = MetadataRepository::with_deletion_directory(
        meta_path.clone(),
        dir.clone(),
        |r: &Record| &r.id,
    );
    assert!(matches!(repo1.load(), Err(MetadataError::Decode { .. })));

    // Case 2: Unknown version
    fs::write(&receipt_path, b"{\"version\": 99, \"id\": \"item1\", \"staged\": \".publishing-550e8400-e29b-41d4-a716-446655440000\"}").unwrap();
    let repo2 = MetadataRepository::with_deletion_directory(
        meta_path.clone(),
        dir.clone(),
        |r: &Record| &r.id,
    );
    assert!(matches!(
        repo2.load(),
        Err(MetadataError::RecoveryBlocked(
            "Invalid publication receipt"
        ))
    ));
}

#[test]
fn path_traversal_and_symlink_receipts_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let (_repo, meta_path, dir) = create_repository(root.path());
    let receipt_path = meta_path.with_extension("publication.json");

    // Path traversal in receipt
    fs::write(
        &receipt_path,
        b"{\"version\": 1, \"id\": \"../escape\", \"staged\": \".publishing-550e8400-e29b-41d4-a716-446655440000\"}",
    )
    .unwrap();
    let repo1 = MetadataRepository::with_deletion_directory(
        meta_path.clone(),
        dir.clone(),
        |r: &Record| &r.id,
    );
    assert!(matches!(
        repo1.load(),
        Err(MetadataError::RecoveryBlocked("Unsafe publication path"))
    ));

    // Ambiguous directories state (both exist when uncommitted)
    let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
    let staged_dir = dir.join(format!(".publishing-{uuid_str}"));
    let target_dir = dir.join("item1");
    fs::create_dir_all(&staged_dir).unwrap();
    fs::create_dir_all(&target_dir).unwrap();
    fs::write(
        &receipt_path,
        format!("{{\"version\": 1, \"id\": \"item1\", \"staged\": \".publishing-{uuid_str}\"}}"),
    )
    .unwrap();
    let repo2 = MetadataRepository::with_deletion_directory(
        meta_path.clone(),
        dir.clone(),
        |r: &Record| &r.id,
    );
    assert!(matches!(
        repo2.load(),
        Err(MetadataError::RecoveryBlocked(
            "Ambiguous publication directories"
        ))
    ));
}

#[test]
fn repeated_recovery_preserves_unrelated_data() {
    let root = tempfile::tempdir().unwrap();
    let (repo, meta_path, dir) = create_repository(root.path());

    // Existing valid item
    let staging = root.path().join("staging_existing");
    fs::create_dir_all(&staging).unwrap();
    fs::write(staging.join("file.txt"), b"existing").unwrap();
    repo.publish(
        &staging,
        &dir.join("existing"),
        Record {
            id: "existing".into(),
            val: 99,
        },
        |_, _| Ok(()),
    )
    .unwrap();

    // Now simulate interrupted uncommitted publication of item2
    let uuid_str = uuid::Uuid::new_v4().to_string();
    let staged_dir = dir.join(format!(".publishing-{uuid_str}"));
    fs::create_dir_all(&staged_dir).unwrap();
    fs::write(
        meta_path.with_extension("publication.json"),
        format!("{{\"version\": 1, \"id\": \"item2\", \"staged\": \".publishing-{uuid_str}\"}}"),
    )
    .unwrap();

    // Reconstruct and recover multiple times
    for _ in 0..3 {
        let r = MetadataRepository::with_deletion_directory(
            meta_path.clone(),
            dir.clone(),
            |rec: &Record| &rec.id,
        );
        let records = r.load().unwrap();
        assert_eq!(
            records,
            vec![Record {
                id: "existing".into(),
                val: 99
            }]
        );
        assert_eq!(
            fs::read(dir.join("existing/file.txt")).unwrap(),
            b"existing"
        );
    }
}

#[test]
fn ordering_between_deletion_and_publication_recovery() {
    let root = tempfile::tempdir().unwrap();
    let (repo, meta_path, dir) = create_repository(root.path());

    // Publish item1
    let s = root.path().join("s1");
    fs::create_dir_all(&s).unwrap();
    fs::write(s.join("f.txt"), b"1").unwrap();
    repo.publish(
        &s,
        &dir.join("item1"),
        Record {
            id: "item1".into(),
            val: 1,
        },
        |_, _| Ok(()),
    )
    .unwrap();

    // Start deletion of item1 (uncommitted crash)
    let del_uuid = uuid::Uuid::new_v4().to_string();
    let backup_dir = dir.join(format!(".deleted-{del_uuid}"));
    fs::rename(dir.join("item1"), &backup_dir).unwrap();
    fs::write(
        meta_path.with_extension("deletion.json"),
        format!("{{\"version\": 1, \"id\": \"item1\", \"backup\": \".deleted-{del_uuid}\"}}"),
    )
    .unwrap();

    // Also simulate interrupted publication of item2
    let pub_uuid = uuid::Uuid::new_v4().to_string();
    let staged_dir = dir.join(format!(".publishing-{pub_uuid}"));
    fs::create_dir_all(&staged_dir).unwrap();
    fs::write(
        meta_path.with_extension("publication.json"),
        format!("{{\"version\": 1, \"id\": \"item2\", \"staged\": \".publishing-{pub_uuid}\"}}"),
    )
    .unwrap();

    // Reconstruct repo: deletion recovery runs first (restoring item1), then publication recovery runs (cleaning item2)
    let new_repo = MetadataRepository::with_deletion_directory(
        meta_path.clone(),
        dir.clone(),
        |r: &Record| &r.id,
    );
    let loaded = new_repo.load().unwrap();
    assert_eq!(
        loaded,
        vec![Record {
            id: "item1".into(),
            val: 1
        }]
    );
    assert!(dir.join("item1/f.txt").exists());
    assert!(!backup_dir.exists());
    assert!(!staged_dir.exists());
    assert!(!meta_path.with_extension("deletion.json").exists());
    assert!(!meta_path.with_extension("publication.json").exists());
}

#[test]
fn concurrent_publications_to_distinct_items_succeed() {
    let root = tempfile::tempdir().unwrap();
    let (repo, _meta_path, dir) = create_repository(root.path());
    let repo = Arc::new(repo);

    let handles: Vec<_> = (0..8)
        .map(|i| {
            let r = repo.clone();
            let root_path = root.path().to_path_buf();
            let dir_clone = dir.clone();
            std::thread::spawn(move || {
                let id = format!("item_{i}");
                let staging = root_path.join(format!("staging_{i}"));
                fs::create_dir_all(&staging).unwrap();
                fs::write(staging.join("data"), format!("{i}")).unwrap();
                r.publish(
                    &staging,
                    &dir_clone.join(&id),
                    Record {
                        id: id.clone(),
                        val: i,
                    },
                    |_, _| Ok(()),
                )
                .unwrap();
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    let loaded = repo.load().unwrap();
    assert_eq!(loaded.len(), 8);
}

#[test]
fn receipt_is_durable_before_the_first_directory_move() {
    let root = tempfile::tempdir().unwrap();
    let (repo, metadata, directory) = create_repository(root.path());
    let staging = root.path().join("staging");
    fs::create_dir(&staging).unwrap();
    fs::write(staging.join("data"), b"original").unwrap();
    let result = repo.publish_with_effects(
        &staging,
        &directory.join("item1"),
        Record {
            id: "item1".into(),
            val: 1,
        },
        |_, _| Ok(()),
        |_| Ok(()),
        |_, _| {
            let receipt = fs::read(metadata.with_extension("publication.json")).unwrap();
            let receipt: PublicationReceipt = serde_json::from_slice(&receipt).unwrap();
            assert_eq!(receipt.id, "item1");
            Err(io::Error::other("injected first move failure"))
        },
        |path| fs::remove_dir_all(path),
        |path| fs::remove_file(path),
    );
    assert!(result.is_err());
    assert_eq!(fs::read(staging.join("data")).unwrap(), b"original");
    assert!(repo.load().unwrap().is_empty());
}

#[test]
fn reported_postcommit_write_failure_preserves_published_directory() {
    let root = tempfile::tempdir().unwrap();
    let (repo, metadata, directory) = create_repository(root.path());
    let staging = root.path().join("staging");
    fs::create_dir(&staging).unwrap();
    fs::write(staging.join("data"), b"committed").unwrap();
    let result = repo.publish_with(
        &staging,
        &directory.join("item1"),
        Record {
            id: "item1".into(),
            val: 1,
        },
        |_, _| Ok(()),
        |records| {
            fs::write(&metadata, serde_json::to_vec(records).unwrap()).unwrap();
            Err(MetadataError::LockPoisoned)
        },
    );
    assert!(result.is_err());
    assert_eq!(
        fs::read(directory.join("item1/data")).unwrap(),
        b"committed"
    );
    assert_eq!(
        repo.load().unwrap(),
        vec![Record {
            id: "item1".into(),
            val: 1
        }]
    );
    assert!(!metadata.with_extension("publication.json").exists());
}

#[test]
fn generic_repository_cannot_guess_publication_identity_from_unrelated_records() {
    let root = tempfile::tempdir().unwrap();
    let metadata = root.path().join("records.json");
    fs::write(&metadata, b"[42]").unwrap();
    let uncommitted = root.path().join("uncommitted");
    fs::create_dir(&uncommitted).unwrap();
    let receipt = PublicationReceipt {
        version: 1,
        id: "uncommitted".into(),
        staged: format!(".publishing-{}", uuid::Uuid::new_v4()),
    };
    fs::write(
        metadata.with_extension("publication.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    let repo = MetadataRepository::<u32>::new(metadata.clone());
    assert!(matches!(
        repo.load(),
        Err(MetadataError::RecoveryBlocked(_))
    ));
    assert!(uncommitted.exists());
    assert!(metadata.with_extension("publication.json").exists());
}

#[test]
fn publication_cannot_reuse_an_existing_identity_when_its_directory_is_missing() {
    let root = tempfile::tempdir().unwrap();
    let (repo, _, directory) = create_repository(root.path());
    repo.update(|records| {
        records.push(Record {
            id: "item1".into(),
            val: 1,
        });
        Ok(())
    })
    .unwrap();
    let staging = root.path().join("staging");
    fs::create_dir(&staging).unwrap();
    fs::write(staging.join("data"), b"new").unwrap();
    assert!(matches!(
        repo.publish(
            &staging,
            &directory.join("item1"),
            Record {
                id: "item1".into(),
                val: 2
            },
            |_, _| Ok(())
        ),
        Err(MetadataError::Conflict(_))
    ));
    assert_eq!(
        repo.load().unwrap(),
        vec![Record {
            id: "item1".into(),
            val: 1
        }]
    );
    assert_eq!(fs::read(staging.join("data")).unwrap(), b"new");
}

#[cfg(unix)]
#[test]
fn linked_publication_directory_blocks_cleanup_of_external_data() {
    let root = tempfile::tempdir().unwrap();
    let (repo, metadata, directory) = create_repository(root.path());
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("world"), b"keep").unwrap();
    std::os::unix::fs::symlink(outside.path(), directory.join("item1")).unwrap();
    let receipt = PublicationReceipt {
        version: 1,
        id: "item1".into(),
        staged: format!(".publishing-{}", uuid::Uuid::new_v4()),
    };
    fs::write(
        metadata.with_extension("publication.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        repo.load(),
        Err(MetadataError::RecoveryBlocked(_))
    ));
    assert_eq!(fs::read(outside.path().join("world")).unwrap(), b"keep");
}
