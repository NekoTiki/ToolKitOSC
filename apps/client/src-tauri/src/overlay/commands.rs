//! The VR overlay commands the webview calls (see `super`).

use tauri::AppHandle;

use super::model::OverlaySettings;

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
    // Where toasts go may have changed.
    crate::addons::changed(&app);
    Ok(())
}

/// Sends a toast right away, to check it shows up in the headset.
#[tauri::command]
pub async fn overlay_test_toast(app: AppHandle) -> Result<(), String> {
    crate::addons::require(&app, "overlay")?;
    super::notify::send(&app, "ToolKitOSC", "Viewer actions will show up here").await
}
