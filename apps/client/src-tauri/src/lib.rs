mod avatar;
mod commands;
mod logging;
mod osc;
mod state;
mod steamvr;
mod types;
mod vrcx;

use std::sync::atomic::Ordering;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, WindowEvent};

use crate::state::AppState;

/// Builds the tray icon and its "Show"/"Quit" menu, built once for the whole app lifetime but
/// hidden until `commands::set_minimize_to_tray` shows it - the icon's only purpose is as a way
/// back into a window that's hidden instead of closed, so it should only be visible while that
/// setting is actually on.
fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<tauri::tray::TrayIcon> {
    let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)
        .inspect(|tray| {
            let _ = tray.set_visible(false);
        })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // A second launch attempt focuses the existing window instead of opening a new one —
            // also guards against two instances fighting over VRChat's OSC UDP port 9001.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::send_osc_message,
            commands::open_url,
            commands::ready,
            commands::intiface_central_available,
            commands::start_intiface_central,
            commands::steamvr_available,
            commands::steamvr_set_auto_launch,
            commands::steamvr_get_auto_launch,
            commands::vrcx_available,
            commands::vrcx_set_auto_launch,
            commands::vrcx_get_auto_launch,
            commands::set_minimize_to_tray
        ])
        .setup(|app| {
            let tray = setup_tray(app.handle())?;
            *app.state::<AppState>().tray_icon.lock().unwrap() = Some(tray);

            if let Some(window) = app.get_webview_window("main") {
                let handle = app.handle().clone();

                // Hides instead of closing while the "keep running in the tray" setting is on
                // (state.minimize_to_tray, kept in sync by commands::set_minimize_to_tray) - the
                // OSC bridge below keeps running either way, so this is the only thing standing
                // between "closing the window" and "quitting the app".
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        let state = handle.state::<AppState>();
                        if state.minimize_to_tray.load(Ordering::Relaxed) {
                            api.prevent_close();
                            if let Some(window) = handle.get_webview_window("main") {
                                let _ = window.hide();
                            }
                        }
                    }
                });
            }

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = osc::udp::run(handle).await {
                    tracing::error!("OSC bridge stopped with an error: {err:?}");
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application")
}
