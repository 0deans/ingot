use super::*;

#[derive(Serialize, Deserialize)]
struct Record {
    id: String,
}

struct Fixture {
    root: tempfile::TempDir,
    repository: MetadataRepository<Record>,
    receipt: Receipt,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let repository = Self::repository(root.path());
        fs::create_dir(root.path().join("items")).unwrap();
        fs::create_dir(root.path().join("items/record")).unwrap();
        fs::write(root.path().join("items/record/world"), b"retained world").unwrap();
        repository
            .update(|records| {
                records.push(Record {
                    id: "record".into(),
                });
                Ok(())
            })
            .unwrap();
        Self {
            root,
            repository,
            receipt: Receipt {
                version: 1,
                id: "record".into(),
                backup: format!(".deleted-{}", uuid::Uuid::new_v4()),
            },
        }
    }

    fn repository(root: &Path) -> MetadataRepository<Record> {
        MetadataRepository::with_deletion_directory(
            root.join("records.json"),
            root.join("items"),
            |record| &record.id,
        )
    }

    fn original(&self) -> PathBuf {
        self.root.path().join("items/record")
    }
    fn backup(&self) -> PathBuf {
        self.root.path().join("items").join(&self.receipt.backup)
    }
    fn retain(&self) {
        self.repository.write_receipt(&self.receipt).unwrap();
        fs::rename(self.original(), self.backup()).unwrap();
    }
}

#[test]
fn restart_before_or_after_rename_restores_metadata_present_directory() {
    for renamed in [false, true] {
        let fixture = Fixture::new();
        fixture.repository.write_receipt(&fixture.receipt).unwrap();
        if renamed {
            fs::rename(fixture.original(), fixture.backup()).unwrap();
        }
        let restarted = Fixture::repository(fixture.root.path());
        assert_eq!(restarted.load().unwrap().len(), 1);
        assert_eq!(
            fs::read(fixture.original().join("world")).unwrap(),
            b"retained world"
        );
        assert!(!fixture.backup().exists());
        assert!(!restarted.receipt_path().exists());
    }
}

#[test]
fn restart_after_metadata_commit_finishes_cleanup_and_acknowledgement() {
    for cleaned in [false, true] {
        let fixture = Fixture::new();
        fixture.retain();
        fixture.repository.write(&[]).unwrap();
        if cleaned {
            fs::remove_dir_all(fixture.backup()).unwrap();
        }
        let restarted = Fixture::repository(fixture.root.path());
        assert!(restarted.load().unwrap().is_empty());
        assert!(!fixture.original().exists());
        assert!(!fixture.backup().exists());
        assert!(!restarted.receipt_path().exists());
    }
}

#[test]
fn cleanup_failure_retains_receipt_and_material_for_next_attempt() {
    let fixture = Fixture::new();
    fixture.retain();
    fixture.repository.write(&[]).unwrap();
    let result = fixture.repository.recover_deletion_with(&[], |_| {
        Err(io::Error::new(io::ErrorKind::PermissionDenied, "injected"))
    });
    assert!(matches!(result, Err(MetadataError::RecoveryCleanup { .. })));
    assert!(fixture.repository.receipt_path().exists());
    assert_eq!(
        fs::read(fixture.backup().join("world")).unwrap(),
        b"retained world"
    );
    assert!(Fixture::repository(fixture.root.path())
        .load()
        .unwrap()
        .is_empty());
    assert!(!fixture.backup().exists());
}

#[test]
fn ambiguous_paths_block_all_repository_mutations_without_discarding_intent() {
    let fixture = Fixture::new();
    fixture.retain();
    fs::create_dir(fixture.original()).unwrap();
    fs::write(fixture.original().join("other"), b"new data").unwrap();
    let result = fixture.repository.update(|records| {
        records.clear();
        Ok(())
    });
    assert!(matches!(result, Err(MetadataError::RecoveryBlocked(_))));
    assert_eq!(fixture.repository.read().unwrap().len(), 1);
    assert!(fixture.repository.receipt_path().exists());
    assert_eq!(
        fs::read(fixture.original().join("other")).unwrap(),
        b"new data"
    );
    assert_eq!(
        fs::read(fixture.backup().join("world")).unwrap(),
        b"retained world"
    );
}

#[test]
fn malformed_or_escaping_receipts_are_preserved_without_touching_directories() {
    for (version, id, backup) in [
        (2, "record", "valid"),
        (
            1,
            "../outside",
            ".deleted-00000000-0000-0000-0000-000000000000",
        ),
        (1, "record", "../outside"),
    ] {
        let fixture = Fixture::new();
        let receipt = Receipt {
            version,
            id: id.into(),
            backup: backup.into(),
        };
        fixture.repository.write_receipt(&receipt).unwrap();
        let bytes = fs::read(fixture.repository.receipt_path()).unwrap();
        assert!(fixture.repository.load().is_err());
        assert_eq!(fs::read(fixture.repository.receipt_path()).unwrap(), bytes);
        assert_eq!(
            fs::read(fixture.original().join("world")).unwrap(),
            b"retained world"
        );
    }
}

#[test]
fn invalid_json_receipts_block_loading_and_remain_available_for_diagnostics() {
    for bytes in [b"{".as_slice(), b"{}".as_slice(), b"null".as_slice()] {
        let fixture = Fixture::new();
        fs::write(fixture.repository.receipt_path(), bytes).unwrap();
        assert!(matches!(
            fixture.repository.load(),
            Err(MetadataError::Decode { .. })
        ));
        assert_eq!(fs::read(fixture.repository.receipt_path()).unwrap(), bytes);
        assert_eq!(
            fs::read(fixture.original().join("world")).unwrap(),
            b"retained world"
        );
    }
}

#[test]
fn duplicate_metadata_or_lost_retained_directory_blocks_recovery() {
    for duplicate in [false, true] {
        let fixture = Fixture::new();
        fixture.retain();
        if duplicate {
            fixture
                .repository
                .write(&[
                    Record {
                        id: "record".into(),
                    },
                    Record {
                        id: "record".into(),
                    },
                ])
                .unwrap();
        } else {
            fs::remove_dir_all(fixture.backup()).unwrap();
        }
        assert!(matches!(
            fixture.repository.load(),
            Err(MetadataError::RecoveryBlocked(_))
        ));
        assert!(fixture.repository.receipt_path().exists());
        assert!(!fixture.original().exists());
    }
}

#[test]
fn corrupt_metadata_blocks_recovery_until_metadata_is_repaired() {
    let fixture = Fixture::new();
    fixture.retain();
    fs::write(&fixture.repository.path, b"invalid").unwrap();
    assert!(matches!(
        fixture.repository.load(),
        Err(MetadataError::Decode { .. })
    ));
    assert!(fixture.backup().exists());
    assert!(!fixture.original().exists());
    assert!(fixture.repository.receipt_path().exists());
}

#[test]
fn publication_recovers_previous_deletion_before_publishing_another_record() {
    let fixture = Fixture::new();
    fixture.retain();
    let stage = fixture.root.path().join("stage");
    let destination = fixture.root.path().join("items/new");
    fs::create_dir(&stage).unwrap();
    fixture
        .repository
        .publish(&stage, &destination, Record { id: "new".into() }, |_, _| {
            Ok(())
        })
        .unwrap();
    assert_eq!(fixture.repository.load().unwrap().len(), 2);
    assert_eq!(
        fs::read(fixture.original().join("world")).unwrap(),
        b"retained world"
    );
    assert!(destination.is_dir());
}

#[test]
fn serialized_effect_does_not_run_with_ambiguous_recovery() {
    let fixture = Fixture::new();
    fixture.retain();
    fs::create_dir(fixture.original()).unwrap();
    let ran = std::cell::Cell::new(false);
    let result = fixture.repository.with_serialized(|| ran.set(true));
    assert!(matches!(result, Err(MetadataError::RecoveryBlocked(_))));
    assert!(!ran.get());
}

#[cfg(unix)]
#[test]
fn linked_backup_blocks_recovery_and_preserves_external_data() {
    let fixture = Fixture::new();
    fixture.retain();
    let outside = fixture.root.path().join("outside");
    fs::rename(fixture.backup(), &outside).unwrap();
    std::os::unix::fs::symlink(&outside, fixture.backup()).unwrap();
    assert!(matches!(
        fixture.repository.load(),
        Err(MetadataError::RecoveryBlocked(_))
    ));
    assert_eq!(fs::read(outside.join("world")).unwrap(), b"retained world");
    assert!(fixture.repository.receipt_path().exists());
}

#[test]
fn metadata_only_deletion_keeps_directory_and_creates_no_receipt() {
    let fixture = Fixture::new();
    fixture
        .repository
        .delete_record("record", DeletionFiles::Retain)
        .unwrap();
    assert!(fixture.repository.load().unwrap().is_empty());
    assert_eq!(
        fs::read(fixture.original().join("world")).unwrap(),
        b"retained world"
    );
    assert!(!fixture.repository.receipt_path().exists());
}

#[test]
fn failed_rename_preserves_metadata_and_acknowledges_unused_intent() {
    let fixture = Fixture::new();
    let result = fixture
        .repository
        .delete_with_rename("record", DeletionFiles::Remove, |_, _| {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "injected"))
        });
    assert!(matches!(result, Err(MetadataError::Io { .. })));
    assert_eq!(fixture.repository.load().unwrap().len(), 1);
    assert!(fixture.original().exists());
    assert!(!fixture.repository.receipt_path().exists());
}
