pub mod account;
pub mod auth;
pub mod ipc;
pub mod keyring_store;
pub mod system;

use ipc::{AppApi, AppApiImpl};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _rt = if tokio::runtime::Handle::try_current().is_err() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        Some(rt)
    } else {
        None
    };
    let _guard = _rt.as_ref().map(|rt| rt.enter());

    let router = taurpc::Router::<tauri::Wry>::new().merge(AppApiImpl.into_handler());

    #[cfg(debug_assertions)]
    let _ = taurpc::Exporter::new().export(&router, "../src/bindings.ts");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(router.into_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
