//! OSC -> VRChat UDP *sender* only. Mirrors the outbound half of the original
//! `src/main/vrc-osc.ts`. There is no receiver in this module - all incoming OSC arrives via
//! `osc::oscquery`, both the ordinary OSCQuery push subscription (`/avatar/parameters/*`) and its
//! polling of `/avatar/change` (VRChat never pushes that one over OSCQuery - see that module's doc
//! comment for how this was confirmed and why polling replaces a fixed-port receiver here rather
//! than the two coexisting). `flatten_packet`/`handle_message` stay here as their canonical home
//! but are `pub(super)` purely so `osc::oscquery` can call them.
//!
//! ## History: a fixed-port `9001` receiver used to live here, twice
//! It was removed once, then reinstated after testing showed OSCQuery never delivers
//! `/avatar/change` at all (only parameters) - reinstating it was the fastest fix at the time.
//! It was removed again once `osc::oscquery` gained polling as a real replacement for that specific
//! gap (rather than "OSCQuery replaces it entirely", which was the first, wrong assumption). If
//! avatar detection or parameter delivery ever regresses again, check `osc::oscquery`'s polling
//! logic and its `AVATAR_POLL_INTERVAL` first before reintroducing a fixed-port receiver - that
//! path has now caused two separate incidents from being removed without a real replacement for
//! what it was quietly covering.
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};
use tokio::net::UdpSocket;

use super::{cache, codec};
use crate::{avatar, state::AppState, types::OscMessage};

const VRCHAT_IP: &str = "127.0.0.1";
const VRCHAT_IN_PORT: u16 = 9000; // we send here

pub async fn run(app: AppHandle) -> anyhow::Result<()> {
    let sender = UdpSocket::bind("0.0.0.0:0").await?;
    sender.connect((VRCHAT_IP, VRCHAT_IN_PORT)).await?;
    let sender = Arc::new(sender);

    let state = app.state::<AppState>();
    *state.sender_socket.lock().unwrap() = Some(sender);

    tracing::info!("OSC sender ready — VRChat at {VRCHAT_IP}:{VRCHAT_IN_PORT}");

    Ok(())
}

/// VRChat can bundle multiple messages in one packet; flatten bundles (recursively) into a
/// simple list, same effect as handling each message individually. `pub(super)`: called by
/// `osc::oscquery`'s receive handler, which gets raw `OscPacket`s from the same decoder - this
/// module has no receive loop of its own (see module doc comment).
pub(super) fn flatten_packet(packet: rosc::OscPacket) -> Vec<rosc::OscMessage> {
    match packet {
        rosc::OscPacket::Message(msg) => vec![msg],
        rosc::OscPacket::Bundle(bundle) => {
            bundle.content.into_iter().flat_map(flatten_packet).collect()
        }
    }
}

/// `pub(super)`: the actual receive path lives in `osc::oscquery`, which calls this per decoded
/// message - see this file's module doc comment for why there's no receive loop here anymore.
pub(super) fn handle_message(app: &AppHandle, raw: rosc::OscMessage) {
    let osc_msg = codec::decode_message(&raw);
    let state = app.state::<AppState>();

    let is_duplicate = {
        let mut messages = state.messages.lock().unwrap();
        let is_duplicate = messages.get(&osc_msg.address) == Some(&osc_msg);
        messages.insert(osc_msg.address.clone(), osc_msg.clone());
        is_duplicate
    };

    // Load-bearing, not just an optimization: osc::oscquery's avatar-change polling calls this
    // with the current value every ~2s regardless of whether it changed (see that module's
    // `poll_avatar_changes`), relying on this to no-op an unchanged value rather than re-emitting/
    // re-triggering avatar-change handling on every single poll tick.
    if is_duplicate {
        return;
    }

    if osc_msg.address == "/avatar/change" {
        if let Some(crate::types::OscArg::Str(avatar_id)) = osc_msg.args.first() {
            tracing::info!("Avatar changed: {avatar_id}");
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

    // The swap above just discarded whatever handle_message had cached for "/avatar/change"
    // itself (loaded_cache is disk-persisted parameter values, which never includes it).
    // Reinsert it so the next call to handle_message for this same address - notably
    // osc::oscquery::poll_avatar_changes, which calls in on an interval regardless of whether
    // the avatar actually changed - has something to dedup against, instead of finding nothing
    // and re-running this entire avatar-change flow on every single poll tick.
    state.messages.lock().unwrap().insert(
        "/avatar/change".to_string(),
        OscMessage {
            address: "/avatar/change".to_string(),
            args: vec![crate::types::OscArg::Str(avatar_id.to_string())]
        }
    );
}
