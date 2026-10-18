use super::*;
use std::{error::Error, io};

#[test]
fn failed_rollback_cleanup_keeps_retryable_checkpoint_and_original_cause() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let backup = backup_dir(root);
    let mut journal = Journal::default();
    write_journal_typed(root, &journal).unwrap();
    let primary =
        VersionChangeError::MetadataCommit(Box::new(io::Error::from(io::ErrorKind::StorageFull)));
    let error = rollback_with(
        root,
        &backup,
        &mut journal,
        primary,
        write_journal_typed,
        |_, path| {
            Err(super::super::super::error::file_io(
                "cleanup",
                path,
                io::Error::from(io::ErrorKind::PermissionDenied),
            ))
        },
    );
    let VersionChangeError::AdditionalFailure { primary, secondary } = error else {
        panic!("cleanup must not be reported as a clean rollback")
    };
    assert_eq!(
        primary
            .source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap()
            .kind(),
        io::ErrorKind::StorageFull
    );
    assert_eq!(
        secondary
            .source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap()
            .kind(),
        io::ErrorKind::PermissionDenied
    );
    assert_eq!(
        super::super::super::journal::read_journal(root)
            .unwrap()
            .phase,
        TransactionPhase::Restored
    );
    super::super::recovery::recover_interrupted_transaction_with_commit(
        root,
        |_| -> Result<(), io::Error> { panic!("cleanup retry must not recommit metadata") },
    )
    .unwrap();
    assert!(!backup.exists());
}

#[test]
fn failed_restore_retains_primary_restore_and_checkpoint_errors() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let backup = backup_dir(root);
    let mut journal = Journal {
        moved: vec![("mods/a.jar".into(), "files/mods/a.jar".into())],
        ..Default::default()
    };
    write_journal_typed(root, &journal).unwrap();
    let error = rollback_with(
        root,
        &backup,
        &mut journal,
        VersionChangeError::MetadataCommit(Box::new(io::Error::from(io::ErrorKind::StorageFull))),
        |_, _| {
            Err(super::super::super::error::file_io(
                "checkpoint",
                &backup,
                io::Error::from(io::ErrorKind::PermissionDenied),
            ))
        },
        |_, _| panic!("must preserve recovery material"),
    );
    let VersionChangeError::RollbackFailed {
        primary,
        restore,
        checkpoint,
        ..
    } = error
    else {
        panic!("expected failed rollback")
    };
    assert!(primary.source().unwrap().is::<io::Error>());
    assert!(matches!(*restore, VersionChangeError::RestoreFailures(_)));
    assert!(matches!(
        *checkpoint.unwrap(),
        VersionChangeError::FileIo { .. }
    ));
    assert!(backup.exists());
}
