//! The bHaptics commands the webview calls (see `super`).

use tauri::AppHandle;

use super::model::{BhapticsSettings, Position};

#[tauri::command]
pub fn bhaptics_settings(app: AppHandle) -> Result<BhapticsSettings, String> {
    crate::addons::require(&app, "bhaptics")?;
    Ok(super::store::get(&app))
}

/// Replaces every bHaptics setting.
#[tauri::command]
pub fn bhaptics_save(app: AppHandle, settings: BhapticsSettings) -> Result<(), String> {
    crate::addons::require(&app, "bhaptics")?;
    super::store::with(&app, |current| {
        let changed = *current != settings;
        *current = settings;
        ((), changed)
    });
    Ok(())
}

/// The connection and what plays now, without waiting for the next `bhaptics-state`.
#[tauri::command]
pub fn bhaptics_state(app: AppHandle) -> Result<super::View, String> {
    crate::addons::require(&app, "bhaptics")?;
    Ok(super::view(&app))
}

#[tauri::command]
pub fn bhaptics_launch_player(app: AppHandle) -> Result<(), String> {
    crate::addons::require(&app, "bhaptics")?;
    super::launch_player()
}

/// Development builds only: talk to the simulated Player.
#[tauri::command]
pub fn bhaptics_simulate(app: AppHandle, simulated: bool) -> Result<(), String> {
    crate::addons::require(&app, "bhaptics")?;
    super::set_simulated(&app, simulated)
}

/// Walks every motor of one position, one at a time.
#[tauri::command]
pub fn bhaptics_test_motors(app: AppHandle, position: Position) -> Result<(), String> {
    crate::addons::require(&app, "bhaptics")?;
    super::test_motors(&app, position)
}

/// Player's answers, the settings, the avatar check and the last 30 s of frames, as text.
#[tauri::command]
pub fn bhaptics_diagnostics(app: AppHandle) -> Result<String, String> {
    crate::addons::require(&app, "bhaptics")?;
    Ok(super::diagnostics(&app))
}
