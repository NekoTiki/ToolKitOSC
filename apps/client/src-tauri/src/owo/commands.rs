//! The OWO commands the webview calls (see `super`).

use tauri::AppHandle;

use super::model::OwoSettings;

#[tauri::command]
pub fn owo_settings(app: AppHandle) -> Result<OwoSettings, String> {
    crate::addons::require(&app, "owo")?;
    Ok(super::store::get(&app))
}

/// Replaces every OWO setting. A new address starts a new link on the next tick.
#[tauri::command]
pub fn owo_save(app: AppHandle, settings: OwoSettings) -> Result<(), String> {
    crate::addons::require(&app, "owo")?;
    super::store::with(&app, |current| {
        let changed = *current != settings;
        *current = settings;
        ((), changed)
    });
    Ok(())
}

/// The connection and what plays now, without waiting for the next `owo-state`.
#[tauri::command]
pub fn owo_state(app: AppHandle) -> Result<super::View, String> {
    crate::addons::require(&app, "owo")?;
    Ok(super::view(&app))
}

/// Development builds only: talk to the OWO tester.
#[tauri::command]
pub fn owo_simulate(app: AppHandle, simulated: bool) -> Result<(), String> {
    crate::addons::require(&app, "owo")?;
    super::set_simulated(&app, simulated)
}

/// Walks every muscle, one at a time.
#[tauri::command]
pub fn owo_test_muscles(app: AppHandle) -> Result<(), String> {
    crate::addons::require(&app, "owo")?;
    super::test_muscles(&app)
}

/// Development builds only: the packets the OWO tester got, oldest first.
#[tauri::command]
pub fn owo_tester_log(app: AppHandle) -> Result<serde_json::Value, String> {
    crate::addons::require(&app, "owo")?;
    #[cfg(debug_assertions)]
    return serde_json::to_value(super::tester_log()).map_err(|err| err.to_string());
    #[cfg(not(debug_assertions))]
    Err("The OWO tester is only in development builds".into())
}
