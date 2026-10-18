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

/// The profiles and the active one, for the panel.
#[tauri::command]
pub fn overlay_set_context(app: AppHandle, profiles: Vec<Profile>, profile_id: Option<String>) {
    super::set_context(&app, profiles, profile_id);
}
