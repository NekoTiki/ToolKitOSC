//! The host's link to the share server (`/host`). Rust owns the socket so it stays up while the
//! webview reloads, sits hidden in the tray or is busy. Viewer commands are checked and run here
//! (see `commands`), against a copy of the controls and rules the webview pushes (`policy`).
//! Other incoming messages go to the webview as `server-message` events, and it sends its own
//! through `server_send`. Avatar parameter values go straight from the OSC receive path to the
//! server (see `on_parameter`), for the parameters the webview asked for. The token it signs in
//! with stays here too (see `auth`).
pub mod auth;
mod viewer_commands;
pub mod limits;
mod link;
pub mod policy;
pub mod viewer;
pub mod commands;

use std::collections::HashSet;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;

use crate::state::AppState;
use crate::types::OscMessage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Closed,
    Connecting,
    Open,
    /// The server refused the token (`auth-error`). Not retried.
    Rejected,
    /// The server speaks another protocol version (`protocol-mismatch`). Not retried.
    Mismatch
}

pub enum LinkCommand {
    Send(String),
    Shutdown
}

struct Link {
    /// Tells a stale task (one being shut down) apart from the current one.
    id: u64,
    url: String,
    token: Option<String>,
    protocol_version: u32,
    tx: mpsc::UnboundedSender<LinkCommand>
}

#[derive(Default)]
struct Inner {
    link: Option<Link>,
    next_id: u64,
    status: Option<Status>,
    /// Set by `auth-success`. Until then the server takes nothing but `auth-token`.
    authed: bool,
    /// Avatar parameters whose values the share page shows (see `set_forwarded`).
    forwarded: HashSet<String>,
    /// The last `auth-success` and `client-list`, replayed to a webview that comes back to a link
    /// that's already up (a reload), so it learns what it missed.
    auth_success: Option<String>,
    client_list: Option<String>,
    /// The viewers connected now, from the last `client-list`, with names kept up to date.
    clients: Vec<serde_json::Value>,
    /// The controls and rules viewer commands are checked against (see `policy`). Kept across
    /// reconnects and webview reloads.
    policy: Option<policy::Policy>
}

#[derive(Default)]
pub struct Server {
    inner: Mutex<Inner>,
    pub auth: auth::Auth
}

fn server(app: &AppHandle) -> &Server {
    &app.state::<AppState>().inner().server
}

/// Connects to `url` (the server's `/host` socket), signed in with its token when there is one.
/// Asking again for the link that's already up keeps it and replays its state instead.
pub fn connect(app: &AppHandle, url: String, protocol_version: u32) {
    let token = auth::token(app, &url);
    let replay = {
        let mut inner = server(app).inner.lock().unwrap();
        let same = inner.link.as_ref().is_some_and(|link| link.url == url && link.token == token);

        if same {
            Some((inner.status, inner.auth_success.clone(), inner.client_list.clone()))
        } else {
            if let Some(old) = inner.link.take() {
                let _ = old.tx.send(LinkCommand::Shutdown);
            }
            inner.next_id += 1;
            inner.authed = false;
            inner.auth_success = None;
            inner.client_list = None;
            inner.clients.clear();

            let id = inner.next_id;
            let auth = token.as_deref().map(|token| auth_message(token, protocol_version));
            let tx = link::spawn(app.clone(), id, url.clone(), auth);
            inner.link = Some(Link { id, url, token, protocol_version, tx });
            None
        }
    };

    if let Some((status, auth_success, client_list)) = replay {
        if let Some(status) = status {
            let _ = app.emit("server-status", status);
        }
        if status == Some(Status::Open) {
            for text in [auth_success, client_list].into_iter().flatten() {
                let _ = app.emit("server-message", text);
            }
        }
    }
}

/// The token for the linked server changed (signed in or out, or loaded at startup): connects
/// again with the new one. Nothing happens while it's the same.
pub fn refresh(app: &AppHandle) {
    let target = {
        let inner = server(app).inner.lock().unwrap();
        inner.link.as_ref().map(|link| (link.url.clone(), link.protocol_version))
    };
    if let Some((url, protocol_version)) = target {
        connect(app, url, protocol_version);
    }
}

pub fn disconnect(app: &AppHandle) {
    let link = {
        let mut inner = server(app).inner.lock().unwrap();
        inner.authed = false;
        inner.auth_success = None;
        inner.client_list = None;
        inner.clients.clear();
        inner.link.take()
    };

    if let Some(link) = link {
        let _ = link.tx.send(LinkCommand::Shutdown);
        set_status(app, link.id, Status::Closed);
    }
}

/// Sends one `{type, message}` envelope. Dropped while there's no open link, as the webview's
/// socket did.
pub fn send(app: &AppHandle, text: String) {
    if let Some(link) = server(app).inner.lock().unwrap().link.as_ref() {
        let _ = link.tx.send(LinkCommand::Send(text));
    }
}

/// The webview's controls and rules, for checking viewer commands. When a profile's limits on a
/// toy change, running toys are brought within them (see `policy::clamp_running`).
pub fn set_policy(app: &AppHandle, policy: policy::Policy) {
    let previous = server(app).inner.lock().unwrap().policy.replace(policy.clone());
    policy::clamp_running(app, previous.as_ref(), &policy);
}

/// How many viewers are on the share page, and how many of them signed in.
pub fn viewer_counts(app: &AppHandle) -> (usize, usize) {
    let inner = server(app).inner.lock().unwrap();
    let signed_in = inner.clients.iter().filter_map(viewer::describe).filter(|viewer| viewer.discord_id.is_some()).count();
    (inner.clients.len(), signed_in)
}

/// The viewers on the share page now, each with the viewer groups they're in.
pub fn viewers(app: &AppHandle) -> Vec<(viewer::Viewer, Vec<String>)> {
    let inner = server(app).inner.lock().unwrap();
    inner
        .clients
        .iter()
        .filter_map(viewer::describe)
        .map(|viewer| {
            let groups = inner.policy.as_ref().and_then(|policy| policy.memberships.get(&viewer.key).cloned()).unwrap_or_default();
            (viewer, groups)
        })
        .collect()
}

/// The control groups of the avatar you're wearing, as the webview last pushed them.
pub fn control_groups(app: &AppHandle) -> Vec<serde_json::Value> {
    server(app).inner.lock().unwrap().policy.as_ref().map(|policy| policy.groups.clone()).unwrap_or_default()
}

/// A control of the avatar you're wearing, as the webview last pushed it.
pub fn find_control(app: &AppHandle, group_id: &str, control_id: &str) -> Option<serde_json::Value> {
    let inner = server(app).inner.lock().unwrap();
    inner.policy.as_ref()?.control(group_id, control_id).map(|(control, _)| control.clone())
}

/// The rules, and the connected viewer with this session id.
fn policy_and_client(app: &AppHandle, peer_id: &str) -> Option<(policy::Policy, serde_json::Value)> {
    let inner = server(app).inner.lock().unwrap();
    let client = inner
        .clients
        .iter()
        .find(|client| client.get("peerId").and_then(serde_json::Value::as_str) == Some(peer_id))?
        .clone();
    Some((inner.policy.clone()?, client))
}

/// Sends a `{type, message}` envelope Rust built itself.
fn send_message(app: &AppHandle, kind: &str, message: serde_json::Value) {
    let text = serde_json::json!({ "type": kind, "message": message }).to_string();
    debug_out(app, &text);
    send(app, text);
}

/// The avatar parameters the visible controls use. Their values go to the server as they change,
/// and every one that has a value is sent now, after the `controls-update` that asked for them.
pub fn set_forwarded(app: &AppHandle, addresses: Vec<String>) {
    let forwarded: HashSet<String> = addresses.into_iter().collect();
    let state = app.state::<AppState>();
    let values: Vec<OscMessage> = {
        let messages = state.messages.lock().unwrap();
        forwarded.iter().filter_map(|address| messages.get(address).cloned()).collect()
    };

    let mut inner = state.server.inner.lock().unwrap();
    inner.forwarded = forwarded;

    if let (true, Some(link)) = (inner.authed, inner.link.as_ref()) {
        for value in values {
            let text = args_update(&value);
            debug_out(app, &text);
            let _ = link.tx.send(LinkCommand::Send(text));
        }
    }
}

/// A parameter changed in VRChat: shows it on the share page when a visible control uses it.
pub fn on_parameter(app: &AppHandle, msg: &OscMessage) {
    let inner = server(app).inner.lock().unwrap();

    if !inner.authed || !inner.forwarded.contains(&msg.address) {
        return;
    }
    if let Some(link) = inner.link.as_ref() {
        let text = args_update(msg);
        debug_out(app, &text);
        let _ = link.tx.send(LinkCommand::Send(text));
    }
}

fn args_update(msg: &OscMessage) -> String {
    serde_json::json!({ "type": "args-update", "message": msg }).to_string()
}

fn auth_message(token: &str, protocol_version: u32) -> String {
    serde_json::json!({
        "type": "auth-token",
        "message": { "token": token, "protocolVersion": protocol_version }
    })
    .to_string()
}

fn is_current(inner: &Inner, id: u64) -> bool {
    inner.link.as_ref().is_some_and(|link| link.id == id)
}

/// From link task `id`. Ignored once that task has been replaced.
fn set_status(app: &AppHandle, id: u64, status: Status) {
    {
        let mut inner = server(app).inner.lock().unwrap();
        // `disconnect` takes the link before reporting Closed for it.
        if !is_current(&inner, id) && !(status == Status::Closed && inner.link.is_none()) {
            return;
        }
        if status != Status::Open {
            inner.authed = false;
            inner.auth_success = None;
            inner.client_list = None;
            inner.clients.clear();
        }
        if inner.status == Some(status) {
            return;
        }
        inner.status = Some(status);
    }

    let _ = app.emit("server-status", status);
}

/// What the link should do after a message from the server.
enum After {
    Continue,
    /// The server won't take us as we are: stop and don't retry.
    Stop(Status)
}

/// A message from the server, from link task `id`: noted here, then handed to the webview.
fn on_message(app: &AppHandle, id: u64, text: String) -> After {
    let value = serde_json::from_str::<serde_json::Value>(&text).unwrap_or_default();
    let kind = value.get("type").and_then(serde_json::Value::as_str).map(str::to_owned);
    let mut joined = Vec::new();
    let mut watching = 0;

    {
        let mut inner = server(app).inner.lock().unwrap();
        if !is_current(&inner, id) {
            return After::Continue;
        }
        match kind.as_deref() {
            Some("auth-success") => {
                inner.authed = true;
                inner.auth_success = Some(text.clone());
            }
            Some("client-list") => {
                // The first list after connecting isn't anyone joining.
                let first = inner.client_list.replace(text.clone()).is_none();
                let clients = value.get("message").and_then(serde_json::Value::as_array).cloned().unwrap_or_default();
                if !first {
                    joined = clients.iter().filter(|client| !inner.clients.iter().any(|old| old.get("peerId") == client.get("peerId"))).filter_map(viewer::describe).map(|viewer| viewer.display_name).collect();
                }
                watching = clients.len();
                inner.clients = clients;
            }
            Some("update-username") => {
                let from = value.get("from").and_then(serde_json::Value::as_str);
                let name = value.pointer("/message/displayName").cloned();
                let client = inner.clients.iter_mut().find(|client| client.get("peerId").and_then(serde_json::Value::as_str) == from);
                if let (Some(client), Some(name)) = (client, name) {
                    if !client.get("user").is_some_and(serde_json::Value::is_object) {
                        client["user"] = serde_json::json!({});
                    }
                    client["user"]["userDefinedDisplayName"] = name;
                }
            }
            _ => {}
        }
    }

    if !joined.is_empty() {
        crate::overlay::viewers_joined(app, joined, watching);
    }
    if kind.as_deref() == Some("client-list") {
        crate::overlay::viewers_changed(app);
    }

    if kind.as_deref() == Some("command") {
        viewer_commands::handle(app, &value);
        return After::Continue;
    }

    let _ = app.emit("server-message", text);

    match kind.as_deref() {
        Some("auth-error") => {
            // The token is no good any more: forget it, which connects again signed out.
            let url = server(app).inner.lock().unwrap().link.as_ref().map(|link| link.url.clone());
            if let Some(url) = url {
                let app = app.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    if let Err(err) = auth::set_token(&app, &url, None) {
                        tracing::error!("Couldn't forget the rejected token: {err}");
                    }
                });
            }
            After::Stop(Status::Rejected)
        }
        Some("protocol-mismatch") => After::Stop(Status::Mismatch),
        _ => After::Continue
    }
}

/// Messages Rust sends by itself, for the webview's `[⇧ WS]` DevTools log. Dev builds only, with
/// the token blanked out.
fn debug_out(app: &AppHandle, text: &str) {
    if cfg!(debug_assertions) {
        let _ = app.emit("server-debug", redact(text));
    }
}

fn redact(text: &str) -> String {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(text) else {
        return text.to_owned();
    };
    if let Some(token) = value.pointer_mut("/message/token") {
        *token = serde_json::Value::String("•••".into());
    }
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_message_shape() {
        let value: serde_json::Value = serde_json::from_str(&auth_message("jwt", 2)).unwrap();
        assert_eq!(value["type"], "auth-token");
        assert_eq!(value["message"]["token"], "jwt");
        assert_eq!(value["message"]["protocolVersion"], 2);
    }

    #[test]
    fn redact_blanks_the_token() {
        let text = redact(&auth_message("secret", 2));
        assert!(!text.contains("secret"));
        assert!(text.contains("protocolVersion"));
    }

    #[test]
    fn redact_leaves_other_messages_alone() {
        let msg = OscMessage { address: "/avatar/parameters/A".into(), args: vec![] };
        let text = args_update(&msg);
        assert_eq!(redact(&text), text);
    }
}
