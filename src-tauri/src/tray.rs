#![cfg(desktop)]

use tauri::menu::{IconMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

pub const TRAY_ID: &str = "main-tray";

pub fn setup_tray<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    let icon = if let Some(default_icon) = app.default_window_icon() {
        default_icon.clone()
    } else {
        let bytes = include_bytes!("../icons/32x32.png");
        tauri::image::Image::from_bytes(bytes)?
    };

    let show_item = IconMenuItem::with_id(
        app,
        "show",
        "Open Ingot",
        true,
        Some(icon.clone()),
        Some("CmdOrCtrl+O"),
    )?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit Ingot", true, Some("CmdOrCtrl+Q"))?;
    let menu = Menu::with_items(app, &[&show_item, &sep, &quit_item])?;

    // On macOS, menu bar icons are expected to show the menu on left click (Apple HIG).
    // On Windows and Linux, left click restores the window while right click opens the menu.
    #[cfg(target_os = "macos")]
    let show_menu_on_left = true;
    #[cfg(not(target_os = "macos"))]
    let show_menu_on_left = false;

    let _tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("Ingot")
        .menu(&menu)
        .show_menu_on_left_click(show_menu_on_left)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                restore_main_window(app);
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|_tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                #[cfg(not(target_os = "macos"))]
                restore_main_window(_tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

pub fn restore_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
