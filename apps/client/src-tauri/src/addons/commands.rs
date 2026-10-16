//! The add-on commands the webview calls (see `super`). `addons_set_enabled` is the only way to
//! turn an add-on on or off.

use tauri::AppHandle;

#[tauri::command]
pub fn addons_list(app: AppHandle) -> super::AddonsView {
    super::view(&app)
}

#[tauri::command]
pub fn addons_set_enabled(app: AppHandle, id: String, enabled: bool) -> Result<(), String> {
    super::set_enabled(&app, &id, enabled)
}

/// "What do you use?" was answered or skipped; `dismiss_kept` also hides the notice about what the
/// update kept on.
#[tauri::command]
pub fn addons_finish_setup(app: AppHandle, dismiss_kept: bool) {
    super::finish_setup(&app, dismiss_kept);
}

/// Stop everything for every add-on that's on. Returns the ones that couldn't be reached.
#[tauri::command]
pub async fn addons_stop_everything(app: AppHandle) -> Vec<super::StopFailure> {
    super::stop_everything(&app).await
}
