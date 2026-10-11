pub mod cache;
pub mod codec;
pub mod oscquery;
pub mod udp;

use crate::state::AppState;
use crate::types::OscCommand;

/// Sends one message to VRChat, typed by the worn avatar's parameters when it's one of them.
pub fn send(state: &AppState, msg: &OscCommand) -> Result<(), String> {
    let socket = state.sender_socket.lock().unwrap().clone();
    let Some(socket) = socket else {
        return Err("OSC sender not ready yet".into());
    };

    let avatar_details = state.avatar_details.lock().unwrap().clone();
    let packet = rosc::OscPacket::Message(codec::encode_command(msg, avatar_details.as_ref()));
    let bytes = rosc::encoder::encode(&packet).map_err(|e| e.to_string())?;

    // OSCQuery discovery (osc::oscquery) may know VRChat's real address, which can differ from
    // the fixed send port 9000 (multiple VRChat instances, custom --osc launch args); fall back
    // to the socket's pre-connected fixed target when discovery hasn't found anything (yet, or
    // ever, on a network where mDNS doesn't work).
    let target = *state.oscquery_target.lock().unwrap();

    // send()/send_to() on a UDP socket is a plain non-blocking syscall in practice; spawn it so
    // callers never have to be async just to satisfy the socket API.
    tauri::async_runtime::spawn(async move {
        let result = match target {
            Some(addr) => socket.send_to(&bytes, addr).await.map(|_| ()),
            None => socket.send(&bytes).await.map(|_| ()),
        };
        if let Err(err) = result {
            tracing::warn!("Failed to send OSC message: {err}");
        }
    });

    Ok(())
}
