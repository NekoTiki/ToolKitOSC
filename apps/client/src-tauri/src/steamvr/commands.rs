//! The SteamVR auto-launch commands the webview calls (see `super`).

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use crate::app::Availability;

fn steamvr_manifest_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("app.vrmanifest"))
}

#[tauri::command]
pub fn steamvr_available() -> Availability {
    Availability::new(super::is_supported(), super::find_openvr_api().is_some())
}

#[tauri::command]
pub fn steamvr_set_auto_launch(app: AppHandle, enable: bool) -> Result<(), String> {
    super::set_auto_launch(&steamvr_manifest_path(&app)?, enable)
}

#[tauri::command]
pub fn steamvr_get_auto_launch(app: AppHandle) -> Result<bool, String> {
    super::get_auto_launch(&steamvr_manifest_path(&app)?)
}
