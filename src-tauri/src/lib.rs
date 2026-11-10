pub use ingot_accounts::{account, auth, keyring_store};
mod app;
pub use ingot_artifacts as artifacts;
pub use ingot_core as core;
#[cfg(test)]
mod ipc_contract;
pub use ingot_settings as settings;
/// User agent for all HTTP requests; the version comes from Cargo.toml at build time
pub const USER_AGENT: &str = concat!(
    "Ingot-Launcher/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/0deans/ingot)"
);

pub mod ipc;
pub(crate) use crate::core::process_tree;
pub mod minecraft {
    pub use ingot_content::{content, importer, instance_export};
    pub use ingot_minecraft::*;
}
pub use ingot_content as content;
pub mod running;
pub mod server {
    pub use crate::app::server_config::{
        create_server, get_server_dir, get_servers_dir, load_servers,
    };
    pub(crate) use crate::app::server_config::{delete_server, update_server};
    pub use ingot_server::*;
    pub use ingot_server_map::{live, map};
}
pub use ingot_server_map as server_map;
pub mod system;
#[cfg(desktop)]
pub mod tray;
pub use ingot_version_change as version_change;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "shell32")]
        extern "system" {
            fn SetCurrentProcessExplicitAppUserModelID(app_id: *const u16) -> i32;
        }

        let app_id: Vec<u16> = std::ffi::OsStr::new("ingot")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            let _ = SetCurrentProcessExplicitAppUserModelID(app_id.as_ptr());
        }
    }

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

    #[cfg(desktop)]
    {
        let startup_args: Vec<String> = std::env::args().collect();
        if app::cli::parse_cli_launch_instance(&startup_args).is_some() {
            let delegated = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(
                    app::cli_waiter::try_delegate_to_running_instance(&startup_args),
                )
            });
            if let Some(exit_code) = delegated {
                std::process::exit(exit_code);
            }
        }
    }

    let router = ipc::router::<tauri::Wry>();

    #[cfg(debug_assertions)]
    if let Err(error) = taurpc::Exporter::new().export(&router, "../src/bindings.ts") {
        eprintln!("[IPC] Failed to export TypeScript bindings: {error}");
    }

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            let is_shortcut = app::cli::parse_cli_launch_instance(&args).is_some();
            if !is_shortcut {
                app::lifecycle::clear_direct_launch_session(app);
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
            app::cli::handle_cli_args(app, &args);
        }));
    }

    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .setup(|_app| {
            let paths = app::AppPaths::new(_app.path().app_data_dir()?);
            _app.manage(app::AppState::new(paths)?);
            let startup_args: Vec<String> = std::env::args().collect();
            let is_direct_launch = app::cli::parse_cli_launch_instance(&startup_args).is_some();
            app::lifecycle::set_direct_launch_session(_app.handle(), is_direct_launch);
            #[cfg(desktop)]
            {
                app::cli_waiter::start_server(_app.handle());
                if let Err(e) = _app
                    .handle()
                    .plugin(tauri_plugin_updater::Builder::new().build())
                {
                    eprintln!("[Updater] Failed to initialize updater plugin: {e}");
                }
                if let Err(e) = tray::setup_tray(_app.handle()) {
                    eprintln!("[Tray] Failed to setup tray: {e}");
                }
            }
            // Games still running from last time show as running again
            let handle = _app.handle().clone();
            tauri::async_runtime::spawn(async move {
                app::lifecycle::adopt_running_games(&handle).await;
                app::cli::handle_cli_args(&handle, &startup_args);
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window quits, but a running server's console lives in Ingot:
            // ask first instead of leaving it running out of reach
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" && !app::lifecycle::is_quitting(window.app_handle()) {
                    api.prevent_close();
                    app::lifecycle::request_quit(tauri::Manager::app_handle(window));
                }
            }
        })
        .invoke_handler(router.into_handler())
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, _event| {
            // Any other way out (the tray, the launcher closing itself) goes through the same check
            #[cfg(desktop)]
            match _event {
                tauri::RunEvent::ExitRequested {
                    api, code: Some(_), ..
                } => {
                    if !app::lifecycle::is_quitting(_app) {
                        api.prevent_exit();
                        app::lifecycle::request_quit(_app);
                    }
                }
                tauri::RunEvent::Exit => {
                    app::cli_waiter::cleanup_endpoint_file(_app);
                }
                _ => {}
            }
        });
}
