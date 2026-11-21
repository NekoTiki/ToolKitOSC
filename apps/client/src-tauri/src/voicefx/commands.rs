//! The Voice effects commands the webview calls (see `super`).

use tauri::AppHandle;

use super::holds::Source;
use super::model::Settings;

#[tauri::command]
pub fn voicefx_settings(app: AppHandle) -> Result<Settings, String> {
    crate::addons::require(&app, "voicefx")?;
    Ok(super::store::get(&app))
}

/// Replaces every Voice effects setting.
#[tauri::command]
pub fn voicefx_save(app: AppHandle, settings: Settings) -> Result<(), String> {
    crate::addons::require(&app, "voicefx")?;
    super::store::with(&app, |current| {
        // What VRChat last said stays the app's to keep.
        let settings = Settings { last_vr: current.last_vr, ..settings };
        let changed = *current != settings;
        *current = settings;
        ((), changed)
    });
    super::voicefx(&app).wake.notify_one();
    Ok(())
}

/// Voice effects right now, without waiting for the next `voicefx-state`.
#[tauri::command]
pub fn voicefx_state(app: AppHandle) -> Result<super::View, String> {
    crate::addons::require(&app, "voicefx")?;
    Ok(super::view(&app))
}

/// The microphones and outputs to pick from.
#[tauri::command]
pub async fn voicefx_devices(app: AppHandle) -> Result<super::audio::Devices, String> {
    crate::addons::require(&app, "voicefx")?;
    // Listing devices can take a moment.
    tauri::async_runtime::spawn_blocking(super::audio::devices).await.map_err(|err| err.to_string())
}

/// Turn on myself, and Let go.
#[tauri::command]
pub fn voicefx_hold(app: AppHandle, on: bool) -> Result<(), String> {
    crate::addons::require(&app, "voicefx")?;
    super::hold_mine(&app, on)
}

/// Ends one hold from the page.
#[tauri::command]
pub fn voicefx_release(app: AppHandle, source: Source) -> Result<(), String> {
    crate::addons::require(&app, "voicefx")?;
    super::release(&app, &source);
    Ok(())
}
