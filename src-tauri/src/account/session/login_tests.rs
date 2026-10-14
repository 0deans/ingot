use super::*;
use crate::account::login::LoginError;

fn no_publication(context: &Context) {
    assert!(context.repository.load().unwrap().is_empty());
    assert!(context
        .credentials
        .read("microsoft:fixture-id")
        .unwrap()
        .is_none());
    assert!(!context.directory.path().join("accounts.recovery").exists());
}

#[tokio::test]
async fn unknown_stale_cancelled_and_altered_codes_never_reach_the_provider() {
    let context = Context::new();
    let service = context.service();
    assert!(matches!(
        service.microsoft_finish(&code()).await,
        Err(SessionError::Login(LoginError::UnknownCode))
    ));
    let old = service.microsoft_start().await.unwrap();
    let current = service.microsoft_start().await.unwrap();
    assert!(matches!(
        service.microsoft_finish(&old).await,
        Err(SessionError::Login(LoginError::UnknownCode))
    ));
    for field in 0..5 {
        let mut changed = current.clone();
        match field {
            0 => changed.device_code.push('x'),
            1 => changed.user_code.push('x'),
            2 => changed.verification_uri.push('x'),
            3 => changed.interval += 1,
            _ => changed.expires_in += 1,
        }
        assert!(matches!(
            service.microsoft_finish(&changed).await,
            Err(SessionError::Login(LoginError::UnknownCode))
        ));
    }
    service.cancel_login().unwrap();
    assert!(matches!(
        service.microsoft_finish(&current).await,
        Err(SessionError::Login(LoginError::UnknownCode))
    ));
    assert!(!context
        .provider
        .calls
        .lock()
        .unwrap()
        .contains(&ProviderOperation::MicrosoftFinish));
    no_publication(&context);
}

#[tokio::test]
async fn overlapping_starts_cannot_install_a_delayed_old_response() {
    let context = Context::new();
    let service = context.service();
    let release = context.provider.pause();
    let mut old = Box::pin(service.microsoft_start());
    assert!(poll_fn(|cx| Poll::Ready(old.as_mut().poll(cx).is_pending())).await);
    let current = service.microsoft_start().await.unwrap();
    release.send(()).unwrap();
    assert!(matches!(
        old.await,
        Err(SessionError::Login(LoginError::Stale))
    ));
    assert_eq!(
        service.microsoft_finish(&current).await.unwrap().id,
        "microsoft:fixture-id"
    );
}

#[tokio::test]
async fn cancellation_while_starting_does_not_register_the_delayed_code() {
    let context = Context::new();
    let service = context.service();
    let release = context.provider.pause();
    let mut request = Box::pin(service.microsoft_start());
    assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
    service.cancel_login().unwrap();
    release.send(()).unwrap();
    assert!(matches!(
        request.await,
        Err(SessionError::Login(LoginError::Stale))
    ));
    no_publication(&context);
}

#[tokio::test]
async fn new_start_invalidates_an_already_polling_completion() {
    let context = Context::new();
    let service = context.service();
    let old = service.microsoft_start().await.unwrap();
    let release = context.provider.pause();
    let mut request = Box::pin(service.microsoft_finish(&old));
    assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
    let current = service.microsoft_start().await.unwrap();
    release.send(()).unwrap();
    assert!(matches!(request.await, Err(SessionError::Cancelled)));
    no_publication(&context);
    assert!(service.microsoft_finish(&current).await.is_ok());
}

#[tokio::test]
async fn duplicate_and_replayed_completions_cannot_publish_twice() {
    let context = Context::new();
    let service = context.service();
    let code = service.microsoft_start().await.unwrap();
    let release = context.provider.pause();
    let mut first = Box::pin(service.microsoft_finish(&code));
    assert!(poll_fn(|cx| Poll::Ready(first.as_mut().poll(cx).is_pending())).await);
    assert!(matches!(
        service.microsoft_finish(&code).await,
        Err(SessionError::Login(LoginError::AlreadyClaimed))
    ));
    release.send(()).unwrap();
    first.await.unwrap();
    let bytes = context.bytes();
    assert!(matches!(
        service.microsoft_finish(&code).await,
        Err(SessionError::Login(LoginError::AlreadyClaimed))
    ));
    assert_eq!(context.bytes(), bytes);
    assert_eq!(context.provider.finish_lifetimes.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn abandoned_and_failed_completions_require_a_fresh_code() {
    let mut context = Context::new();
    let code = context.service().microsoft_start().await.unwrap();
    let release = context.provider.pause();
    {
        let service = context.service();
        let mut request = Box::pin(service.microsoft_finish(&code));
        assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
        drop(request);
    }
    assert!(release.send(()).is_err());
    assert!(matches!(
        context.service().microsoft_finish(&code).await,
        Err(SessionError::Login(LoginError::AlreadyClaimed))
    ));
    context.provider.fail = Some(ProviderOperation::MicrosoftFinish);
    let code = context.service().microsoft_start().await.unwrap();
    assert!(matches!(
        context.service().microsoft_finish(&code).await,
        Err(SessionError::Provider(_))
    ));
    assert!(matches!(
        context.service().microsoft_finish(&code).await,
        Err(SessionError::Login(LoginError::AlreadyClaimed))
    ));
    no_publication(&context);
}

#[tokio::test]
async fn expiry_prevents_polling_and_delayed_completion_uses_remaining_lifetime() {
    let context = Context::new();
    let service = context.service();
    let expired = service.microsoft_start().await.unwrap();
    context.elapsed_seconds.store(900, Ordering::SeqCst);
    assert!(matches!(
        service.microsoft_finish(&expired).await,
        Err(SessionError::Login(LoginError::Expired))
    ));
    assert!(context.provider.finish_lifetimes.lock().unwrap().is_empty());
    no_publication(&context);
    let current = service.microsoft_start().await.unwrap();
    context.elapsed_seconds.store(1000, Ordering::SeqCst);
    service.microsoft_finish(&current).await.unwrap();
    assert_eq!(*context.provider.finish_lifetimes.lock().unwrap(), [800]);
}

#[tokio::test]
async fn a_code_already_expired_on_receipt_is_not_registered() {
    let context = Context::new();
    let service = context.service();
    let release = context.provider.pause();
    let mut request = Box::pin(service.microsoft_start());
    assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
    context.elapsed_seconds.store(900, Ordering::SeqCst);
    release.send(()).unwrap();
    assert!(matches!(
        request.await,
        Err(SessionError::Login(LoginError::Expired))
    ));
    no_publication(&context);
}

#[tokio::test]
async fn malformed_microsoft_response_is_rejected_before_publication_and_preserves_pending_grant() {
    let malformed_sessions = vec![
        MinecraftSession {
            access_token: "rotated-access".to_owned(),
            refresh_token: "rotated-refresh".to_owned(),
            expires_at: 9999,
            uuid: "".to_owned(),
            username: "UpdatedPlayer".to_owned(),
            skin_url: None,
        },
        MinecraftSession {
            access_token: "rotated-access".to_owned(),
            refresh_token: "rotated-refresh".to_owned(),
            expires_at: 9999,
            uuid: "   ".to_owned(),
            username: "UpdatedPlayer".to_owned(),
            skin_url: None,
        },
        MinecraftSession {
            access_token: "rotated-access".to_owned(),
            refresh_token: "rotated-refresh".to_owned(),
            expires_at: 9999,
            uuid: "fixture-id".to_owned(),
            username: "".to_owned(),
            skin_url: None,
        },
        MinecraftSession {
            access_token: "rotated-access".to_owned(),
            refresh_token: "rotated-refresh".to_owned(),
            expires_at: 9999,
            uuid: "fixture-id".to_owned(),
            username: "   ".to_owned(),
            skin_url: None,
        },
    ];

    for bad_session in malformed_sessions {
        let mut context = Context::new();
        context.provider.malformed_minecraft_session = Some(bad_session);
        let service = context.service();
        let code = service.microsoft_start().await.unwrap();
        let result = service.microsoft_finish(&code).await;

        assert!(matches!(result, Err(SessionError::Validation(_))));
        assert!(context.repository.load().unwrap().is_empty());
        assert!(!context.directory.path().join("accounts.recovery").exists());
        let scope = context.directory.path().join("accounts.json");
        let scope_id = uuid::Uuid::new_v3(
            &uuid::Uuid::NAMESPACE_URL,
            scope.to_string_lossy().as_bytes(),
        );
        let recovery_key = format!("ingot:account-recovery:v1:{scope_id}");
        assert!(context.credentials.read(&recovery_key).unwrap().is_none());
        assert!(context.credentials.read("microsoft:").unwrap().is_none());
        assert!(context
            .credentials
            .read("microsoft:fixture-id")
            .unwrap()
            .is_none());

        let pending = context.service().pending_microsoft_login().unwrap();
        assert!(pending.is_some());
        let pending = pending.unwrap();
        assert_eq!(
            pending.status,
            crate::account::pending_login::PendingLoginStatus::Ready
        );
        let grant_key = format!("ingot:microsoft-pending:v1:{scope_id}");
        let raw = context.credentials.read(&grant_key).unwrap().unwrap();
        assert!(raw.contains("rotated-refresh"));
        assert!(raw.contains(&code.user_code));
    }
}

#[tokio::test]
async fn malformed_microsoft_response_on_resume_is_rejected_and_preserves_rotated_grant() {
    let mut context = Context::new();
    let code = context.service().microsoft_start().await.unwrap();
    context.provider.malformed_minecraft_session = Some(MinecraftSession {
        access_token: "rotated-access".to_owned(),
        refresh_token: "rotated-refresh".to_owned(),
        expires_at: 9999,
        uuid: "".to_owned(),
        username: "UpdatedPlayer".to_owned(),
        skin_url: None,
    });
    let result = context.service().microsoft_finish(&code).await;
    assert!(matches!(result, Err(SessionError::Validation(_))));

    let pending = context
        .service()
        .pending_microsoft_login()
        .unwrap()
        .unwrap();
    let ticket = pending.ticket.clone();

    let resume_result = context.service().resume_microsoft_login(&ticket).await;
    assert!(matches!(resume_result, Err(SessionError::Validation(_))));

    assert!(context.repository.load().unwrap().is_empty());
    assert!(!context.directory.path().join("accounts.recovery").exists());
    let scope = context.directory.path().join("accounts.json");
    let scope_id = uuid::Uuid::new_v3(
        &uuid::Uuid::NAMESPACE_URL,
        scope.to_string_lossy().as_bytes(),
    );
    let recovery_key = format!("ingot:account-recovery:v1:{scope_id}");
    assert!(context.credentials.read(&recovery_key).unwrap().is_none());

    let pending_after = context
        .service()
        .pending_microsoft_login()
        .unwrap()
        .unwrap();
    assert_eq!(pending_after.ticket, ticket);
    let grant_key = format!("ingot:microsoft-pending:v1:{scope_id}");
    let raw = context.credentials.read(&grant_key).unwrap().unwrap();
    assert!(raw.contains("rotated-refresh"));
}
