//! The OSC commands the webview calls: sending, pulling values, and the replay on load.

use tauri::{AppHandle, Emitter, State};

use crate::state::AppState;
use crate::types::OscCommand;

#[tauri::command]
pub fn send_osc_message(state: State<AppState>, msg: OscCommand) -> Result<(), String> {
    crate::osc::send(&state, &msg)
}

/// Fetches every current avatar parameter value from VRChat over OSCQuery and feeds each through
/// the normal message pipeline (so the frontend just sees ordinary `vrc-osc-message` events) -
/// see `osc::oscquery::force_pull_parameters` for why this is needed on top of the push
/// subscription. Returns the number of parameters actually pulled.
#[tauri::command]
pub async fn force_pull_parameters(app: AppHandle, missing_only: bool) -> Result<usize, String> {
    super::oscquery::force_pull_parameters(&app, missing_only).await
}

/// Replays current state to a (re)connecting frontend — equivalent to the original's
/// `preload-ready` IPC handler.
#[tauri::command]
pub fn ready(app: AppHandle, state: State<AppState>) {
    if let Some(details) = state.avatar_details.lock().unwrap().clone() {
        let _ = app.emit("vrc-avatar-details", &details);
    }

    let messages: Vec<_> = state.messages.lock().unwrap().values().cloned().collect();
    for msg in messages {
        let _ = app.emit("vrc-osc-message", &msg);
    }
}
