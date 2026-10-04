use super::*;

fn profile(id: &str) -> AccountProfile {
    AccountProfile {
        id: id.to_owned(),
        account_type: "offline".to_owned(),
        username: id.to_owned(),
        uuid: id.to_owned(),
        skin_url: None,
        is_active: true,
        created_at: 42,
    }
}

#[test]
fn failed_profile_removal_does_not_delete_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    std::fs::write(&path, b"[broken").unwrap();
    let repository = AccountRepository::new(path.clone());
    let mut cleaned = false;
    let error = remove_with_cleanup(&repository, "a", |_| {
        cleaned = true;
        Ok(())
    })
    .unwrap_err();
    assert!(matches!(error, AccountError::Decode { .. }));
    assert!(!cleaned);
    assert_eq!(std::fs::read(path).unwrap(), b"[broken");
}

#[test]
fn cleanup_failure_keeps_committed_removal_and_promoted_selection() {
    let dir = tempfile::tempdir().unwrap();
    let repository = AccountRepository::new(dir.path().join("accounts.json"));
    repository.activate(profile("a")).unwrap();
    repository.activate(profile("b")).unwrap();
    remove_with_cleanup(&repository, "b", |id| {
        assert_eq!(id, "b");
        let accounts = repository.load().unwrap(); // Cleanup runs outside the repository lock.
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].id, "a");
        assert!(accounts[0].is_active);
        Err(CredentialError::Vault {
            operation: "delete",
            source: crate::account::credentials::VaultFailure("injected vault failure".to_owned()),
        })
    })
    .unwrap();
    assert_eq!(repository.load().unwrap().len(), 1);
}
