use super::*;
use std::{error::Error, io};

#[test]
fn failed_journal_directory_creation_preserves_io_cause_and_existing_data() {
    let root = tempfile::tempdir().unwrap();
    let backup = backup_dir(root.path());
    fs::create_dir_all(backup.parent().unwrap()).unwrap();
    fs::write(&backup, b"keep existing recovery data").unwrap();
    let error = write_journal_typed(root.path(), &Journal::default()).unwrap_err();
    assert!(matches!(
        &error,
        VersionChangeError::JournalIo {
            operation: "create directory for",
            ..
        }
    ));
    assert!(error.source().unwrap().is::<io::Error>());
    assert_eq!(fs::read(&backup).unwrap(), b"keep existing recovery data");
    assert!(matches!(
        crate::ipc::ApiError::from(error),
        crate::ipc::ApiError::StorageFailure { .. }
    ));
}

#[test]
fn corrupt_journal_cannot_be_reported_as_no_backup() {
    let root = tempfile::tempdir().unwrap();
    let backup = backup_dir(root.path());
    fs::create_dir_all(&backup).unwrap();
    let path = backup.join(JOURNAL);
    fs::write(&path, b"{corrupt journal").unwrap();
    assert!(last_backup(root.path()).is_err());
    let status = transaction_status(root.path());
    assert!(status.is_incomplete);
    assert!(status.error.is_some());
    assert_eq!(fs::read(path).unwrap(), b"{corrupt journal");
}

#[test]
fn unreadable_backup_is_not_measured_as_zero() {
    let root = tempfile::tempdir().unwrap();
    let error = backup_summary_with(root.path(), &Journal::default(), |_| {
        Err(io::Error::from(io::ErrorKind::PermissionDenied))
    })
    .unwrap_err();
    assert!(
        matches!(&error, VersionChangeError::JournalIo { operation: "measure backup for", path, source }
        if *path == backup_dir(root.path()) && source.kind() == io::ErrorKind::PermissionDenied)
    );
    assert!(error.source().unwrap().is::<io::Error>());
}
