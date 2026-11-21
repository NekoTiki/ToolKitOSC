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

/// Adds an Intiface server by address and connects to it. Returns its id.
#[tauri::command]
pub fn intiface_add_server(app: AppHandle, label: String, host: String, port: u16) -> Result<String, String> {
    super::add_server(&app, label, host, port)
}

/// Renames a server, or moves it to a new address.
#[tauri::command]
pub fn intiface_update_server(app: AppHandle, id: String, label: String, host: String, port: u16) -> Result<(), String> {
    super::update_server(&app, &id, label, host, port)
}

#[tauri::command]
pub fn intiface_remove_server(app: AppHandle, id: String) -> Result<(), String> {
    super::remove_server(&app, &id)
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
