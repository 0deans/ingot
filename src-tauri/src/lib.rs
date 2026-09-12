pub mod account;
pub mod auth;
pub mod ipc;
pub mod keyring_store;

use ipc::{AppApi, AppApiImpl};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let router = taurpc::Router::<tauri::Wry>::new().merge(AppApiImpl.into_handler());

    #[cfg(debug_assertions)]
    let _ = taurpc::Exporter::new().export(&router, "../src/bindings.ts");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(router.into_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
