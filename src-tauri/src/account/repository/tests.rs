use std::{
    error::Error,
    io::Write,
    sync::{Arc, Barrier},
};

use super::*;
use tempfile::NamedTempFile;

fn profile(id: &str) -> AccountProfile {
    AccountProfile {
        id: id.to_owned(),
        account_type: "offline".to_owned(),
        username: id.to_owned(),
        uuid: format!("uuid-{id}"),
        skin_url: None,
        is_active: false,
        created_at: 42,
    }
}

#[test]
fn missing_storage_is_empty_without_creating_directories() {
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path().join("not-created");
    let repository = AccountRepository::new(parent.join("accounts.json"));
    assert!(repository.load().unwrap().is_empty());
    assert!(!parent.exists());
    repository.activate(profile("a")).unwrap();
    assert!(parent.join("accounts.json").is_file());
}

#[test]
fn corrupt_storage_blocks_reads_and_updates_and_preserves_input_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    let repository = AccountRepository::new(path.clone());
    for corrupt in [b"[{broken".as_slice(), b"{}", b"null", b"\xff\xfe"] {
        fs::write(&path, corrupt).unwrap();
        let error = repository.load().unwrap_err();
        assert!(error.source().is_some());
        assert!(matches!(error, AccountError::Decode { .. }));
        let mut called = false;
        assert!(matches!(
            repository.update(|_| {
                called = true;
                Ok(())
            }),
            Err(AccountError::Decode { .. })
        ));
        assert!(!called);
        assert_eq!(fs::read(&path).unwrap(), corrupt);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}

#[test]
fn unreadable_storage_is_an_io_failure_rather_than_missing_or_corrupt() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    // Reading a directory as a file fails on each supported desktop platform, even as admin/root.
    fs::create_dir(&path).unwrap();
    let repository = AccountRepository::new(path.clone());
    let error = repository.load().unwrap_err();
    assert!(error.source().is_some());
    assert!(matches!(
        error,
        AccountError::Io {
            operation: "read",
            ..
        }
    ));
    assert!(matches!(
        repository.activate(profile("a")),
        Err(AccountError::Io {
            operation: "read",
            ..
        })
    ));
    assert!(path.is_dir());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[derive(Clone, Copy)]
enum Failure {
    Stage,
    Write,
    Flush,
    Replace,
}

struct FailingPersistence(Failure);

impl Persistence for FailingPersistence {
    fn stage(&self, directory: &Path) -> io::Result<NamedTempFile> {
        if matches!(self.0, Failure::Stage) {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Filesystem.stage(directory)
    }

    fn write(&self, file: &mut NamedTempFile, bytes: &[u8]) -> io::Result<()> {
        if matches!(self.0, Failure::Write) {
            file.write_all(&bytes[..bytes.len() / 2])?;
            return Err(io::ErrorKind::WriteZero.into());
        }
        Filesystem.write(file, bytes)
    }

    fn flush(&self, file: &NamedTempFile) -> io::Result<()> {
        if matches!(self.0, Failure::Flush) {
            return Err(io::ErrorKind::Other.into());
        }
        Filesystem.flush(file)
    }

    fn replace(&self, file: NamedTempFile, destination: &Path) -> io::Result<()> {
        if matches!(self.0, Failure::Replace) {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Filesystem.replace(file, destination)
    }
}

#[test]
fn failed_publication_preserves_previous_bytes_cleans_staging_and_allows_retry() {
    for (failure, operation) in [
        (Failure::Stage, "stage"),
        (Failure::Write, "write"),
        (Failure::Flush, "flush"),
        (Failure::Replace, "replace"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("accounts.json");
        let repository = AccountRepository::new(path.clone());
        repository.activate(profile("a")).unwrap();
        let previous = fs::read(&path).unwrap();
        let error = repository
            .update_with(
                |accounts| {
                    accounts[0].username = "changed".to_owned();
                    Ok("only-after-commit")
                },
                &FailingPersistence(failure),
            )
            .unwrap_err();
        assert!(matches!(error, AccountError::Io { operation: actual, .. } if actual == operation));
        assert_eq!(fs::read(&path).unwrap(), previous);
        assert_eq!(repository.load().unwrap()[0].username, "a");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
        repository
            .update_profile("a", |account| account.username = "retry".to_owned())
            .unwrap();
        assert_eq!(repository.load().unwrap()[0].username, "retry");
    }
}

#[test]
fn rejected_mutation_preserves_existing_storage_even_after_changing_the_local_copy() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    let repository = AccountRepository::new(path.clone());
    repository.activate(profile("a")).unwrap();
    let previous = fs::read(&path).unwrap();
    assert!(matches!(
        repository.update(|accounts| {
            accounts.clear();
            Err::<(), _>(policy::AccountPolicyError::NotFound.into())
        }),
        Err(AccountError::Policy(policy::AccountPolicyError::NotFound))
    ));
    assert_eq!(fs::read(&path).unwrap(), previous);
}

#[test]
fn concurrent_activations_keep_every_profile_and_exactly_one_selection() {
    let dir = tempfile::tempdir().unwrap();
    let repository = Arc::new(AccountRepository::new(dir.path().join("accounts.json")));
    let barrier = Arc::new(Barrier::new(12));
    std::thread::scope(|scope| {
        for index in 0..12 {
            let repository = Arc::clone(&repository);
            let barrier = Arc::clone(&barrier);
            scope.spawn(move || {
                barrier.wait();
                repository
                    .activate(profile(&format!("offline:{index}")))
                    .unwrap();
            });
        }
    });
    let accounts = repository.load().unwrap();
    assert_eq!(accounts.len(), 12);
    assert_eq!(
        accounts.iter().filter(|account| account.is_active).count(),
        1
    );
    for index in 0..12 {
        assert!(accounts
            .iter()
            .any(|account| account.id == format!("offline:{index}")));
    }
}

#[test]
fn delayed_metadata_patch_keeps_new_selection_and_does_not_resurrect_removed_accounts() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    let repository = AccountRepository::new(path.clone());
    repository.activate(profile("a")).unwrap();
    let before_provider_io = repository.load().unwrap();
    repository.activate(profile("b")).unwrap();
    repository
        .update_profile(&before_provider_io[0].id, |account| {
            account.skin_url = Some("skin".to_owned())
        })
        .unwrap();
    let accounts = repository.load().unwrap();
    assert_eq!(accounts.len(), 2);
    assert!(!accounts[0].is_active);
    assert_eq!(accounts[0].skin_url.as_deref(), Some("skin"));
    assert!(accounts[1].is_active);
    repository
        .update(|accounts| {
            policy::remove(accounts, "a")?;
            Ok(())
        })
        .unwrap();
    let previous = fs::read(&path).unwrap();
    assert!(matches!(
        repository.update_profile("a", |account| account.username = "late".to_owned()),
        Err(AccountError::Policy(policy::AccountPolicyError::NotFound))
    ));
    assert_eq!(fs::read(&path).unwrap(), previous);
    assert_eq!(repository.load().unwrap().len(), 1);
}

#[test]
fn existing_provider_metadata_survives_a_selection_update() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    let original: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/accounts-current.json"
    ))
    .unwrap();
    fs::write(&path, serde_json::to_vec_pretty(&original).unwrap()).unwrap();
    let repository = AccountRepository::new(path.clone());
    repository
        .update(|accounts| {
            policy::select(accounts, "microsoft:456")?;
            Ok(())
        })
        .unwrap();
    let mut expected = original;
    expected[0]["isActive"] = false.into();
    expected[1]["isActive"] = true.into();
    let stored: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(stored, expected);
}

#[test]
fn poisoned_lock_returns_a_typed_failure_instead_of_panicking() {
    let dir = tempfile::tempdir().unwrap();
    let repository = AccountRepository::new(dir.path().join("accounts.json"));
    std::thread::scope(|scope| {
        let result = scope
            .spawn(|| {
                let _guard = repository.lock.lock().unwrap();
                panic!("injected mutation panic");
            })
            .join();
        assert!(result.is_err());
    });
    assert!(matches!(repository.load(), Err(AccountError::LockPoisoned)));
    assert!(matches!(
        repository.activate(profile("a")),
        Err(AccountError::LockPoisoned)
    ));
}
