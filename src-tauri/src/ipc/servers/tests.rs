use super::*;
use crate::app::{AppPaths, AppState};
use serde_json::{json, Value};
use tauri::{ipc::InvokeBody, test::MockRuntime, webview::InvokeRequest, Manager};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn server_mutations_reject_busy_owners_before_touching_files_or_metadata() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("servers.json");
    let config = json!({
        "id": "server", "name": "Server", "core": "paper", "gameVersion": "1.20.1",
        "port": 25565, "memoryMinMb": 1024, "memoryMaxMb": 2048, "createdAt": 0
    });
    let bytes = serde_json::to_vec(&[&config]).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    let world = root.path().join("servers/server/world.dat");
    std::fs::create_dir_all(world.parent().unwrap()).unwrap();
    std::fs::write(&world, b"keep world").unwrap();
    let app = tauri::test::mock_builder()
        .invoke_handler(crate::ipc::router::<MockRuntime>().into_handler())
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    app.manage(AppState::new(AppPaths::new(root.path().to_owned())).unwrap());
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let manager = &crate::app::state(app.handle()).servers;
    for sleeping in [false, true] {
        let ownership = if sleeping {
            manager
                .set_server_status("server", server::ServerStatus::Sleeping)
                .await;
            None
        } else {
            Some(manager.reserve_maintenance("server").await.unwrap())
        };
        for (command, arguments) in [
            (
                "TauRPC__servers.delete_server",
                json!({"server_id": "server", "delete_files": true}),
            ),
            ("TauRPC__servers.update_server", json!({"server": config})),
        ] {
            let error: Value = tauri::test::get_ipc_response(
                &webview,
                InvokeRequest {
                    cmd: command.to_owned(),
                    callback: tauri::ipc::CallbackFn(0),
                    error: tauri::ipc::CallbackFn(1),
                    url: crate::ipc::test_invoke_url(),
                    body: InvokeBody::Json(arguments),
                    headers: Default::default(),
                    invoke_key: tauri::test::INVOKE_KEY.to_owned(),
                },
            )
            .unwrap_err();
            assert_eq!(error["kind"], "conflict");
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
            assert_eq!(std::fs::read(&world).unwrap(), b"keep world");
        }
        drop(ownership);
    }
    assert!(!manager.is_starting("server"));
}
