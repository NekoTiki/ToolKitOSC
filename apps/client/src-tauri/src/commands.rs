//! Tauri commands — the `invoke()`-callable surface the frontend's TS bridge
//! (src/lib/tauri-bridge.ts) wraps, matching the shape of the old Electron `window.api`.
use tauri::{AppHandle, Emitter, State};

use crate::osc::codec;
use crate::state::AppState;
use crate::types::OscCommand;

#[tauri::command]
pub fn send_osc_message(state: State<AppState>, msg: OscCommand) -> Result<(), String> {
    let socket = state.sender_socket.lock().unwrap().clone();
    let Some(socket) = socket else {
        return Err("OSC sender not ready yet".into());
    };

    let packet = rosc::OscPacket::Message(codec::encode_command(&msg));
    let bytes = rosc::encoder::encode(&packet).map_err(|e| e.to_string())?;

    // send() on a connected UDP socket is a plain non-blocking syscall in practice; spawn it so
    // this command handler never has to be async just to satisfy the socket API.
    tauri::async_runtime::spawn(async move {
        if let Err(err) = socket.send(&bytes).await {
            tracing::warn!("Failed to send OSC message: {err}");
        }
    });

    Ok(())
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
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
