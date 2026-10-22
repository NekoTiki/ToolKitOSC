//! The VR overlay commands the webview calls (see `super`).

use serde::Serialize;
use tauri::AppHandle;

use super::model::{OverlaySettings, Target};
use crate::wrist::protocol::{Profile, Theme};

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

/// What each part of the overlay is doing, for its tab in Settings.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayStatus {
    /// The panel process runs: SteamVR is on and the panel or the HUD is.
    in_vr: bool,
    /// The panel process crashed too often and stopped.
    gave_up: bool,
    /// Where toasts go now, None while off or nothing runs.
    toasts_to: Option<Target>
}

#[tauri::command]
pub fn overlay_status(app: AppHandle) -> OverlayStatus {
    let (in_vr, gave_up) = {
        let panel = super::overlay(&app).panel.lock().unwrap();
        (panel.running(), panel.gave_up)
    };
    OverlayStatus { in_vr, gave_up, toasts_to: super::destination(&app) }
}

/// Sends a toast right away, to check it shows up in the headset.
#[tauri::command]
pub async fn overlay_test_toast(app: AppHandle) -> Result<(), String> {
    crate::addons::require(&app, "overlay")?;
    super::notify::send(&app, "ToolKitOSC", "Viewer actions will show up here").await
}

/// The profiles and the active one, the viewer groups' names and the theme's colors (None when the
/// webview couldn't read them), for the panel.
#[tauri::command]
pub fn overlay_set_context(app: AppHandle, profiles: Vec<Profile>, profile_id: Option<String>, viewer_groups: Vec<Profile>, theme: Option<Theme>) {
    super::set_context(&app, profiles, profile_id, viewer_groups, theme.unwrap_or_default());
}

/// The controls pinned to your wrist on the avatar you're wearing.
#[tauri::command]
pub fn overlay_pinned(app: AppHandle) -> Vec<String> {
    super::panel::pinned::pinned_ids(&app)
}

/// Pins a control of the avatar you're wearing to your wrist, or unpins it.
#[tauri::command]
pub fn overlay_set_pinned(app: AppHandle, control_id: String, pinned: bool) -> Result<(), String> {
    crate::addons::require(&app, "overlay")?;
    super::panel::pinned::set_pinned(&app, &control_id, pinned)?;
    super::settings_changed(&app);
    Ok(())
}
