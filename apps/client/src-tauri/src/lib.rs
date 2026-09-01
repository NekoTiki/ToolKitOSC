mod avatar;
mod commands;
mod logging;
mod osc;
mod state;
mod types;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // A second launch attempt focuses the existing window instead of opening a new one —
            // also guards against two instances fighting over VRChat's OSC UDP port 9001.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::send_osc_message,
            commands::open_url,
            commands::ready
        ])
        .setup(|app| {
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
