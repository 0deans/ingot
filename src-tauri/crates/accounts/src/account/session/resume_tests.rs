use super::*;
use crate::account::{
    login::LoginError,
    pending_login::{PendingGrantError, PendingLoginStatus},
};

fn key(context: &Context) -> String {
    let scope = uuid::Uuid::new_v3(
        &uuid::Uuid::NAMESPACE_URL,
        context.repository.path().to_string_lossy().as_bytes(),
    );
    format!("ingot:microsoft-pending:v1:{scope}")
}

fn seed(context: &Context) -> String {
    context
        .recovery
        .with_pending_login(|| {
            context.pending_login.checkpoint(
                &context.credentials,
                &code(),
                "initial-refresh",
                1000,
            )?;
            Ok::<(), SessionError>(())
        })
        .unwrap();
    context
        .service()
        .pending_microsoft_login()
        .unwrap()
        .unwrap()
        .ticket
}

#[tokio::test]
async fn explicit_resume_after_restart_uses_retained_token_and_consumes_ticket_once() {
    let context = Context::new();
    let ticket = seed(&context);
    let metadata = context
        .service()
        .pending_microsoft_login()
        .unwrap()
        .unwrap();
    assert_eq!(metadata.status, PendingLoginStatus::Ready);
    let json = serde_json::to_string(&metadata).unwrap();
    assert!(!json.contains("refresh"));
    assert!(!json.contains("fixture-device"));
    let login = LoginAttempts::default();
    let mut restarted = context.service();
    restarted.login = &login;
    let profile = restarted.resume_microsoft_login(&ticket).await.unwrap();
    assert_eq!(profile.id, "microsoft:fixture-id");
    assert_eq!(
        *context.provider.refresh_inputs.lock().unwrap(),
        ["initial-refresh"]
    );
    assert!(!context
        .provider
        .calls
        .lock()
        .unwrap()
        .contains(&ProviderOperation::MicrosoftFinish));
    assert!(restarted.pending_microsoft_login().unwrap().is_none());
    assert!(matches!(
        restarted.resume_microsoft_login(&ticket).await,
        Err(SessionError::PendingGrant(PendingGrantError::NotFound))
    ));
    assert_eq!(context.repository.load().unwrap().len(), 1);
}

#[tokio::test]
async fn failed_exchange_retains_rotation_and_retry_does_not_extend_retention() {
    let mut context = Context::new();
    let ticket = seed(&context);
    context.provider.fail = Some(ProviderOperation::MicrosoftExchange);
    assert!(matches!(
        context.service().resume_microsoft_login(&ticket).await,
        Err(SessionError::Provider(_))
    ));
    let pending = context
        .service()
        .pending_microsoft_login()
        .unwrap()
        .unwrap();
    assert_eq!(pending.status, PendingLoginStatus::Ready);
    assert_eq!(pending.created_at, 1000);
    assert_eq!(pending.expires_at, 87400);
    context.provider.fail = None;
    context
        .service()
        .resume_microsoft_login(&ticket)
        .await
        .unwrap();
    assert_eq!(
        *context.provider.refresh_inputs.lock().unwrap(),
        ["initial-refresh", "rotated-refresh"]
    );
}

#[tokio::test]
async fn concurrent_resume_is_rejected_and_dropping_the_owner_releases_its_lease() {
    let context = Context::new();
    let ticket = seed(&context);
    let service = context.service();
    let release = context.provider.pause_exchange();
    let mut first = Box::pin(service.resume_microsoft_login(&ticket));
    assert!(poll_fn(|cx| Poll::Ready(first.as_mut().poll(cx).is_pending())).await);
    assert_eq!(
        service.pending_microsoft_login().unwrap().unwrap().status,
        PendingLoginStatus::Busy
    );
    assert!(matches!(
        service.resume_microsoft_login(&ticket).await,
        Err(SessionError::Login(LoginError::Busy))
    ));
    let generation = context.login.current();
    assert!(matches!(
        service.discard_pending_microsoft_login(&uuid::Uuid::new_v4().to_string()),
        Err(SessionError::PendingGrant(PendingGrantError::NotFound))
    ));
    assert!(!context.login.is_cancelled(generation));
    drop(first);
    assert!(release.send(()).is_err());
    assert_eq!(
        service.pending_microsoft_login().unwrap().unwrap().status,
        PendingLoginStatus::Ready
    );
    service.resume_microsoft_login(&ticket).await.unwrap();
    assert_eq!(
        *context.provider.refresh_inputs.lock().unwrap(),
        ["initial-refresh", "rotated-refresh"]
    );
}

#[tokio::test]
async fn initial_completion_owns_the_grant_until_its_future_finishes_or_is_dropped() {
    let context = Context::new();
    let service = context.service();
    let code = service.microsoft_start().await.unwrap();
    let release = context.provider.pause_exchange();
    let mut initial = Box::pin(service.microsoft_finish(&code));
    assert!(poll_fn(|cx| Poll::Ready(initial.as_mut().poll(cx).is_pending())).await);
    let ticket = service.pending_microsoft_login().unwrap().unwrap().ticket;
    assert!(matches!(
        service.resume_microsoft_login(&ticket).await,
        Err(SessionError::Login(LoginError::Busy))
    ));
    drop(initial);
    assert!(release.send(()).is_err());
    assert!(matches!(
        service.microsoft_finish(&code).await,
        Err(SessionError::Login(LoginError::AlreadyClaimed))
    ));
    service.resume_microsoft_login(&ticket).await.unwrap();
}

#[tokio::test]
async fn expired_legacy_and_wrong_tickets_never_start_a_provider_request() {
    let context = Context::new();
    let ticket = seed(&context);
    for now in [999, 87400] {
        let mut service = context.service();
        service.now = if now == 999 { || 999 } else { || 87400 };
        assert_eq!(
            service.pending_microsoft_login().unwrap().unwrap().status,
            PendingLoginStatus::Expired
        );
        assert!(matches!(
            service.resume_microsoft_login(&ticket).await,
            Err(SessionError::PendingGrant(PendingGrantError::Expired))
        ));
    }
    let record = context.credentials.read(&key(&context)).unwrap().unwrap();
    let legacy = record
        .replace("\"version\":2", "\"version\":1")
        .replace("\"state\":\"AwaitingExchange\",", "");
    context.credentials.put(&key(&context), &legacy);
    let service = context.service();
    assert_eq!(
        service.pending_microsoft_login().unwrap().unwrap().status,
        PendingLoginStatus::Legacy
    );
    assert!(matches!(
        service.resume_microsoft_login(&ticket).await,
        Err(SessionError::PendingGrant(PendingGrantError::Legacy))
    ));
    for invalid in [
        "private-device-invalid",
        "00000000-0000-0000-0000-000000000000",
    ] {
        let before = context.login.current();
        let error = service.resume_microsoft_login(invalid).await.unwrap_err();
        assert!(matches!(
            error,
            SessionError::PendingGrant(PendingGrantError::NotFound)
        ));
        assert!(!format!("{error:?}: {error}").contains(invalid));
        assert!(!context.login.is_cancelled(before));
    }
    assert!(context.provider.calls.lock().unwrap().is_empty());
    service.discard_pending_microsoft_login(&ticket).unwrap();
    assert!(service.pending_microsoft_login().unwrap().is_none());
}

#[tokio::test]
async fn failed_rotation_checkpoint_stops_exchange_and_preserves_the_previous_record() {
    let context = Context::new();
    let ticket = seed(&context);
    let original = context.credentials.read(&key(&context)).unwrap();
    context.credentials.fail_write_for(Some(&key(&context)));
    assert!(matches!(
        context.service().resume_microsoft_login(&ticket).await,
        Err(SessionError::ResumeCheckpoint(_))
    ));
    assert_eq!(context.credentials.read(&key(&context)).unwrap(), original);
    assert!(!context
        .provider
        .calls
        .lock()
        .unwrap()
        .contains(&ProviderOperation::MicrosoftExchange));
    assert!(context.repository.load().unwrap().is_empty());
}

#[tokio::test]
async fn cancellation_or_explicit_discard_stops_late_refresh_and_exchange_results() {
    for pause_refresh in [true, false] {
        let context = Context::new();
        let ticket = seed(&context);
        let service = context.service();
        let release = if pause_refresh {
            context.provider.pause()
        } else {
            context.provider.pause_exchange()
        };
        let mut request = Box::pin(service.resume_microsoft_login(&ticket));
        assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
        if pause_refresh {
            service.cancel_login().unwrap();
        } else {
            service.discard_pending_microsoft_login(&ticket).unwrap();
        }
        release.send(()).unwrap();
        assert!(matches!(request.await, Err(SessionError::Cancelled)));
        assert!(service.pending_microsoft_login().unwrap().is_none());
        assert!(context.repository.load().unwrap().is_empty());
        assert!(context
            .credentials
            .read("microsoft:fixture-id")
            .unwrap()
            .is_none());
    }
}

#[tokio::test]
async fn superseded_resume_cannot_overwrite_a_new_ticket_or_release_its_lease() {
    let context = Context::new();
    let ticket = seed(&context);
    let service = context.service();
    let release = context.provider.pause();
    let mut old = Box::pin(service.resume_microsoft_login(&ticket));
    assert!(poll_fn(|cx| Poll::Ready(old.as_mut().poll(cx).is_pending())).await);
    let new_code = service.microsoft_start().await.unwrap();
    let new_release = context.provider.pause_exchange();
    let mut newer = Box::pin(service.microsoft_finish(&new_code));
    assert!(poll_fn(|cx| Poll::Ready(newer.as_mut().poll(cx).is_pending())).await);
    let next = service.pending_microsoft_login().unwrap().unwrap();
    assert_ne!(next.ticket, ticket);
    release.send(()).unwrap();
    assert!(matches!(old.await, Err(SessionError::Cancelled)));
    assert_eq!(
        service.pending_microsoft_login().unwrap().unwrap().status,
        PendingLoginStatus::Busy
    );
    assert!(matches!(
        service.resume_microsoft_login(&next.ticket).await,
        Err(SessionError::Login(LoginError::Busy))
    ));
    new_release.send(()).unwrap();
    newer.await.unwrap();
}

#[tokio::test]
async fn pending_account_handoff_recovers_before_resume_and_does_not_reactivate_after_removal() {
    let context = Context::new();
    let ticket = seed(&context);
    let service = context.service();
    let release = context.provider.pause_exchange();
    let mut request = Box::pin(service.resume_microsoft_login(&ticket));
    assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
    context.credentials.fail_write_for(Some(&key(&context)));
    release.send(()).unwrap();
    assert!(matches!(
        request.await,
        Err(SessionError::Recovery(RecoveryError::PendingGrant(_)))
    ));
    context.credentials.fail_write_for(None);
    let calls = context.provider.calls.lock().unwrap().len();
    assert!(matches!(
        service.resume_microsoft_login(&ticket).await,
        Err(SessionError::PendingGrant(PendingGrantError::NotFound))
    ));
    assert_eq!(context.provider.calls.lock().unwrap().len(), calls);
    let operation = context
        .operations
        .acquire("microsoft:fixture-id")
        .await
        .unwrap();
    context
        .recovery
        .remove(&context.credentials, &context.repository, &operation)
        .unwrap();
    assert!(service.pending_microsoft_login().unwrap().is_none());
    assert!(matches!(
        service.resume_microsoft_login(&ticket).await,
        Err(SessionError::PendingGrant(PendingGrantError::NotFound))
    ));
    assert!(context.repository.load().unwrap().is_empty());
}

#[tokio::test]
async fn failed_discard_acknowledgement_still_invalidates_the_owner() {
    let context = Context::new();
    let ticket = seed(&context);
    let service = context.service();
    let release = context.provider.pause_exchange();
    let mut request = Box::pin(service.resume_microsoft_login(&ticket));
    assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
    context.credentials.fail_write_for(Some(&key(&context)));
    assert!(matches!(
        service.discard_pending_microsoft_login(&ticket),
        Err(SessionError::PendingGrant(_))
    ));
    release.send(()).unwrap();
    assert!(matches!(request.await, Err(SessionError::Cancelled)));
    assert!(context.repository.load().unwrap().is_empty());
    context.credentials.fail_write_for(None);
    service.discard_pending_microsoft_login(&ticket).unwrap();
}

#[tokio::test]
async fn failed_refresh_preserves_the_retained_grant_and_releases_resume_ownership() {
    let mut context = Context::new();
    let ticket = seed(&context);
    let original = context.credentials.read(&key(&context)).unwrap();
    context.provider.fail = Some(ProviderOperation::MicrosoftRefresh);
    assert!(matches!(
        context.service().resume_microsoft_login(&ticket).await,
        Err(SessionError::Provider(_))
    ));
    assert_eq!(context.credentials.read(&key(&context)).unwrap(), original);
    assert_eq!(
        context
            .service()
            .pending_microsoft_login()
            .unwrap()
            .unwrap()
            .status,
        PendingLoginStatus::Ready
    );
    context.provider.fail = None;
    context
        .service()
        .resume_microsoft_login(&ticket)
        .await
        .unwrap();
}
