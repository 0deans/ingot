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
async fn settings_namespace_dispatches_and_preserves_errors_and_storage() {
    let dir = tempfile::tempdir().unwrap();
    let router = taurpc::Router::<MockRuntime>::new().merge(SettingsApiImpl.into_handler());
    let app = tauri::test::mock_builder()
        .invoke_handler(router.into_handler())
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    app.manage(AppState::new(AppPaths::new(dir.path().to_owned())));
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
