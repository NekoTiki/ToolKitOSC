//! The VRCX auto-launch commands the webview calls (see `super`).

use crate::app::Availability;

#[tauri::command]
pub fn vrcx_available() -> Availability {
    Availability::new(super::is_supported(), super::is_vrcx_installed())
}

#[tauri::command]
pub fn vrcx_set_auto_launch(enable: bool) -> Result<(), String> {
    super::set_auto_launch(enable)
}

#[tauri::command]
pub fn vrcx_get_auto_launch() -> bool {
    super::get_auto_launch()
}
