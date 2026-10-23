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
