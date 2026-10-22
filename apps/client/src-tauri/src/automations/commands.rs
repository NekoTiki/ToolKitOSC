//! The automation commands the webview calls (see `super`).

use tauri::AppHandle;

use super::model::Automation;

#[tauri::command]
pub fn automations_list(app: AppHandle) -> Vec<Automation> {
    super::list(&app)
}

/// Creates or replaces an automation, or fails with a message to show.
#[tauri::command]
pub fn automation_save(app: AppHandle, automation: Automation) -> Result<(), String> {
    // Saved by you, so no longer an assistant's draft.
    super::save(&app, Automation { created_by: None, ..automation })
}

#[tauri::command]
pub fn automation_delete(app: AppHandle, id: String) {
    super::delete(&app, &id);
}

#[tauri::command]
pub fn automation_set_enabled(app: AppHandle, id: String, enabled: bool) {
    super::set_enabled(&app, &id, enabled);
}

/// Runs the automation's action once, without its trigger.
#[tauri::command]
pub fn automation_test(app: AppHandle, id: String) -> Result<(), String> {
    super::test(&app, &id)
}

/// Mirrors Stop everything's paused state.
#[tauri::command]
pub fn automations_set_paused(app: AppHandle, paused: bool) {
    super::set_paused(&app, paused);
}

/// Something happened in the app that automations can start from (a viewer used a control...).
#[tauri::command]
pub fn automation_event(app: AppHandle, event: super::model::AppEvent) {
    super::event(&app, &event);
}

/// The active profile and how many viewers are on, for "only if".
#[tauri::command]
pub fn automations_set_context(app: AppHandle, context: super::model::Context) {
    super::set_context(&app, context);
}
