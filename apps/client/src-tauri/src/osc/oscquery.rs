//! OSCQuery discovery - the **sole receive path** for incoming OSC (see `osc::udp`'s module doc
//! comment for why its fixed-port `9001` receiver was removed). This module covers two distinct
//! needs, by two different mechanisms:
//! - Parameter updates (`/avatar/parameters/*`) arrive as ordinary OSCQuery *pushes*, same as any
//!   other OSC receiver registration - see `register()` below.
//! - Avatar changes (`/avatar/change`) are never pushed by VRChat to an OSCQuery-registered
//!   endpoint, confirmed live (toggling OSC off/on repeatedly against a real VRChat instance
//!   produced zero `/avatar/change` pushes, while `/avatar/parameters/*` kept flowing normally).
//!   VRChat does still expose the current avatar ID as a plain readable value in its OSCQuery tree
//!   (confirmed live: `curl http://127.0.0.1:<oscquery-port>/avatar/change` returns
//!   `{"ACCESS":3,"TYPE":"s","VALUE":["avtr_..."]}`), so `poll_avatar_changes` below actively polls
//!   that value on an interval instead of waiting for a push that will never come. Polling (not a
//!   one-shot fetch on discovery) is what covers switching avatars mid-session, not just the
//!   initial "what avatar is this" question on connect.
//!
//! On top of both of those, mDNS/OSCQuery silently doesn't work over VPNs or non-multicast
//! networks - there is no fixed-port fallback for that case; see `osc::udp`'s doc comment for that
//! trade-off. This module also discovers VRChat's real OSC address instead of assuming the fixed
//! send port 9000, which matters for multiple VRChat instances or a VRChat launched with custom
//! `--osc` args (`osc::udp` still owns that fixed-port *sender*, which this supplements rather
//! than replaces via `commands::send_osc_message`'s automatic fallback).
//!
//! `VRChatOSC::new(None)` lets the crate auto-pick a non-loopback interface for its own mDNS
//! multicast join, rather than forcing loopback: VRChat's OSCQuery HTTP tree is loopback-only, but
//! its OSC transport (and the mDNS traffic advertising it) is bound to `0.0.0.0` and, on a machine
//! with a VPN/virtual adapter (e.g. Tailscale), actually observed reachable via that real
//! interface rather than `127.0.0.1` - binding our own multicast join to loopback instead means we
//! never see VRChat's announcements on the interface they're actually sent on. Confirmed by
//! testing against a live VRChat instance: `Some(LOCALHOST)` here silently discovered nothing.
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use vrchat_osc::{models::OscRootNode, models::OscValue, ServiceType, VRChatOSC};

use super::udp::{flatten_packet, handle_message};
use crate::state::AppState;

const SERVICE_NAME: &str = "vrc-osc-toolkit";
const VRCHAT_SERVICE_PREFIX: &str = "VRChat-Client-";

/// A few seconds of lag on detecting an avatar swap is an acceptable trade for a plain HTTP GET
/// on loopback instead of a full push subscription VRChat doesn't support here anyway.
const AVATAR_POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Runs until aborted (see `AppState::avatar_poll_task`). Deliberately has no special-cased retry
/// logic for VRChat's avatar still loading, a transient GET failure, etc. - the next tick a few
/// seconds later covers all of those, and `handle_message`'s own dedup means re-asserting an
/// unchanged value here is a no-op rather than something this loop needs to track separately.
async fn poll_avatar_changes(app: AppHandle, vrchat_osc: Arc<VRChatOSC>, addr: SocketAddr) {
    loop {
        if let Ok(node) = vrchat_osc.get_parameter_from_addr("/avatar/change", addr).await {
            if let Some(OscValue::String(avatar_id)) = node.value.and_then(|v| v.into_iter().next()) {
                // Feeds the exact same path as a real push (dedup, caching, emit) - see
                // osc::udp::handle_message.
                handle_message(
                    &app,
                    rosc::OscMessage {
                        addr: "/avatar/change".to_string(),
                        args: vec![rosc::OscType::String(avatar_id)]
                    }
                );
            }
        }

        tokio::time::sleep(AVATAR_POLL_INTERVAL).await;
    }
}

pub async fn run(app: AppHandle) -> anyhow::Result<()> {
    let vrchat_osc = VRChatOSC::new(None).await?;

    let discovery_app = app.clone();
    let discovery_vrchat_osc = vrchat_osc.clone();
    vrchat_osc
        .on_connect(move |service| match service {
            ServiceType::Osc(name, addr) if name.starts_with(VRCHAT_SERVICE_PREFIX) => {
                tracing::info!("Discovered VRChat OSC target via OSCQuery: {name} at {addr}");
                let state = discovery_app.state::<AppState>();
                *state.oscquery_target.lock().unwrap() = Some(addr);
            }
            ServiceType::OscQuery(name, addr) if name.starts_with(VRCHAT_SERVICE_PREFIX) => {
                tracing::info!("Discovered VRChat OSCQuery service: {name} at {addr}");
                let app = discovery_app.clone();
                let vrchat_osc = discovery_vrchat_osc.clone();
                let task = tokio::spawn(async move { poll_avatar_changes(app, vrchat_osc, addr).await });

                // Replace, not just set: a rediscovery (VRChat restarted its OSCQuery service at a
                // new address) must stop the old poll rather than leaving it running forever
                // against a dead address alongside the new one.
                let state = discovery_app.state::<AppState>();
                let previous = state.avatar_poll_task.lock().unwrap().replace(task);
                if let Some(old) = previous {
                    old.abort();
                }
            }
            _ => {}
        })
        .await;

    let handler_app = app.clone();
    let root_node = OscRootNode::new().with_avatar();
    vrchat_osc
        .register(SERVICE_NAME, root_node, move |packet| {
            for message in flatten_packet(packet) {
                handle_message(&handler_app, message);
            }
        })
        .await?;

    tracing::info!("OSCQuery service registered as {SERVICE_NAME}");

    // Kept alive for the app's lifetime - dropping it would tear down the mDNS advertisement and
    // OSCQuery HTTP server it owns.
    *app.state::<AppState>().oscquery.lock().unwrap() = Some(vrchat_osc);

    Ok(())
}
