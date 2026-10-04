use serde_json::{json, Value};
use tauri::{ipc::InvokeBody, test::MockRuntime, webview::InvokeRequest, Listener, Manager};

use super::{
    events::TauRpcEventsApiEventTrigger,
    settings::{SettingsApi, SettingsApiImpl},
};
use crate::{
    app::{AppPaths, AppState},
    settings::MemorySettings,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn corrupt_account_storage_is_rejected_and_cannot_be_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    let corrupt = b"[{broken-account-data";
    std::fs::write(&path, corrupt).unwrap();
    let app = tauri::test::mock_builder()
        .invoke_handler(super::router::<MockRuntime>().into_handler())
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    app.manage(AppState::new(AppPaths::new(dir.path().to_owned())).unwrap());
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    for (command, arguments) in [
        ("TauRPC__accounts.get_accounts", json!({})),
        (
            "TauRPC__accounts.add_offline_account",
            json!({"username":"Player"}),
        ),
        (
            "TauRPC__accounts.reorder_accounts",
            json!({"account_ids":[]}),
        ),
        (
            "TauRPC__accounts.remove_account",
            json!({"account_id":"offline:player"}),
        ),
    ] {
        let error = invoke(&webview, command, arguments).unwrap_err();
        assert!(error.is_string());
        assert_eq!(std::fs::read(&path).unwrap(), corrupt);
    }
}

fn invoke(
    webview: &tauri::WebviewWindow<MockRuntime>,
    command: &str,
    body: Value,
) -> Result<Value, Value> {
    tauri::test::get_ipc_response(
        webview,
        InvokeRequest {
            cmd: command.to_owned(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "http://tauri.localhost".parse().unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_owned(),
        },
    )
    .map(|response| response.deserialize::<Value>().unwrap())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn credential_status_route_distinguishes_missing_corrupt_and_unavailable_vault_data() {
    use crate::account::credentials::tests::FakeCredentialStore;
    let dir = tempfile::tempdir().unwrap();
    let app = tauri::test::mock_builder()
        .invoke_handler(super::router::<MockRuntime>().into_handler())
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let mut state = AppState::new(AppPaths::new(dir.path().to_owned())).unwrap();
    let vault = FakeCredentialStore::default();
    state.credentials = Box::new(vault.clone());
    app.manage(state);
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let arguments = json!({"account_id":"ely:test"});
    assert_eq!(
        invoke(
            &webview,
            "TauRPC__skins.has_ely_web_credentials",
            arguments.clone()
        )
        .unwrap(),
        false
    );
    vault.put(
        "ely:test",
        r#"{"accessToken":"fixture-access","clientToken":"c","password":"fixture-password"}"#,
    );
    assert_eq!(
        invoke(
            &webview,
            "TauRPC__skins.has_ely_web_credentials",
            arguments.clone()
        )
        .unwrap(),
        true
    );
    vault.put("ely:test", "corrupt-credential-secret");
    let error = invoke(
        &webview,
        "TauRPC__skins.has_ely_web_credentials",
        arguments.clone(),
    )
    .unwrap_err();
    assert!(error.is_string());
    assert!(!error
        .as_str()
        .unwrap()
        .contains("corrupt-credential-secret"));
    vault.fail_reads();
    let error = invoke(&webview, "TauRPC__skins.has_ely_web_credentials", arguments).unwrap_err();
    assert!(error.is_string());
    assert!(!error.as_str().unwrap().contains("private vault diagnostic"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_namespace_dispatches_and_preserves_errors_and_storage() {
    let dir = tempfile::tempdir().unwrap();
    let router = taurpc::Router::<MockRuntime>::new().merge(SettingsApiImpl.into_handler());
    let app = tauri::test::mock_builder()
        .invoke_handler(router.into_handler())
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    app.manage(AppState::new(AppPaths::new(dir.path().to_owned())).unwrap());
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let memory = invoke(
        &webview,
        "TauRPC__settings.set_memory_settings",
        json!({"min_ram_mb":8192,"max_ram_mb":1024}),
    )
    .unwrap();
    assert_eq!(memory, json!({"minRamMb":1024,"maxRamMb":8192}));
    assert_eq!(
        invoke(&webview, "TauRPC__settings.get_memory_settings", json!({})).unwrap(),
        memory
    );

    let saved = std::fs::read(dir.path().join("settings.json")).unwrap();
    let error = invoke(
        &webview,
        "TauRPC__settings.set_memory_settings",
        json!({"min_ram_mb":0,"max_ram_mb":4096}),
    )
    .unwrap_err();
    assert!(
        error.is_string(),
        "The transitional wire error must remain a string"
    );
    assert_eq!(
        std::fs::read(dir.path().join("settings.json")).unwrap(),
        saved
    );
    assert!(invoke(
        &webview,
        "TauRPC__set_memory_settings",
        json!({"min_ram_mb":2048,"max_ram_mb":4096})
    )
    .is_err());
}

#[test]
fn event_trigger_uses_the_namespaced_route_and_listener_can_be_removed() {
    let app = tauri::test::mock_app();
    let (sender, receiver) = std::sync::mpsc::channel();
    let listener = app.listen("TauRpc_event", move |event| {
        sender
            .send(serde_json::from_str::<Value>(event.payload()).unwrap())
            .unwrap();
    });
    let trigger = TauRpcEventsApiEventTrigger::new(app.handle().clone());
    trigger
        .on_memory_changed(MemorySettings::default())
        .unwrap();
    let event = receiver.try_recv().unwrap();
    assert_eq!(event["event_name"], "events.on_memory_changed");
    assert_eq!(
        event["event"]["input_type"],
        json!({"minRamMb":2048,"maxRamMb":4096})
    );
    app.unlisten(listener);
    trigger
        .on_memory_changed(MemorySettings::default())
        .unwrap();
    assert!(receiver.try_recv().is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn application_shutdown_uses_managed_state_and_existing_root_commands_share_it() {
    let dir = tempfile::tempdir().unwrap();
    let router = super::router::<MockRuntime>();
    let app = tauri::test::mock_builder()
        .invoke_handler(router.into_handler())
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let state = AppState::new(AppPaths::new(dir.path().to_owned())).unwrap();
    let servers = state.servers.clone();
    servers
        .set_server_status("sleeping", crate::server::ServerStatus::Sleeping)
        .await;
    app.manage(state);
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    assert_eq!(
        invoke(&webview, "TauRPC__app.stop_all_servers", json!({})).unwrap(),
        Value::Null
    );
    assert_eq!(
        servers.get_server_status("sleeping").await,
        crate::server::ServerStatus::Stopped
    );
    assert_eq!(
        invoke(&webview, "TauRPC__get_running_servers", json!({})).unwrap(),
        json!([])
    );
    assert_eq!(
        invoke(&webview, "TauRPC__app.greet", json!({"name":"test"})).unwrap(),
        "Hello, test! You've been greeted from Rust via TauRPC!"
    );
    assert!(invoke(&webview, "TauRPC__stop_all_servers", json!({})).is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn account_and_skin_routes_preserve_profiles_arguments_and_rejections() {
    let dir = tempfile::tempdir().unwrap();
    let app = tauri::test::mock_builder()
        .invoke_handler(super::router::<MockRuntime>().into_handler())
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    app.manage(AppState::new(AppPaths::new(dir.path().to_owned())).unwrap());
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    assert_eq!(
        invoke(&webview, "TauRPC__accounts.get_accounts", json!({})).unwrap(),
        json!([])
    );
    let profile = invoke(
        &webview,
        "TauRPC__accounts.add_offline_account",
        json!({"username":"  Player  "}),
    )
    .unwrap();
    assert_eq!(profile["id"], "offline:player");
    assert_eq!(profile["username"], "Player");
    assert_eq!(profile["accountType"], "offline");
    assert_eq!(profile["isActive"], true);
    assert!(profile["createdAt"].is_u64());
    assert_eq!(
        invoke(
            &webview,
            "TauRPC__accounts.get_active_account_token",
            json!({})
        )
        .unwrap(),
        "offline"
    );
    let saved = std::fs::read(dir.path().join("accounts.json")).unwrap();
    assert!(invoke(
        &webview,
        "TauRPC__accounts.set_active_account",
        json!({"account_id":"missing"})
    )
    .unwrap_err()
    .is_string());
    assert_eq!(
        std::fs::read(dir.path().join("accounts.json")).unwrap(),
        saved
    );
    assert_eq!(
        invoke(&webview, "TauRPC__accounts.get_accounts", json!({})).unwrap(),
        json!([profile])
    );
    let attempt = crate::app::state(app.handle()).account_login.current();
    assert_eq!(
        invoke(
            &webview,
            "TauRPC__accounts.microsoft_login_cancel",
            json!({})
        )
        .unwrap(),
        Value::Null
    );
    assert!(crate::app::state(app.handle())
        .account_login
        .is_cancelled(attempt));
    assert!(invoke(
        &webview,
        "TauRPC__skins.get_skin_data_url",
        json!({"skin_url":""})
    )
    .unwrap_err()
    .is_string());
    assert!(invoke(
        &webview,
        "TauRPC__skins.upload_microsoft_skin",
        json!({"account_id":"unused", "image_base64":"!invalid!", "is_slim":true})
    )
    .unwrap_err()
    .is_string());
    assert!(invoke(&webview, "TauRPC__get_accounts", json!({})).is_err());
    assert!(invoke(
        &webview,
        "TauRPC__get_skin_data_url",
        json!({"skin_url":""})
    )
    .is_err());
}
