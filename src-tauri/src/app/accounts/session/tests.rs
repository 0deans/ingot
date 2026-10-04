use super::*;
use crate::{
    account::{
        credentials::{self, tests::FakeCredentialStore},
        policy,
    },
    app::{AppPaths, AppState},
};
use std::{
    future::{poll_fn, Future},
    task::Poll,
};
use tauri::Manager;

fn profile() -> AccountProfile {
    AccountProfile {
        id: "microsoft:test".to_owned(),
        account_type: "microsoft".to_owned(),
        username: "Player".to_owned(),
        uuid: "test".to_owned(),
        skin_url: None,
        is_active: true,
        created_at: 42,
    }
}

#[tokio::test]
async fn waiting_token_request_reads_current_credentials_after_relogin() {
    let dir = tempfile::tempdir().unwrap();
    let app = tauri::test::mock_app();
    let mut state = AppState::new(AppPaths::new(dir.path().to_owned())).unwrap();
    let vault = FakeCredentialStore::default();
    state.credentials = Box::new(vault.clone());
    state.accounts.activate(profile()).unwrap();
    let initial = AccountSecrets {
        access_token: "old".to_owned(),
        expires_at: Some(u64::MAX),
        ..Default::default()
    };
    vault.put("microsoft:test", &serde_json::to_string(&initial).unwrap());
    app.manage(state);
    let state = crate::app::state(app.handle());
    let operation = state
        .account_operations
        .acquire("microsoft:test")
        .await
        .unwrap();
    let mut request = Box::pin(get_active_account_token(app.handle().clone()));
    assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
    let next = AccountSecrets {
        access_token: "new".to_owned(),
        expires_at: Some(u64::MAX),
        ..Default::default()
    };
    credentials::publish(state.credentials.as_ref(), &operation, &next, || {
        state.accounts.activate(profile())
    })
    .unwrap();
    drop(operation);
    assert_eq!(request.await.unwrap(), "new");
}

#[tokio::test]
async fn waiting_token_request_rejects_removed_profile_without_touching_vault() {
    let dir = tempfile::tempdir().unwrap();
    let app = tauri::test::mock_app();
    let mut state = AppState::new(AppPaths::new(dir.path().to_owned())).unwrap();
    let vault = FakeCredentialStore::default();
    vault.fail_reads(); // A stale request must fail on the missing profile, before this vault read.
    state.credentials = Box::new(vault);
    state.accounts.activate(profile()).unwrap();
    app.manage(state);
    let state = crate::app::state(app.handle());
    let operation = state
        .account_operations
        .acquire("microsoft:test")
        .await
        .unwrap();
    let mut request = Box::pin(get_active_account_token(app.handle().clone()));
    assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
    state
        .accounts
        .update(|accounts| {
            policy::remove(accounts, "microsoft:test")?;
            Ok(())
        })
        .unwrap();
    drop(operation);
    assert_eq!(request.await.unwrap_err(), "Account not found");
}
