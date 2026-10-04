use super::*;

fn stored(context: &Context, id: &str) -> AccountSecrets {
    credentials::load(&context.credentials, id).unwrap()
}

fn exchange_was_not_started(context: &Context) {
    assert!(!context
        .provider
        .calls
        .lock()
        .unwrap()
        .contains(&ProviderOperation::MicrosoftExchange));
}

#[tokio::test]
async fn exchange_failure_preserves_rotation_and_retry_uses_the_new_refresh_token() {
    let mut context = Context::new();
    let id = context.add("microsoft", 0);
    let before = context.bytes();
    context.provider.fail = Some(ProviderOperation::MicrosoftExchange);
    assert!(matches!(
        context.service().active_token().await,
        Err(SessionError::Provider(ProviderError::Rejected {
            operation: ProviderOperation::MicrosoftExchange,
            ..
        }))
    ));
    let checkpoint = stored(&context, &id);
    assert_eq!(checkpoint.refresh_token.as_deref(), Some("rotated-refresh"));
    assert_eq!(checkpoint.access_token, "old-access");
    assert_eq!(checkpoint.expires_at, Some(0));
    assert_eq!(context.bytes(), before);
    assert!(!context.directory.path().join("accounts.recovery").exists());
    context.provider.fail = None;
    assert_eq!(
        context.service().active_token().await.unwrap(),
        "rotated-access"
    );
    assert_eq!(
        *context.provider.refresh_inputs.lock().unwrap(),
        ["old-refresh", "rotated-refresh"]
    );
}

#[tokio::test]
async fn dropping_exchange_keeps_rotation_across_restart_and_releases_account_ownership() {
    let context = Context::new();
    let id = context.add("microsoft", 0);
    let release = context.provider.pause_exchange();
    let service = context.service();
    let mut request = Box::pin(service.active_token());
    assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
    assert_eq!(
        stored(&context, &id).refresh_token.as_deref(),
        Some("rotated-refresh")
    );
    drop(request);
    assert!(release.send(()).is_err());
    let ownership = context.operations.acquire(&id).await.unwrap();
    drop(ownership);
    let repository = AccountRepository::new(context.directory.path().join("accounts.json"));
    let recovery = AccountRecovery::new(context.directory.path().join("accounts.recovery"));
    let mut restarted = context.service();
    restarted.repository = &repository;
    restarted.recovery = &recovery;
    assert_eq!(restarted.active_token().await.unwrap(), "rotated-access");
    assert_eq!(
        *context.provider.refresh_inputs.lock().unwrap(),
        ["old-refresh", "rotated-refresh"]
    );
}

#[tokio::test]
async fn failed_canonical_checkpoint_write_recovers_before_retrying_the_exchange() {
    let context = Context::new();
    let id = context.add("microsoft", 0);
    let before = context.bytes();
    context.credentials.fail_write_for(Some(&id));
    let error = context.service().active_token().await.unwrap_err();
    assert!(matches!(
        error,
        SessionError::RotationCheckpoint(RecoveryError::PendingCredentials(_))
    ));
    exchange_was_not_started(&context);
    assert_eq!(
        stored(&context, &id).refresh_token.as_deref(),
        Some("old-refresh")
    );
    assert_eq!(context.bytes(), before);
    let marker = context.directory.path().join("accounts.recovery");
    assert!(marker.exists());
    assert_eq!(
        std::fs::read(&marker).unwrap(),
        b"ingot-account-recovery-v1"
    );
    context.credentials.fail_write_for(None);
    let recovery = AccountRecovery::new(marker.clone());
    let mut restarted = context.service();
    restarted.recovery = &recovery;
    assert_eq!(restarted.active_token().await.unwrap(), "rotated-access");
    assert_eq!(
        *context.provider.refresh_inputs.lock().unwrap(),
        ["old-refresh", "rotated-refresh"]
    );
    assert!(!marker.exists());
}

#[tokio::test]
async fn unavailable_journal_reports_uncheckpointed_rotation_and_stops_exchange() {
    let context = Context::new();
    let id = context.add("microsoft", 0);
    let before = context.bytes();
    context.credentials.fail_writes();
    let error = context.service().active_token().await.unwrap_err();
    assert!(matches!(
        error,
        SessionError::RotationCheckpoint(RecoveryError::Credentials(_))
    ));
    exchange_was_not_started(&context);
    assert_eq!(
        stored(&context, &id).refresh_token.as_deref(),
        Some("old-refresh")
    );
    assert_eq!(context.bytes(), before);
    assert!(!error.to_string().contains("rotated-refresh"));
    assert!(!error.to_string().contains("fixture-ms-access"));
    assert!(std::error::Error::source(&error).is_some());
}

#[tokio::test]
async fn discovery_failure_after_rotation_stops_exchange_without_overwriting_credentials() {
    let context = Context::new();
    let id = context.add("microsoft", 0);
    let release = context.provider.pause();
    let service = context.service();
    let mut request = Box::pin(service.active_token());
    assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
    std::fs::create_dir(context.directory.path().join("accounts.recovery")).unwrap();
    release.send(()).unwrap();
    assert!(matches!(
        request.await,
        Err(SessionError::RotationCheckpoint(RecoveryError::Discovery(
            _
        )))
    ));
    exchange_was_not_started(&context);
    assert_eq!(
        stored(&context, &id).refresh_token.as_deref(),
        Some("old-refresh")
    );
}

#[tokio::test]
async fn final_profile_failure_preserves_full_session_and_recovers_without_another_refresh() {
    let context = Context::new();
    let id = context.add("microsoft", 0);
    let before = context.bytes();
    let release = context.provider.pause_exchange();
    let service = context.service();
    let mut request = Box::pin(service.active_token());
    assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
    let path = context.directory.path().join("accounts.json");
    std::fs::write(&path, b"[corrupt").unwrap();
    release.send(()).unwrap();
    assert!(matches!(
        request.await,
        Err(SessionError::Recovery(RecoveryError::Profile(_)))
    ));
    assert_eq!(stored(&context, &id).access_token, "rotated-access");
    assert_eq!(context.bytes(), b"[corrupt");
    std::fs::write(path, before).unwrap();
    let recovery = AccountRecovery::new(context.directory.path().join("accounts.recovery"));
    let mut restarted = context.service();
    restarted.recovery = &recovery;
    assert_eq!(restarted.active_token().await.unwrap(), "rotated-access");
    assert_eq!(context.provider.refresh_inputs.lock().unwrap().len(), 1);
    assert_eq!(
        context.repository.load().unwrap()[0].username,
        "UpdatedPlayer"
    );
}

#[tokio::test]
async fn final_credential_failure_retains_checkpoint_and_full_session_journal() {
    let context = Context::new();
    let id = context.add("microsoft", 0);
    let release = context.provider.pause_exchange();
    let service = context.service();
    let mut request = Box::pin(service.active_token());
    assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
    context.credentials.fail_write_for(Some(&id));
    release.send(()).unwrap();
    assert!(matches!(
        request.await,
        Err(SessionError::Recovery(RecoveryError::PendingCredentials(_)))
    ));
    let checkpoint = stored(&context, &id);
    assert_eq!(checkpoint.access_token, "old-access");
    assert_eq!(checkpoint.refresh_token.as_deref(), Some("rotated-refresh"));
    assert_eq!(checkpoint.expires_at, Some(0));
    context.credentials.fail_write_for(None);
    let recovery = AccountRecovery::new(context.directory.path().join("accounts.recovery"));
    let mut restarted = context.service();
    restarted.recovery = &recovery;
    assert_eq!(restarted.active_token().await.unwrap(), "rotated-access");
    assert_eq!(context.provider.refresh_inputs.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn removal_after_abandoned_exchange_cleans_checkpoint_without_recreating_profile() {
    let context = Context::new();
    let id = context.add("microsoft", 0);
    let release = context.provider.pause_exchange();
    let service = context.service();
    let mut request = Box::pin(service.active_token());
    assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
    drop(request);
    assert!(release.send(()).is_err());
    let operation = context.operations.acquire(&id).await.unwrap();
    context
        .recovery
        .remove(&context.credentials, &context.repository, &operation)
        .unwrap();
    assert!(context.credentials.read(&id).unwrap().is_none());
    assert!(context.repository.load().unwrap().is_empty());
    assert!(matches!(
        service.active_token().await,
        Err(SessionError::NoActiveAccount)
    ));
    assert!(!context.directory.path().join("accounts.recovery").exists());
}
