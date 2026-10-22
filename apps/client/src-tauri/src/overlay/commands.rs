//! The VR overlay commands the webview calls (see `super`).

use tauri::AppHandle;

use super::model::OverlaySettings;
use crate::wrist::protocol::Profile;

#[tauri::command]
pub fn overlay_settings(app: AppHandle) -> Result<OverlaySettings, String> {
    crate::addons::require(&app, "overlay")?;
    Ok(super::store::get(&app))
}

/// Replaces every overlay setting.
#[tauri::command]
pub fn overlay_save(app: AppHandle, settings: OverlaySettings) -> Result<(), String> {
    crate::addons::require(&app, "overlay")?;
    super::store::with(&app, |current| {
        let changed = *current != settings;
        *current = settings;
        ((), changed)
    });
    super::settings_changed(&app);
    Ok(())
}

/// Sends a toast right away, to check it shows up in the headset.
#[tauri::command]
pub async fn overlay_test_toast(app: AppHandle) -> Result<(), String> {
    crate::addons::require(&app, "overlay")?;
    super::notify::send(&app, "ToolKitOSC", "Viewer actions will show up here").await
}

/// The profiles and the active one, and the viewer groups' names, for the panel.
#[tauri::command]
pub fn overlay_set_context(app: AppHandle, profiles: Vec<Profile>, profile_id: Option<String>, viewer_groups: Vec<Profile>) {
    super::set_context(&app, profiles, profile_id, viewer_groups);
}

/// The controls pinned to your wrist on the avatar you're wearing.
#[tauri::command]
pub fn overlay_pinned(app: AppHandle) -> Vec<String> {
    super::tiles::pinned_ids(&app)
}

/// Pins a control of the avatar you're wearing to your wrist, or unpins it.
#[tauri::command]
pub fn overlay_set_pinned(app: AppHandle, control_id: String, pinned: bool) -> Result<(), String> {
    crate::addons::require(&app, "overlay")?;
    super::tiles::set_pinned(&app, &control_id, pinned)?;
    super::settings_changed(&app);
    Ok(())
}
