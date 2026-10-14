use super::*;
use crate::account::{
    credentials::{self, tests::FakeCredentialStore, CredentialStore},
    login::LoginAttempts,
    operations::AccountOperations,
    pending_login::PendingLoginGrants,
    provider::{
        AuthenticateResponse, Cancelled, ProviderError, ProviderFuture, ProviderOperation,
        RefreshResponse, SessionProvider, TokenValidity,
    },
    recovery::{AccountRecovery, RecoveryError},
    repository::AccountRepository,
    AccountSecrets,
};
use crate::auth::microsoft::{MicrosoftDeviceCode, MicrosoftGrant, MinecraftSession};
use std::{
    future::{poll_fn, Future},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    task::Poll,
    time::Duration,
};
use tokio::sync::oneshot;

const AUTH: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/ely-session-authenticate.json"
));
const REFRESH: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/ely-session-refresh.json"
));

struct Provider {
    fail: Option<ProviderOperation>,
    validity: TokenValidity,
    missing_profile: bool,
    malformed_ely_profile: Option<crate::account::provider::ProfileInfo>,
    malformed_minecraft_session: Option<MinecraftSession>,
    gate: Mutex<Option<oneshot::Receiver<()>>>,
    calls: Mutex<Vec<ProviderOperation>>,
    finish_lifetimes: Mutex<Vec<u32>>,
    exchange_gate: Mutex<Option<oneshot::Receiver<()>>>,
    refresh_inputs: Mutex<Vec<String>>,
}
impl Default for Provider {
    fn default() -> Self {
        Self {
            fail: None,
            validity: TokenValidity::Valid,
            missing_profile: false,
            malformed_ely_profile: None,
            malformed_minecraft_session: None,
            gate: Mutex::new(None),
            calls: Mutex::new(Vec::new()),
            finish_lifetimes: Mutex::new(Vec::new()),
            exchange_gate: Mutex::new(None),
            refresh_inputs: Mutex::new(Vec::new()),
        }
    }
}
impl Provider {
    fn called(&self, operation: ProviderOperation) -> Result<(), ProviderError> {
        self.calls.lock().unwrap().push(operation);
        if self.fail == Some(operation) {
            return Err(ProviderError::Rejected {
                operation,
                status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
            });
        }
        Ok(())
    }
    async fn wait(&self) {
        let gate = self.gate.lock().unwrap().take();
        if let Some(gate) = gate {
            gate.await.unwrap();
        }
    }
    fn pause(&self) -> oneshot::Sender<()> {
        let (release, gate) = oneshot::channel();
        *self.gate.lock().unwrap() = Some(gate);
        release
    }
    fn pause_exchange(&self) -> oneshot::Sender<()> {
        let (release, gate) = oneshot::channel();
        *self.exchange_gate.lock().unwrap() = Some(gate);
        release
    }
}
fn session() -> MinecraftSession {
    MinecraftSession {
        access_token: "rotated-access".to_owned(),
        refresh_token: "rotated-refresh".to_owned(),
        expires_at: 9999,
        uuid: "fixture-id".to_owned(),
        username: "UpdatedPlayer".to_owned(),
        skin_url: Some("https://fixture.invalid/skin".to_owned()),
    }
}
fn code() -> MicrosoftDeviceCode {
    MicrosoftDeviceCode {
        user_code: "fixture-code".to_owned(),
        verification_uri: "https://fixture.invalid/link".to_owned(),
        device_code: "fixture-device".to_owned(),
        interval: 1,
        expires_in: 900,
    }
}
impl SessionProvider for Provider {
    fn ely_login<'a>(
        &'a self,
        username: &'a str,
        password: &'a str,
        client: &'a str,
    ) -> ProviderFuture<'a, AuthenticateResponse> {
        Box::pin(async move {
            self.called(ProviderOperation::ElySignIn)?;
            assert_eq!(username, "FixturePlayer");
            assert!(!password.is_empty());
            assert!(!client.is_empty());
            let mut response: AuthenticateResponse = serde_json::from_str(AUTH).unwrap();
            if self.missing_profile {
                response.selected_profile = None;
            } else if let Some(profile) = &self.malformed_ely_profile {
                response.selected_profile = Some(profile.clone());
            }
            Ok(response)
        })
    }
    fn ely_validate<'a>(&'a self, _: &'a str, _: &'a str) -> ProviderFuture<'a, TokenValidity> {
        Box::pin(async {
            self.called(ProviderOperation::ElyValidate)?;
            Ok(self.validity)
        })
    }
    fn ely_refresh<'a>(&'a self, _: &'a str, _: &'a str) -> ProviderFuture<'a, RefreshResponse> {
        Box::pin(async {
            self.called(ProviderOperation::ElyRefresh)?;
            Ok(serde_json::from_str(REFRESH).unwrap())
        })
    }
    fn microsoft_start(&self) -> ProviderFuture<'_, MicrosoftDeviceCode> {
        Box::pin(async {
            self.called(ProviderOperation::MicrosoftStart)?;
            let mut code = code();
            let starts = self
                .calls
                .lock()
                .unwrap()
                .iter()
                .filter(|operation| **operation == ProviderOperation::MicrosoftStart)
                .count();
            code.device_code = format!("fixture-device-{starts}");
            self.wait().await;
            Ok(code)
        })
    }
    fn microsoft_finish<'a>(
        &'a self,
        code: &'a MicrosoftDeviceCode,
        _: Cancelled<'a>,
    ) -> ProviderFuture<'a, MicrosoftGrant> {
        // Deliberately ignores cancellation: the service must recheck before publishing.
        Box::pin(async move {
            self.called(ProviderOperation::MicrosoftFinish)?;
            self.finish_lifetimes.lock().unwrap().push(code.expires_in);
            self.wait().await;
            Ok(
                MicrosoftGrant::new("fixture-ms-access".to_owned(), "rotated-refresh".to_owned())
                    .unwrap(),
            )
        })
    }
    fn microsoft_refresh<'a>(&'a self, refresh: &'a str) -> ProviderFuture<'a, MicrosoftGrant> {
        Box::pin(async move {
            self.called(ProviderOperation::MicrosoftRefresh)?;
            self.refresh_inputs.lock().unwrap().push(refresh.to_owned());
            self.wait().await;
            Ok(
                MicrosoftGrant::new("fixture-ms-access".to_owned(), "rotated-refresh".to_owned())
                    .unwrap(),
            )
        })
    }
    fn microsoft_exchange<'a>(
        &'a self,
        grant: MicrosoftGrant,
        _: Cancelled<'a>,
    ) -> ProviderFuture<'a, MinecraftSession> {
        Box::pin(async move {
            self.called(ProviderOperation::MicrosoftExchange)?;
            assert_eq!(grant.refresh_token(), "rotated-refresh");
            let gate = self.exchange_gate.lock().unwrap().take();
            if let Some(gate) = gate {
                gate.await.unwrap();
            }
            if let Some(session) = &self.malformed_minecraft_session {
                return Ok(session.clone());
            }
            Ok(session())
        })
    }
}

struct Context {
    directory: tempfile::TempDir,
    repository: AccountRepository,
    credentials: FakeCredentialStore,
    operations: AccountOperations,
    recovery: AccountRecovery,
    login: LoginAttempts,
    pending_login: PendingLoginGrants,
    provider: Provider,
    elapsed_seconds: Arc<AtomicU64>,
}
impl Context {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let elapsed_seconds = Arc::new(AtomicU64::new(0));
        let clock = elapsed_seconds.clone();
        Self {
            repository: AccountRepository::new(directory.path().join("accounts.json")),
            pending_login: PendingLoginGrants::new(&directory.path().join("accounts.json")),
            recovery: AccountRecovery::new(directory.path().join("accounts.recovery")),
            directory,
            credentials: FakeCredentialStore::default(),
            operations: AccountOperations::default(),
            login: LoginAttempts::with_clock(move || {
                Duration::from_secs(clock.load(Ordering::SeqCst))
            }),
            provider: Provider::default(),
            elapsed_seconds,
        }
    }
    fn service(&self) -> SessionService<'_> {
        SessionService {
            repository: &self.repository,
            credentials: &self.credentials,
            operations: &self.operations,
            recovery: &self.recovery,
            provider: &self.provider,
            login: &self.login,
            pending_login: &self.pending_login,
            now: || 1000,
        }
    }
    fn add(&self, kind: &str, expires_at: u64) -> String {
        let id = format!("{kind}:fixture-id");
        self.repository
            .activate(AccountProfile {
                id: id.clone(),
                account_type: kind.to_owned(),
                username: if kind == "offline" {
                    "fixture-id".to_owned()
                } else {
                    "Player".to_owned()
                },
                uuid: "fixture-id".to_owned(),
                skin_url: None,
                is_active: true,
                created_at: 42,
            })
            .unwrap();
        self.credentials.put(
            &id,
            &serde_json::to_string(&AccountSecrets {
                access_token: "old-access".to_owned(),
                client_token: "old-client".to_owned(),
                password: Some("old-password".to_owned()),
                refresh_token: Some("old-refresh".to_owned()),
                expires_at: Some(expires_at),
            })
            .unwrap(),
        );
        id
    }
    fn bytes(&self) -> Vec<u8> {
        std::fs::read(self.directory.path().join("accounts.json")).unwrap()
    }
}

#[tokio::test]
async fn fresh_microsoft_token_needs_no_provider_and_missing_refresh_is_typed() {
    let context = Context::new();
    let id = context.add("microsoft", 1301);
    assert_eq!(
        context.service().active_token().await.unwrap(),
        "old-access"
    );
    assert!(context.provider.calls.lock().unwrap().is_empty());
    context.credentials.put(
        &id,
        &serde_json::to_string(&AccountSecrets::default()).unwrap(),
    );
    assert!(matches!(
        context.service().active_token().await,
        Err(SessionError::MissingRefreshToken)
    ));
    assert!(context.provider.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn validation_failure_does_not_refresh_or_change_existing_session() {
    let mut context = Context::new();
    context.provider.fail = Some(ProviderOperation::ElyValidate);
    let id = context.add("ely", 0);
    let before = context.bytes();
    let error = context.service().active_token().await.unwrap_err();
    assert!(matches!(
        error,
        SessionError::Provider(ProviderError::Rejected {
            operation: ProviderOperation::ElyValidate,
            ..
        })
    ));
    assert_eq!(
        *context.provider.calls.lock().unwrap(),
        [ProviderOperation::ElyValidate]
    );
    assert_eq!(context.bytes(), before);
    assert_eq!(
        credentials::load(&context.credentials, &id)
            .unwrap()
            .access_token,
        "old-access"
    );
}

#[tokio::test]
async fn invalid_ely_token_refreshes_and_preserves_saved_password() {
    let mut context = Context::new();
    context.provider.validity = TokenValidity::Invalid;
    let id = context.add("ely", 0);
    assert_eq!(
        context.service().active_token().await.unwrap(),
        "rotated-access"
    );
    let stored = credentials::load(&context.credentials, &id).unwrap();
    assert_eq!(stored.password.as_deref(), Some("old-password"));
    assert_eq!(stored.client_token, "fixture-client");
    assert!(!String::from_utf8(context.bytes())
        .unwrap()
        .contains("old-password"));
}

#[tokio::test]
async fn provider_refresh_failures_preserve_credentials_and_profile_bytes() {
    for (kind, operation) in [
        ("microsoft", ProviderOperation::MicrosoftRefresh),
        ("ely", ProviderOperation::ElyRefresh),
    ] {
        let mut context = Context::new();
        context.provider.validity = TokenValidity::Invalid;
        context.provider.fail = Some(operation);
        let id = context.add(kind, 0);
        let before = context.bytes();
        let previous = context.credentials.read(&id).unwrap();
        assert!(matches!(
            context.service().active_token().await,
            Err(SessionError::Provider(_))
        ));
        assert_eq!(context.bytes(), before);
        assert_eq!(context.credentials.read(&id).unwrap(), previous);
        assert!(!context.directory.path().join("accounts.recovery").exists());
    }
}

#[tokio::test]
async fn refresh_completion_patches_latest_metadata_without_changing_new_selection() {
    let context = Context::new();
    let id = context.add("microsoft", 0);
    let release = context.provider.pause();
    let service = context.service();
    let mut request = Box::pin(service.active_token());
    assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
    context
        .recovery
        .with_recovered(&context.credentials, &context.repository, || {
            context.repository.activate(AccountProfile {
                id: "offline:other".to_owned(),
                account_type: "offline".to_owned(),
                username: "Other".to_owned(),
                uuid: "other".to_owned(),
                skin_url: None,
                is_active: true,
                created_at: 43,
            })
        })
        .unwrap();
    release.send(()).unwrap();
    assert_eq!(request.await.unwrap(), "rotated-access");
    let profiles = context.repository.load().unwrap();
    assert_eq!(
        profiles.iter().find(|p| p.is_active).unwrap().id,
        "offline:other"
    );
    assert_eq!(
        profiles.iter().find(|p| p.id == id).unwrap().username,
        "UpdatedPlayer"
    );
    assert_eq!(
        credentials::load(&context.credentials, &id)
            .unwrap()
            .refresh_token
            .as_deref(),
        Some("rotated-refresh")
    );
}

#[tokio::test]
async fn storage_failure_after_remote_refresh_keeps_rotation_and_pending_recovery() {
    let context = Context::new();
    let id = context.add("microsoft", 0);
    let release = context.provider.pause();
    let service = context.service();
    let mut request = Box::pin(service.active_token());
    assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
    std::fs::write(context.directory.path().join("accounts.json"), b"[corrupt").unwrap();
    release.send(()).unwrap();
    assert!(matches!(
        request.await,
        Err(SessionError::RotationCheckpoint(RecoveryError::Profile(_)))
    ));
    assert_eq!(
        credentials::load(&context.credentials, &id)
            .unwrap()
            .refresh_token
            .as_deref(),
        Some("rotated-refresh")
    );
    assert!(context.directory.path().join("accounts.recovery").exists());
    assert_eq!(context.bytes(), b"[corrupt");
}

#[tokio::test]
async fn cancellation_during_provider_and_ownership_waits_prevents_login_publication() {
    for wait_for_provider in [true, false] {
        let context = Context::new();
        let code = context.service().microsoft_start().await.unwrap();
        let held = if wait_for_provider {
            None
        } else {
            Some(
                context
                    .operations
                    .acquire("microsoft:fixture-id")
                    .await
                    .unwrap(),
            )
        };
        let release = if wait_for_provider {
            Some(context.provider.pause())
        } else {
            None
        };
        let service = context.service();
        let mut request = Box::pin(service.microsoft_finish(&code));
        assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
        service.cancel_login().unwrap();
        if let Some(release) = release {
            release.send(()).unwrap();
        }
        drop(held);
        assert!(matches!(request.await, Err(SessionError::Cancelled)));
        assert!(context.repository.load().unwrap().is_empty());
        assert!(context
            .credentials
            .read("microsoft:fixture-id")
            .unwrap()
            .is_none());
        assert!(!context.directory.path().join("accounts.recovery").exists());
    }
}

#[tokio::test]
async fn abandoning_refresh_releases_ownership_without_publishing_a_new_session() {
    let context = Context::new();
    let id = context.add("microsoft", 0);
    let release = context.provider.pause();
    let service = context.service();
    let mut request = Box::pin(service.active_token());
    assert!(poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx).is_pending())).await);
    drop(request);
    assert!(release.send(()).is_err());
    let ownership = context.operations.acquire(&id).await.unwrap();
    assert_eq!(
        credentials::load(&context.credentials, &id)
            .unwrap()
            .access_token,
        "old-access"
    );
    drop(ownership);
    assert_eq!(service.active_token().await.unwrap(), "rotated-access");
}

#[tokio::test]
async fn login_fixture_is_published_and_missing_profile_is_rejected_without_writes() {
    let context = Context::new();
    let profile = context
        .service()
        .ely_login(" FixturePlayer ", "password".to_owned())
        .await
        .unwrap();
    assert_eq!(profile.id, "ely:fixture-id");
    assert_eq!(profile.created_at, 1000);
    assert_eq!(
        context.repository.load().unwrap()[0].username,
        "FixturePlayer"
    );
    assert_eq!(
        credentials::load(&context.credentials, &profile.id)
            .unwrap()
            .password
            .as_deref(),
        Some("password")
    );
    let mut missing = Context::new();
    missing.provider.missing_profile = true;
    assert!(matches!(
        missing
            .service()
            .ely_login("FixturePlayer", "password".to_owned())
            .await,
        Err(SessionError::MissingMinecraftProfile)
    ));
    assert!(missing.repository.load().unwrap().is_empty());
    assert!(missing
        .credentials
        .read("ely:fixture-id")
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn malformed_ely_response_is_rejected_before_publication_without_recovery_or_credential_effects(
) {
    let malformed_profiles = vec![
        crate::account::provider::ProfileInfo {
            id: "".to_owned(),
            name: "FixturePlayer".to_owned(),
        },
        crate::account::provider::ProfileInfo {
            id: "   ".to_owned(),
            name: "FixturePlayer".to_owned(),
        },
        crate::account::provider::ProfileInfo {
            id: "fixture-id".to_owned(),
            name: "".to_owned(),
        },
        crate::account::provider::ProfileInfo {
            id: "fixture-id".to_owned(),
            name: "   ".to_owned(),
        },
    ];

    for malformed_profile in malformed_profiles {
        let mut context = Context::new();
        context.provider.malformed_ely_profile = Some(malformed_profile);
        let result = context
            .service()
            .ely_login("FixturePlayer", "password".to_owned())
            .await;
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
        assert!(context
            .credentials
            .read("ely:fixture-id")
            .unwrap()
            .is_none());
        assert!(context.credentials.read("ely:").unwrap().is_none());
    }
}

#[tokio::test]
async fn invalid_inputs_and_corrupt_storage_stop_before_provider_authentication() {
    let context = Context::new();
    assert!(matches!(
        context
            .service()
            .ely_login(" ", "password".to_owned())
            .await,
        Err(SessionError::InvalidLogin)
    ));
    std::fs::write(context.directory.path().join("accounts.json"), b"[corrupt").unwrap();
    assert!(matches!(
        context
            .service()
            .ely_login("FixturePlayer", "password".to_owned())
            .await,
        Err(SessionError::Recovery(_))
    ));
    assert!(context.provider.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn concurrent_token_requests_share_one_refresh_and_use_the_rotated_session() {
    let context = Context::new();
    context.add("microsoft", 0);
    let release = context.provider.pause();
    let service = context.service();
    let mut first = Box::pin(service.active_token());
    let mut second = Box::pin(service.active_token());
    assert!(poll_fn(|cx| Poll::Ready(first.as_mut().poll(cx).is_pending())).await);
    assert!(poll_fn(|cx| Poll::Ready(second.as_mut().poll(cx).is_pending())).await);
    release.send(()).unwrap();
    assert_eq!(first.await.unwrap(), "rotated-access");
    assert_eq!(second.await.unwrap(), "rotated-access");
    assert_eq!(
        *context.provider.calls.lock().unwrap(),
        [
            ProviderOperation::MicrosoftRefresh,
            ProviderOperation::MicrosoftExchange
        ]
    );
}

#[tokio::test]
async fn valid_ely_token_keeps_file_and_vault_unchanged() {
    let context = Context::new();
    let id = context.add("ely", 0);
    let before = context.bytes();
    let previous = context.credentials.read(&id).unwrap();
    assert_eq!(
        context.service().active_token().await.unwrap(),
        "old-access"
    );
    assert_eq!(
        *context.provider.calls.lock().unwrap(),
        [ProviderOperation::ElyValidate]
    );
    assert_eq!(context.bytes(), before);
    assert_eq!(context.credentials.read(&id).unwrap(), previous);
}

#[tokio::test]
async fn starting_a_new_login_invalidates_previous_attempt_even_when_provider_fails() {
    let mut context = Context::new();
    context.provider.fail = Some(ProviderOperation::MicrosoftStart);
    let previous = context.login.current();
    assert!(matches!(
        context.service().microsoft_start().await,
        Err(SessionError::Provider(_))
    ));
    assert!(context.login.is_cancelled(previous));
    assert!(context.repository.load().unwrap().is_empty());
}

#[tokio::test]
async fn microsoft_sign_in_publishes_profile_and_credentials_without_exposing_tokens_in_json() {
    let context = Context::new();
    context.add("offline", 0);
    let code = context.service().microsoft_start().await.unwrap();
    let profile = context.service().microsoft_finish(&code).await.unwrap();
    assert_eq!(profile.id, "microsoft:fixture-id");
    assert_eq!(profile.username, "UpdatedPlayer");
    assert_eq!(profile.created_at, 1000);
    let stored = credentials::load(&context.credentials, &profile.id).unwrap();
    assert_eq!(stored.refresh_token.as_deref(), Some("rotated-refresh"));
    assert_eq!(
        context
            .repository
            .load()
            .unwrap()
            .iter()
            .filter(|p| p.is_active)
            .count(),
        1
    );
    let file = String::from_utf8(context.bytes()).unwrap();
    assert!(!file.contains("rotated-access"));
    assert!(!file.contains("rotated-refresh"));
}

#[path = "login_tests.rs"]
mod login_tests;

#[path = "rotation_tests.rs"]
mod rotation_tests;

#[path = "pending_login_tests.rs"]
mod pending_login_tests;

#[path = "resume_tests.rs"]
mod resume_tests;
