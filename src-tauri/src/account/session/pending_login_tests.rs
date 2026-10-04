use super::*;
use crate::account::pending_login::PendingGrantError;

fn key(context: &Context) -> String {
    let scope = context.directory.path().join("accounts.json");
    let scope = uuid::Uuid::new_v3(
        &uuid::Uuid::NAMESPACE_URL,
        scope.to_string_lossy().as_bytes(),
    );
    format!("ingot:microsoft-pending:v1:{scope}")
}
fn retained(context: &Context) -> Option<String> {
    context.credentials.read(&key(context)).unwrap()
}

#[tokio::test]
async fn failed_exchange_retains_grant_without_creating_or_activating_an_account_after_restart() {
    let mut context = Context::new();
    context.provider.fail = Some(ProviderOperation::MicrosoftExchange);
    let service = context.service();
    let code = service.microsoft_start().await.unwrap();
    assert!(matches!(
        service.microsoft_finish(&code).await,
        Err(SessionError::Provider(_))
    ));
    let value = retained(&context).unwrap();
    assert!(value.contains("rotated-refresh"));
    assert!(!value.contains("fixture-ms-access"));
    assert!(context.repository.load().unwrap().is_empty());
    assert_eq!(
        std::fs::read_dir(context.directory.path()).unwrap().count(),
        0
    );
    let login = LoginAttempts::default();
    let grants = PendingLoginGrants::new(&context.directory.path().join("accounts.json"));
    let mut restarted = context.service();
    restarted.login = &login;
    restarted.pending_login = &grants;
    assert!(restarted.profiles().unwrap().is_empty());
    assert!(matches!(
        restarted.microsoft_finish(&code).await,
        Err(SessionError::Login(
            crate::account::login::LoginError::UnknownCode
        ))
    ));
    assert_eq!(retained(&context).unwrap(), value);
    restarted.microsoft_start().await.unwrap();
    assert!(retained(&context).is_none());
}

#[tokio::test]
async fn dropped_exchange_retains_grant_until_explicit_cancellation() {
    let context = Context::new();
    let service = context.service();
    let code = service.microsoft_start().await.unwrap();
    let release = context.provider.pause_exchange();
    let mut request = Box::pin(service.microsoft_finish(&code));
    assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
    assert!(retained(&context).is_some());
    drop(request);
    assert!(release.send(()).is_err());
    service.cancel_login().unwrap();
    assert!(retained(&context).is_none());
    assert!(context.repository.load().unwrap().is_empty());
}

#[tokio::test]
async fn checkpoint_failure_stops_before_exchange() {
    let context = Context::new();
    let service = context.service();
    let code = service.microsoft_start().await.unwrap();
    context.credentials.fail_writes();
    assert!(matches!(
        service.microsoft_finish(&code).await,
        Err(SessionError::PendingGrant(PendingGrantError::Credentials(
            _
        )))
    ));
    assert!(!context
        .provider
        .calls
        .lock()
        .unwrap()
        .contains(&ProviderOperation::MicrosoftExchange));
    assert!(retained(&context).is_none());
    assert!(context.repository.load().unwrap().is_empty());
}

#[tokio::test]
async fn cancellation_cleanup_failure_still_prevents_publication_and_reports_retained_grant() {
    let context = Context::new();
    let service = context.service();
    let code = service.microsoft_start().await.unwrap();
    let release = context.provider.pause_exchange();
    let mut request = Box::pin(service.microsoft_finish(&code));
    assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
    context.credentials.fail_write_for(Some(&key(&context)));
    assert!(matches!(
        service.cancel_login(),
        Err(SessionError::PendingGrant(_))
    ));
    release.send(()).unwrap();
    assert!(matches!(request.await, Err(SessionError::Cancelled)));
    assert!(context.repository.load().unwrap().is_empty());
    assert!(retained(&context).is_some());
    context.credentials.fail_write_for(None);
    service.cancel_login().unwrap();
    assert!(retained(&context).is_none());
}

#[tokio::test]
async fn new_start_discards_the_old_grant_and_old_exchange_cannot_replace_it() {
    let context = Context::new();
    let service = context.service();
    let code = service.microsoft_start().await.unwrap();
    let release = context.provider.pause_exchange();
    let mut request = Box::pin(service.microsoft_finish(&code));
    assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
    assert!(retained(&context).is_some());
    let new_code = service.microsoft_start().await.unwrap();
    assert!(retained(&context).is_none());
    release.send(()).unwrap();
    assert!(matches!(request.await, Err(SessionError::Cancelled)));
    assert!(context.repository.load().unwrap().is_empty());
    service.microsoft_finish(&new_code).await.unwrap();
    assert!(retained(&context).is_none());
}

#[tokio::test]
async fn handoff_failure_blocks_publication_and_restart_recovers_without_redeeming_again() {
    let context = Context::new();
    let service = context.service();
    let code = service.microsoft_start().await.unwrap();
    let release = context.provider.pause_exchange();
    let mut request = Box::pin(service.microsoft_finish(&code));
    assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
    context.credentials.fail_write_for(Some(&key(&context)));
    release.send(()).unwrap();
    assert!(matches!(
        request.await,
        Err(SessionError::Recovery(
            crate::account::recovery::RecoveryError::PendingGrant(_)
        ))
    ));
    assert!(context.repository.load().unwrap().is_empty());
    assert!(context
        .credentials
        .read("microsoft:fixture-id")
        .unwrap()
        .is_none());
    assert!(retained(&context).is_some());
    context.credentials.fail_write_for(None);
    let calls = context.provider.calls.lock().unwrap().len();
    let restarted_recovery =
        AccountRecovery::new(context.directory.path().join("accounts.recovery"));
    let mut restarted = context.service();
    restarted.recovery = &restarted_recovery;
    assert_eq!(restarted.profiles().unwrap()[0].id, "microsoft:fixture-id");
    assert_eq!(
        credentials::load(&context.credentials, "microsoft:fixture-id")
            .unwrap()
            .access_token,
        "rotated-access"
    );
    assert_eq!(context.provider.calls.lock().unwrap().len(), calls);
    assert_eq!(
        retained(&context).as_deref(),
        Some("ingot-microsoft-pending-discarded-v1")
    );
    assert!(matches!(
        service.microsoft_finish(&code).await,
        Err(SessionError::Login(
            crate::account::login::LoginError::AlreadyClaimed
        ))
    ));
    service.cancel_login().unwrap();
    assert!(retained(&context).is_none());
    assert_eq!(context.repository.load().unwrap().len(), 1);
}
