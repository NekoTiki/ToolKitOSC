//! The haptics commands the webview calls (see `super`).

use tauri::AppHandle;

use super::model::HapticsSettings;

#[tauri::command]
pub fn haptics_settings(app: AppHandle) -> Result<HapticsSettings, String> {
    crate::addons::require(&app, "haptics")?;
    Ok(super::store::get(&app))
}

/// Replaces every haptics setting.
#[tauri::command]
pub fn haptics_save(app: AppHandle, settings: HapticsSettings) -> Result<(), String> {
    crate::addons::require(&app, "haptics")?;
    super::store::with(&app, |current| {
        let changed = *current != settings;
        *current = settings;
        ((), changed)
    });
    Ok(())
}

/// The actuators touch drives right now, without waiting for the next `toys-state`. Empty while
/// haptics are off.
#[tauri::command]
pub fn toys_touched(app: AppHandle) -> Vec<crate::toys::Touched> {
    super::touched(&app)
}

/// The zones and levels right now, without waiting for the next `haptics-state`.
#[tauri::command]
pub fn haptics_state(app: AppHandle) -> Result<super::View, String> {
    crate::addons::require(&app, "haptics")?;
    Ok(super::view(&app))
}
