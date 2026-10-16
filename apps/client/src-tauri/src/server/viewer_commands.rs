//! A viewer's command, checked and run here instead of in the webview, so it works the same while
//! the window is hidden, busy or reloading. The checks and their order are the ones
//! `useWebsocketHost.ts` had:
//! 1. Stop everything is on: dropped, not logged.
//! 2. The control is looked up and the command checked against it for this viewer (their groups
//!    open it up). An unknown control is still logged and accepted, without running anything; a
//!    command the checks refuse is dropped.
//! 3. A viewer's own pattern may need them signed in, and a hidden group is never reachable.
//! 4. Ban, then "who can use your controls", then the rate limit, each with its own reply.
//! 5. A live recording only moves the toy. Anything else runs, is accepted (shown to every
//!    viewer as the last action), may start automations, and goes to the webview to be logged and
//!    announced (`command-executed`).
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use super::limits::{self, VIEWER_LIVE_ID, VIEWER_PATTERN_ID};
use super::viewer;

/// The same fixed window as the relay's: 20 commands a second, 50 when signed in. Only clears a
/// dragged slider, which the share page already slows to about 10 a second.
const RATE_WINDOW: Duration = Duration::from_secs(1);
const RATE_MAX: u32 = 20;
const RATE_MAX_SIGNED_IN: u32 = 50;

static RATE: Mutex<Option<HashMap<String, (u32, Instant)>>> = Mutex::new(None);

fn rate_limited(peer_id: &str, max: u32) -> bool {
    let mut guard = RATE.lock().unwrap();
    let buckets = guard.get_or_insert_with(HashMap::new);
    let now = Instant::now();
    let bucket = buckets.entry(peer_id.to_owned()).or_insert((0, now));
    if now.duration_since(bucket.1) >= RATE_WINDOW {
        *bucket = (1, now);
        return false;
    }
    bucket.0 += 1;
    bucket.0 > max
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

/// One `{type: 'command', from, message}` from the server.
pub fn handle(app: &AppHandle, envelope: &Value) {
    if crate::automations::is_paused(app) {
        return;
    }
    let Some(peer_id) = text(envelope, "from") else {
        return;
    };
    let message = envelope.get("message").cloned().unwrap_or(Value::Null);
    let (Some(group_id), Some(control_id)) = (text(&message, "groupId"), text(&message, "controlId")) else {
        return;
    };

    // Nothing is decided before the webview has said what the controls and rules are.
    let Some((policy, client)) = super::policy_and_client(app, peer_id) else {
        tracing::debug!("Viewer command before the controls are known, dropped");
        return;
    };
    let Some(viewer) = viewer::describe(&client) else {
        return;
    };

    let control = policy.control(group_id, control_id).map(|(control, _)| control.clone());
    let groups = policy.memberships.get(&viewer.key).cloned().unwrap_or_default();
    let command = match &control {
        Some(control) => match limits::sanitize_command(&limits::resolve_control_for_viewer(control, &groups), &message) {
            Some(command) => Some(command),
            None => return
        },
        None => None
    };

    let kind = command.as_ref().and_then(|c| text(c, "type"));
    let pattern_id = command.as_ref().and_then(|c| text(c, "value"));
    let viewer_pattern = kind == Some("intiface-pattern") && matches!(pattern_id, Some(VIEWER_PATTERN_ID | VIEWER_LIVE_ID));
    let signed_in_only = control.as_ref().and_then(|c| c.pointer("/viewerPatterns/signedInOnly")).and_then(Value::as_bool).unwrap_or(false);
    if (viewer_pattern && signed_in_only && viewer.discord_id.is_none()) || policy.is_group_hidden(group_id) {
        return;
    }

    if let Some(ban) = policy.banned(&viewer.ip, viewer.discord_id.as_deref()) {
        super::send_message(app, "client-banned", json!({ "peerId": peer_id, "scope": ban.scope, "reason": ban.reason }));
        return;
    }
    if !viewer::passes_client_check(&policy.client_type, &viewer) {
        super::send_message(
            app,
            "client-invalid",
            json!({ "peerId": peer_id, "reason": "Client failed validation", "type": policy.client_type })
        );
        return;
    }
    // Deliberately no delay before this: a dragged slider's position on the share page waits for
    // this round trip.
    let max = if viewer.discord_id.is_some() { RATE_MAX_SIGNED_IN } else { RATE_MAX };
    if rate_limited(peer_id, max) {
        super::send_message(app, "rate-limited", json!({ "peerId": peer_id, "reason": "You are sending commands too quickly - slow down." }));
        return;
    }

    let viewer_name = policy.show_viewers.then(|| viewer.display_name.clone());

    // Live values while a viewer records only move the toy: nothing to log, announce or show.
    if kind == Some("intiface-pattern") && pattern_id == Some(VIEWER_LIVE_ID) {
        if let (Some(control), Some(command)) = (&control, &command) {
            crate::controls::run(app, control, command, viewer_name);
        }
        return;
    }

    let outcome = match (&control, &command) {
        (Some(control), Some(command)) => crate::controls::run(app, control, command, viewer_name),
        _ => Default::default()
    };

    let sent = command.as_ref().unwrap_or(&message);
    let control_name = control
        .as_ref()
        .and_then(|c| text(c, "name"))
        .filter(|name| !name.is_empty())
        .or_else(|| text(&message, "controlName").filter(|name| !name.is_empty()))
        .unwrap_or(control_id);
    let accepted = json!({
        "groupId": group_id,
        "controlId": control_id,
        "controlName": control_name,
        "type": message.get("type"),
        "value": outcome.shock.clone().or_else(|| sent.get("value").cloned())
    });

    // Shown to every viewer as the share page's last action.
    let mut reply = accepted.clone();
    reply["peerId"] = json!(peer_id);
    super::send_message(app, "command-accepted", reply);

    // The webview logs it, notes who used the control last and posts to the chatbox.
    let mut executed = accepted;
    executed["peerId"] = json!(peer_id);
    if viewer_pattern {
        executed["samples"] = command.as_ref().and_then(|c| c.get("samples")).cloned().unwrap_or(Value::Null);
        executed["patternRun"] = json!(outcome.pattern_run);
    }
    let _ = app.emit("command-executed", executed);

    // Automations can start from it. A shock that didn't go out (cooldown) doesn't count.
    if let Some(control) = &control {
        let control_type = text(control, "type").map(str::to_owned);
        if control_type.as_deref() != Some("open-shock-shocker") || outcome.shock.is_some() {
            crate::automations::event(
                app,
                &crate::automations::model::AppEvent::ViewerCommand { control_id: Some(control_id.to_owned()), control_type }
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_counts_per_peer_and_window() {
        for _ in 0..3 {
            assert!(!rate_limited("test-peer-a", 3));
        }
        assert!(rate_limited("test-peer-a", 3));
        assert!(!rate_limited("test-peer-b", 3));
    }
}
