//! The leash commands the webview calls (see `super`).

use tauri::AppHandle;

use super::model::LeashSettings;

#[tauri::command]
pub fn leash_settings(app: AppHandle) -> Result<LeashSettings, String> {
    crate::addons::require(&app, "leash")?;
    Ok(super::store::get(&app))
}

/// Replaces every leash setting.
#[tauri::command]
pub fn leash_save(app: AppHandle, settings: LeashSettings) -> Result<(), String> {
    crate::addons::require(&app, "leash")?;
    super::store::with(&app, |current| {
        let changed = *current != settings;
        *current = settings;
        ((), changed)
    });
    Ok(())
}

/// The leash right now, without waiting for the next `leash-state`.
#[tauri::command]
pub fn leash_state(app: AppHandle) -> Result<super::View, String> {
    crate::addons::require(&app, "leash")?;
    Ok(super::view(&app))
}

/// Walks you forward for a second.
#[tauri::command]
pub fn leash_test_pull(app: AppHandle) -> Result<(), String> {
    crate::addons::require(&app, "leash")?;
    super::test_pull(&app)
}
