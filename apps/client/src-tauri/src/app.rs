//! Commands about the app itself rather than anything it talks to: opening links, the tray, and
//! whether an optional integration can be offered.

use std::sync::atomic::Ordering;

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::state::AppState;

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

/// Whether an optional integration (SteamVR, VRCX) can be offered, so Settings can tell "can't
/// work on this OS" (hide the toggle) apart from "not installed" (show it disabled, saying why).
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Availability {
    Available,
    NotInstalled,
    Unsupported,
}

impl Availability {
    pub fn new(supported: bool, installed: bool) -> Self {
        match (supported, installed) {
            (false, _) => Self::Unsupported,
            (true, false) => Self::NotInstalled,
            (true, true) => Self::Available,
        }
    }
}

/// Keeps the window's native close handler and the tray icon (see `lib.rs`) in sync with the
/// frontend's localStorage-backed "keep running in the tray" setting - called once at startup and
/// on every change, since the setting itself lives in the webview, out of reach of either.
#[tauri::command]
pub fn set_minimize_to_tray(state: State<AppState>, enabled: bool) {
    state.minimize_to_tray.store(enabled, Ordering::Relaxed);

    if let Some(tray) = state.tray_icon.lock().unwrap().as_ref() {
        let _ = tray.set_visible(enabled);
    }
}
