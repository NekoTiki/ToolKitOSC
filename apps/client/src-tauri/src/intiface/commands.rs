//! The Intiface commands the webview calls (see `super`).

use tauri::AppHandle;

#[tauri::command]
pub fn intiface_state(app: AppHandle) -> super::View {
    super::view(&app)
}

/// Once, from the settings the webview used to keep in localStorage.
#[tauri::command]
pub fn intiface_migrate(app: AppHandle, url: Option<String>, enabled: Option<bool>, auto_launch: Option<bool>) {
    super::migrate(&app, url, enabled, auto_launch);
}

/// Empty or missing goes back to the default URL.
#[tauri::command]
pub fn intiface_set_url(app: AppHandle, url: Option<String>) {
    super::set_url(&app, url);
}

#[tauri::command]
pub fn intiface_set_auto_launch(app: AppHandle, auto_launch: bool) {
    super::set_auto_launch(&app, auto_launch);
}

/// Opens the local Intiface Central install.
#[tauri::command]
pub fn intiface_launch() -> Result<(), String> {
    super::launch_now()
}
