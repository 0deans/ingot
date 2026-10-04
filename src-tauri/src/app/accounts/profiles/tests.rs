use super::*;
use crate::{
    account::credentials::{tests::FakeCredentialStore, CredentialStore},
    app::{AppPaths, AppState},
};
use tauri::Manager;

#[tokio::test]
async fn offline_removal_needs_no_vault_and_promotes_remaining_profile() {
    let dir = tempfile::tempdir().unwrap();
    let app = tauri::test::mock_app();
    let mut state = AppState::new(AppPaths::new(dir.path().to_owned())).unwrap();
    let vault = FakeCredentialStore::default();
    vault.fail_reads();
    vault.fail_writes();
    state.credentials = Box::new(vault);
    app.manage(state);
    add_offline_account(app.handle().clone(), "First".to_owned())
        .await
        .unwrap();
    add_offline_account(app.handle().clone(), "Second".to_owned())
        .await
        .unwrap();
    remove_account(app.handle().clone(), "offline:second".to_owned())
        .await
        .unwrap();
    let profiles = get_accounts(app.handle().clone()).unwrap();
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].id, "offline:first");
    assert!(profiles[0].is_active);
}

#[tokio::test]
async fn failed_profile_removal_does_not_delete_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    std::fs::write(&path, b"[broken").unwrap();
    let app = tauri::test::mock_app();
    let mut state = AppState::new(AppPaths::new(dir.path().to_owned())).unwrap();
    let vault = FakeCredentialStore::default();
    vault.put("microsoft:test", "old");
    state.credentials = Box::new(vault.clone());
    app.manage(state);
    assert!(
        remove_account(app.handle().clone(), "microsoft:test".to_owned())
            .await
            .is_err()
    );
    assert_eq!(
        vault.read("microsoft:test").unwrap().as_deref(),
        Some("old")
    );
    assert_eq!(std::fs::read(path).unwrap(), b"[broken");
    assert!(!dir.path().join("accounts.recovery").exists());
}

#[tokio::test]
async fn account_read_recovers_pending_login_when_application_state_is_recreated() {
    use crate::account::{recovery::ProfileChange, AccountSecrets};
    let dir = tempfile::tempdir().unwrap();
    let paths = || AppPaths::new(dir.path().to_owned());
    let vault = FakeCredentialStore::default();
    vault.fail_write_for(Some("microsoft:test"));
    let mut state = AppState::new(paths()).unwrap();
    state.credentials = Box::new(vault.clone());
    let operation = state
        .account_operations
        .acquire("microsoft:test")
        .await
        .unwrap();
    let profile = AccountProfile {
        id: "microsoft:test".to_owned(),
        account_type: "microsoft".to_owned(),
        username: "Recovered".to_owned(),
        uuid: "test".to_owned(),
        skin_url: None,
        is_active: true,
        created_at: 42,
    };
    assert!(state
        .account_recovery
        .publish(
            state.credentials.as_ref(),
            &state.accounts,
            &operation,
            &AccountSecrets {
                access_token: "new-token".to_owned(),
                ..Default::default()
            },
            ProfileChange::Activate { profile }
        )
        .is_err());
    drop(operation);
    drop(state);
    vault.fail_write_for(None);
    let app = tauri::test::mock_app();
    let mut restarted = AppState::new(paths()).unwrap();
    restarted.credentials = Box::new(vault.clone());
    app.manage(restarted);
    let profiles = get_accounts(app.handle().clone()).unwrap();
    assert_eq!(profiles[0].username, "Recovered");
    assert!(!dir.path().join("accounts.recovery").exists());
    remove_account(app.handle().clone(), "microsoft:test".to_owned())
        .await
        .unwrap();
    assert!(get_accounts(app.handle().clone()).unwrap().is_empty());
    assert!(vault.read("microsoft:test").unwrap().is_none());
}
