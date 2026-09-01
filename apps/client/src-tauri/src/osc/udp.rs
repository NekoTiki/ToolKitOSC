//! OSC <-> VRChat UDP bridge. Mirrors `oscReceiver.on('message', ...)` from the original
//! src/main/vrc-osc.ts, including its exact event ordering on `/avatar/change`.
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};
use tokio::net::UdpSocket;

use super::{cache, codec};
use crate::{avatar, state::AppState, types::OscMessage};

const VRCHAT_IP: &str = "127.0.0.1";
const VRCHAT_IN_PORT: u16 = 9000; // we send here
const VRCHAT_OUT_PORT: u16 = 9001; // we listen here

pub async fn run(app: AppHandle) -> anyhow::Result<()> {
    let sender = UdpSocket::bind("0.0.0.0:0").await?;
    sender.connect((VRCHAT_IP, VRCHAT_IN_PORT)).await?;
    let sender = Arc::new(sender);

    {
        let state = app.state::<AppState>();
        *state.sender_socket.lock().unwrap() = Some(sender.clone());
    }

    println!("OSC sender ready — VRChat at {VRCHAT_IP}:{VRCHAT_IN_PORT}");

    let receiver = UdpSocket::bind((VRCHAT_IP, VRCHAT_OUT_PORT)).await?;
    println!("OSC receiver listening on {VRCHAT_IP}:{VRCHAT_OUT_PORT}");

    let mut buf = [0u8; 8192];

    loop {
        let (len, _addr) = receiver.recv_from(&mut buf).await?;

        let packet = match rosc::decoder::decode_udp(&buf[..len]) {
            Ok((_, packet)) => packet,
            Err(err) => {
                eprintln!("Failed to decode incoming OSC packet: {err:?}");
                continue;
            }
        };

        for message in flatten_packet(packet) {
            handle_message(&app, message);
        }
    }
}

/// VRChat can bundle multiple messages in one packet; flatten bundles (recursively) into a
/// simple list, same effect as handling each message individually.
fn flatten_packet(packet: rosc::OscPacket) -> Vec<rosc::OscMessage> {
    match packet {
        rosc::OscPacket::Message(msg) => vec![msg],
        rosc::OscPacket::Bundle(bundle) => {
            bundle.content.into_iter().flat_map(flatten_packet).collect()
        }
    }
}

fn handle_message(app: &AppHandle, raw: rosc::OscMessage) {
    let osc_msg = codec::decode_message(&raw);
    let state = app.state::<AppState>();

    state
        .messages
        .lock()
        .unwrap()
        .insert(osc_msg.address.clone(), osc_msg.clone());

    if osc_msg.address == "/avatar/change" {
        if let Some(crate::types::OscArg::Str(avatar_id)) = osc_msg.args.first() {
            println!("Avatar changed: {avatar_id}");
            on_avatar_change(app, avatar_id);
        }
    }

    let _ = app.emit("vrc-osc-message", &osc_msg);

    let avatar_id = state.avatar_details.lock().unwrap().as_ref().map(|d| d.id.clone());
    if let Some(avatar_id) = avatar_id {
        let cache_snapshot = state.messages.lock().unwrap().clone();
        cache::save_throttled(app.clone(), &state.cache_throttle, avatar_id, cache_snapshot);
    }
}

/// Order matters and mirrors the original exactly: resolve avatar metadata, emit avatar details,
/// swap the in-memory cache to that avatar's saved cache file, then emit the bulk replay — all
/// *before* the individual `/avatar/change` message itself gets emitted by the caller.
fn on_avatar_change(app: &AppHandle, avatar_id: &str) {
    let Some(details) = avatar::get_avatar_details(avatar_id) else {
        return;
    };

    let state = app.state::<AppState>();

    *state.avatar_details.lock().unwrap() = Some(details.clone());
    let _ = app.emit("vrc-avatar-details", &details);

    let loaded_cache = cache::load(app, &details.id);
    *state.messages.lock().unwrap() = loaded_cache.clone();

    let bulk: Vec<OscMessage> = loaded_cache.into_values().collect();
    let _ = app.emit("vrc-osc-message-bulk", &bulk);
}
