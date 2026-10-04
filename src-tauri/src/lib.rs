mod app;
mod settings;
#[cfg(test)]
mod ipc_contract;
pub mod account;
pub mod auth;
/// User agent for all HTTP requests; the version comes from Cargo.toml at build time
pub const USER_AGENT: &str = concat!(
    "Ingot-Launcher/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/0deans/ingot)"
);

pub mod ipc;
pub mod keyring_store;
pub mod minecraft;
pub mod running;
pub mod server;
pub mod system;
pub mod version_change;
#[cfg(desktop)]
pub mod tray;

use tauri::Manager;

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

    let router = ipc::router::<tauri::Wry>();

    #[cfg(debug_assertions)]
    if let Err(error) = taurpc::Exporter::new().export(&router, "../src/bindings.ts") {
        eprintln!("[IPC] Failed to export TypeScript bindings: {error}");
    }

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }));
    }

    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .setup(|_app| {
            let paths = app::AppPaths::new(_app.path().app_data_dir()?);
            _app.manage(app::AppState::new(paths));
            #[cfg(desktop)]
            {
                if let Err(e) = _app.handle().plugin(tauri_plugin_updater::Builder::new().build()) {
                    eprintln!("[Updater] Failed to initialize updater plugin: {e}");
                }
                if let Err(e) = tray::setup_tray(_app.handle()) {
                    eprintln!("[Tray] Failed to setup tray: {e}");
                }
            }
            // Games still running from last time show as running again
            let handle = _app.handle().clone();
            tauri::async_runtime::spawn(async move { ipc::adopt_running_games(&handle).await });
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window quits, but a running server's console lives in Ingot:
            // ask first instead of leaving it running out of reach
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" && !ipc::is_quitting() {
                    api.prevent_close();
                    ipc::request_quit(tauri::Manager::app_handle(window));
                }
            }
        })
        .invoke_handler(router.into_handler())
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, _event| {
            // Any other way out (the tray, the launcher closing itself) goes through the same check
            #[cfg(desktop)]
            if let tauri::RunEvent::ExitRequested { api, code: Some(_), .. } = _event {
                if !ipc::is_quitting() {
                    api.prevent_exit();
                    ipc::request_quit(_app);
                }
            }
        });
}


