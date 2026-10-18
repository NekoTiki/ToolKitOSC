mod addons;
mod app;
mod automations;
mod bhaptics;
mod avatar;
mod boards;
mod controls;
mod haptics;
mod leash;
mod intiface;
mod logging;
mod net;
mod openshock;
mod owo;
mod overlay;
mod osc;
mod presets;
mod secrets;
mod server;
mod state;
mod steam;
mod steamvr;
mod toys;
mod types;
mod vrcx;

use std::sync::atomic::Ordering;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, WindowEvent};

use crate::state::AppState;

/// Builds the tray icon and its "Show"/"Quit" menu, built once for the whole app lifetime but
/// hidden until `app::set_minimize_to_tray` shows it - the icon's only purpose is as a way
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
    // Before any HTTPS client is built (OpenShock's is, with the app state).
    let _ = rustls::crypto::ring::default_provider().install_default();

    let builder = tauri::Builder::default();

    // The Stop everything hotkey. Its key is the user's choice, so the UI registers it.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_global_shortcut::Builder::new().build());

    builder
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // A second launch attempt focuses the existing window instead of opening a new one —
            // also guards against two instances fighting over VRChat's OSC UDP port 9001.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            // The app itself, SteamVR and VRCX
            app::open_url,
            steamvr::commands::steamvr_available,
            steamvr::commands::steamvr_set_auto_launch,
            steamvr::commands::steamvr_get_auto_launch,
            vrcx::commands::vrcx_available,
            vrcx::commands::vrcx_set_auto_launch,
            vrcx::commands::vrcx_get_auto_launch,
            app::set_minimize_to_tray,
            app::updater_check_tag,
            // OSC and presets
            osc::commands::send_osc_message,
            osc::commands::ready,
            presets::load_presets,
            presets::save_presets,
            osc::commands::force_pull_parameters,
            // Running a control as you
            controls::control_run,
            // Add-ons
            addons::commands::addons_list,
            addons::commands::addons_set_enabled,
            addons::commands::addons_finish_setup,
            addons::commands::addons_stop_everything,
            // ESP32 boards
            boards::commands::boards_list,
            boards::commands::board_add,
            boards::commands::board_add_by_address,
            boards::commands::board_pair_cancel,
            boards::commands::board_remove,
            boards::commands::board_save_outputs,
            boards::commands::board_test_output,
            boards::commands::boards_stop_all,
            // Automations
            automations::commands::automations_list,
            automations::commands::automation_save,
            automations::commands::automation_delete,
            automations::commands::automation_set_enabled,
            automations::commands::automation_test,
            automations::commands::automations_set_paused,
            automations::commands::automation_event,
            automations::commands::automations_set_context,
            // The share server: link, viewer commands, sign-in
            server::commands::server_connect,
            server::commands::server_disconnect,
            server::commands::server_send,
            server::commands::server_set_forwarded_addresses,
            server::commands::server_request,
            server::commands::server_set_policy,
            server::commands::auth_state,
            server::commands::auth_migrate,
            server::commands::auth_sign_out,
            server::commands::auth_sign_in,
            server::commands::auth_sign_in_cancel,
            // OpenShock
            openshock::commands::openshock_state,
            openshock::commands::openshock_migrate,
            openshock::commands::openshock_set_token,
            openshock::commands::openshock_recheck,
            openshock::commands::openshock_shockers,
            openshock::commands::openshock_fire,
            openshock::commands::openshock_stop_all,
            // Intiface, toys and patterns
            intiface::commands::intiface_state,
            intiface::commands::intiface_migrate,
            intiface::commands::intiface_set_url,
            intiface::commands::intiface_set_auto_launch,
            intiface::commands::intiface_launch,
            toys::commands::toys_set,
            toys::commands::toys_stop_all,
            toys::commands::patterns_play,
            toys::commands::patterns_off,
            toys::commands::patterns_stop,
            toys::commands::patterns_release,
            toys::commands::patterns_stop_all,
            toys::commands::patterns_saved_list,
            toys::commands::patterns_save,
            toys::commands::patterns_delete,
            toys::commands::patterns_migrate,
            // Avatar haptics
            haptics::commands::haptics_settings,
            haptics::commands::haptics_save,
            haptics::commands::haptics_state,
            haptics::commands::toys_touched,
            // Avatar leash
            leash::commands::leash_settings,
            leash::commands::leash_save,
            leash::commands::leash_state,
            leash::commands::leash_test_pull,
            // bHaptics
            bhaptics::commands::bhaptics_settings,
            bhaptics::commands::bhaptics_save,
            bhaptics::commands::bhaptics_state,
            bhaptics::commands::bhaptics_launch_player,
            bhaptics::commands::bhaptics_simulate,
            bhaptics::commands::bhaptics_test_motors,
            bhaptics::commands::bhaptics_diagnostics,
            owo::commands::owo_settings,
            owo::commands::owo_save,
            owo::commands::owo_state,
            owo::commands::owo_simulate,
            owo::commands::owo_test_muscles,
            owo::commands::owo_tester_log,
            // VR overlay
            overlay::commands::overlay_settings,
            overlay::commands::overlay_save,
            overlay::commands::overlay_test_toast
        ])
        .setup(|app| {
            let tray = setup_tray(app.handle())?;
            *app.state::<AppState>().tray_icon.lock().unwrap() = Some(tray);

            if let Some(window) = app.get_webview_window("main") {
                let handle = app.handle().clone();

                // Hides instead of closing while the "keep running in the tray" setting is on
                // (state.minimize_to_tray, kept in sync by app::set_minimize_to_tray) - the
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

            // Fixed-port OSC *sender* only - see osc::udp's module doc comment for why its old
            // fixed-port receiver was removed (all incoming OSC now arrives via OSCQuery below).
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = osc::udp::run(handle).await {
                    tracing::error!("OSC sender setup failed: {err:?}");
                }
            });

            // The sole OSC *receive* path (see osc::udp's doc comment) - errors here are logged,
            // not fatal to startup, but unlike a fixed-port receiver there's no fallback if this
            // never succeeds (e.g. mDNS blocked by a firewall/VPN): incoming OSC simply won't
            // arrive.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = osc::oscquery::run(handle).await {
                    tracing::error!("OSCQuery setup failed, no OSC will be received: {err:?}");
                }
            });

            // The added ESP32 boards are known even while their add-on is off, for automations.
            boards::load(app.handle());
            automations::start(app.handle().clone());
            // Tokens a launch with no password store had to put in a file go to the store first,
            // if there's one now, before anything reads them. Then the add-ons that are on start
            // (deciding which are, the first time, can read OpenShock's token).
            let handle = app.handle().clone();
            tauri::async_runtime::spawn_blocking(move || {
                secrets::init(&handle);
                addons::start(&handle);
                server::auth::start(handle);
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application")
}
