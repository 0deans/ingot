use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_sign_in_routes_preserve_metadata_named_tickets_and_string_rejections() {
    let dir = tempfile::tempdir().unwrap();
    let app = tauri::test::mock_builder()
        .invoke_handler(super::super::router::<MockRuntime>().into_handler())
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let mut state = AppState::new(AppPaths::new(dir.path().to_owned())).unwrap();
    let vault = crate::account::credentials::tests::FakeCredentialStore::default();
    state.credentials = Box::new(vault.clone());
    state
        .pending_login
        .checkpoint(
            &vault,
            &crate::auth::microsoft::MicrosoftDeviceCode {
                user_code: "fixture-user".into(),
                verification_uri: "https://fixture.invalid/link".into(),
                device_code: "private-device".into(),
                interval: 1,
                expires_in: 900,
            },
            "private-refresh",
            1000,
        )
        .unwrap();
    app.manage(state);
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let pending = invoke(
        &webview,
        "TauRPC__accounts.get_pending_microsoft_login",
        json!({}),
    )
    .unwrap();
    assert_eq!(pending["status"], "Expired");
    assert_eq!(pending["createdAt"], 1000);
    assert_eq!(pending["expiresAt"], 87400);
    assert!(pending["ticket"].is_string());
    assert!(!pending.to_string().contains("private-"));
    let generation = crate::app::state(app.handle()).account_login.current();
    let invalid = invoke(
        &webview,
        "TauRPC__accounts.discard_pending_microsoft_login",
        json!({"ticket":"private-invalid"}),
    )
    .unwrap_err();
    assert_eq!(invalid["kind"], "notFound");
    assert!(!invalid.to_string().contains("private-invalid"));
    assert!(!crate::app::state(app.handle())
        .account_login
        .is_cancelled(generation));
    let resume_err = invoke(
        &webview,
        "TauRPC__accounts.resume_microsoft_login",
        json!({"ticket":pending["ticket"]}),
    )
    .unwrap_err();
    assert_eq!(resume_err["kind"], "invalidInput");
    assert_eq!(
        invoke(
            &webview,
            "TauRPC__accounts.discard_pending_microsoft_login",
            json!({"ticket":pending["ticket"]})
        )
        .unwrap(),
        Value::Null
    );
    assert_eq!(
        invoke(
            &webview,
            "TauRPC__accounts.get_pending_microsoft_login",
            json!({})
        )
        .unwrap(),
        Value::Null
    );
    assert!(!dir.path().join("accounts.json").exists());
}
