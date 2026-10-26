//! The commands the Controls and Profiles pages call (see `super`). Each change comes back as
//! `setup-controls`.
use serde_json::Value;
use tauri::AppHandle;

#[tauri::command]
pub fn setup_controls(app: AppHandle) -> super::Snapshot {
    super::snapshot(&app)
}

/// Saves the worn avatar's groups and their controls, in this order.
#[tauri::command]
pub fn setup_save_groups(app: AppHandle, avatar_id: String, groups: Vec<Value>) -> Result<(), String> {
    super::save_groups(&app, &avatar_id, groups)
}

/// Saves the worn avatar's profiles and which one is its default.
#[tauri::command]
pub fn setup_save_profiles(app: AppHandle, avatar_id: String, profiles: Vec<super::Profile>, default_profile: Option<String>) -> Result<(), String> {
    super::save_profiles(&app, &avatar_id, profiles, default_profile)
}

/// Turns a profile on, or none, until the app closes.
#[tauri::command]
pub fn setup_switch_profile(app: AppHandle, profile_id: Option<String>) -> Result<(), String> {
    super::switch_profile(&app, profile_id)
}

#[tauri::command]
pub fn setup_saved_control_types(app: AppHandle) -> Vec<String> {
    super::saved_control_types(&app)
}

/// Once, from what the webview used to keep in localStorage.
#[tauri::command]
pub fn setup_migrate(app: AppHandle, avatars: Vec<super::Legacy>) {
    super::migrate(&app, avatars);
}

#[tauri::command]
pub fn setup_viewers(app: AppHandle) -> super::viewers::ViewersFile {
    super::viewers::snapshot(&app)
}

/// Saves the viewer groups you made, in this order. Anyone in a group that's gone is taken out.
#[tauri::command]
pub fn setup_save_viewer_groups(app: AppHandle, groups: Vec<super::viewers::ViewerGroup>) -> Result<(), String> {
    super::viewers::save_groups(&app, groups)
}

/// Puts a viewer (by viewer key) in a group, for `durationMs` or until removed.
#[tauri::command]
pub fn setup_add_to_group(app: AppHandle, key: String, group_id: String, duration_ms: Option<u64>) -> Result<(), String> {
    super::viewers::add_to_group(&app, &key, &group_id, duration_ms)
}

#[tauri::command]
pub fn setup_remove_from_group(app: AppHandle, key: String, group_id: String) -> Result<(), String> {
    super::viewers::remove_from_group(&app, &key, &group_id)
}

#[tauri::command]
pub fn setup_ban(app: AppHandle, scope: String, value: String, reason: Option<String>) -> Result<(), String> {
    super::viewers::ban(&app, scope, value, reason)
}

#[tauri::command]
pub fn setup_set_approved(app: AppHandle, discord_id: String, approved: bool) -> Result<(), String> {
    super::viewers::set_approved(&app, &discord_id, approved)
}

#[tauri::command]
pub fn setup_unban(app: AppHandle, scope: String, value: String) -> Result<(), String> {
    super::viewers::unban(&app, &scope, &value)
}

/// Once, from what the webview used to keep in localStorage and its database.
#[tauri::command]
pub fn setup_migrate_viewers(app: AppHandle, viewers: super::viewers::ViewersFile) -> Result<(), String> {
    super::viewers::migrate(&app, viewers)
}

/// Whether Stop everything is on.
#[tauri::command]
pub fn stop_paused(app: AppHandle) -> bool {
    super::stop::is_paused(&app)
}

/// Lets viewers, assistants and automations act again after Stop everything.
#[tauri::command]
pub fn stop_resume(app: AppHandle) {
    super::stop::resume(&app);
}

/// Once, from the paused state the webview used to keep in localStorage.
#[tauri::command]
pub fn stop_migrate(app: AppHandle, paused: bool) {
    super::stop::migrate(&app, paused);
}
