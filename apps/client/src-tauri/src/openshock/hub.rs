//! OpenShock's live user hub (`/1/hubs/user`), a SignalR hub spoken by hand over its JSON protocol:
//! a `{"protocol":"json","version":1}` handshake, then records that each end with 0x1E. It tells us
//! which shocker hubs are online, when the list changes, and takes controls over the open socket.
//! The hub only offers WebSockets, so there's no negotiate step.
//!
//! With an API token the server ignores `EmergencyStop`, `Reboot` and `CaptivePortal` (they need a
//! user session), so Stop everything sends `Stop` to each shocker instead.
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use tauri::AppHandle;
use tokio::sync::mpsc;
use tokio::time::{interval_at, sleep, timeout, Instant, MissedTickBehavior};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;

use super::{Control, USER_AGENT};
use crate::net::Socket;

const URL: &str = "wss://api.openshock.app/1/hubs/user";
const RECORD_SEPARATOR: char = '\u{1e}';
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const PING_INTERVAL: Duration = Duration::from_secs(15);
/// SignalR's default server timeout: twice the server's own 15 s keep-alive.
const SILENCE_TIMEOUT: Duration = Duration::from_secs(30);
const MIN_BACKOFF: Duration = Duration::from_secs(2);
const MAX_BACKOFF: Duration = Duration::from_secs(60);

pub enum HubCommand {
    /// Controls, and the name the OpenShock log shows for them.
    Control(Vec<Control>, Option<String>),
    Shutdown
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceOnlineState {
    device: String,
    online: bool
}

pub fn spawn(app: AppHandle, id: u64, token: String) -> mpsc::UnboundedSender<HubCommand> {
    let (tx, rx) = mpsc::unbounded_channel();
    tauri::async_runtime::spawn(run(app, id, token, rx));
    tx
}

async fn run(app: AppHandle, id: u64, token: String, mut commands: mpsc::UnboundedReceiver<HubCommand>) {
    let mut backoff = MIN_BACKOFF;

    loop {
        let was_connected = session(&app, id, &token, &mut commands).await;
        super::hub_connected(&app, id, false);
        let Some(was_connected) = was_connected else {
            return;
        };
        if was_connected {
            backoff = MIN_BACKOFF;
        }

        let retry = sleep(backoff);
        tokio::pin!(retry);
        loop {
            tokio::select! {
                _ = &mut retry => break,
                command = commands.recv() => match command {
                    Some(HubCommand::Shutdown) | None => return,
                    // Callers only send here while connected. One that raced the drop goes by REST.
                    Some(HubCommand::Control(controls, name)) => super::send_by_rest(&app, controls, name)
                }
            }
        }
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

async fn connect(token: &str) -> Result<Socket, String> {
    let mut request = URL.into_client_request().map_err(|e| e.to_string())?;
    let headers = request.headers_mut();
    headers.insert("OpenShockToken", HeaderValue::from_str(token).map_err(|e| e.to_string())?);
    headers.insert("User-Agent", HeaderValue::from_static(USER_AGENT));

    match timeout(CONNECT_TIMEOUT, tokio_tungstenite::connect_async(request)).await {
        Ok(Ok((socket, _))) => Ok(socket),
        Ok(Err(err)) => Err(err.to_string()),
        Err(_) => Err("timed out".into())
    }
}

/// `None` once told to stop, otherwise whether it got as far as the handshake.
async fn session(
    app: &AppHandle,
    id: u64,
    token: &str,
    commands: &mut mpsc::UnboundedReceiver<HubCommand>
) -> Option<bool> {
    let mut socket = match connect(token).await {
        Ok(socket) => socket,
        Err(err) => {
            tracing::debug!("OpenShock hub unreachable: {err}");
            return Some(false);
        }
    };

    if send(&mut socket, &json!({ "protocol": "json", "version": 1 })).await.is_err() {
        return Some(false);
    }

    // The handshake answer is `{}`, or `{"error": ...}`.
    let handshake = timeout(CONNECT_TIMEOUT, async {
        while let Some(Ok(message)) = socket.next().await {
            if let Message::Text(text) = message {
                return records(&text).next().map(|record| record.get("error").is_none());
            }
        }
        None
    })
    .await;
    if handshake != Ok(Some(true)) {
        tracing::warn!("OpenShock hub refused the handshake");
        return Some(false);
    }

    tracing::info!("Connected to the OpenShock hub");
    super::hub_connected(app, id, true);

    let mut ticker = interval_at(Instant::now() + PING_INTERVAL, PING_INTERVAL);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut last_heard = Instant::now();

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                if last_heard.elapsed() > SILENCE_TIMEOUT {
                    tracing::info!("OpenShock hub went quiet");
                    return Some(true);
                }
                if send(&mut socket, &json!({ "type": 6 })).await.is_err() {
                    return Some(true);
                }
            }
            message = socket.next() => {
                last_heard = Instant::now();
                match message {
                    Some(Ok(Message::Text(text))) => {
                        for record in records(&text) {
                            if record.get("type").and_then(Value::as_u64) == Some(7) {
                                tracing::info!("OpenShock hub closed the connection: {:?}", record.get("error"));
                                return Some(true);
                            }
                            on_record(app, id, &record);
                        }
                    }
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return Some(true),
                    Some(Ok(_)) => {}
                }
            }
            command = commands.recv() => match command {
                Some(HubCommand::Control(controls, name)) => {
                    if send(&mut socket, &invocation(&controls, name.as_deref())).await.is_err() {
                        super::send_by_rest(app, controls, name);
                        return Some(true);
                    }
                }
                Some(HubCommand::Shutdown) | None => {
                    let _ = socket.close(None).await;
                    return None;
                }
            }
        }
    }
}

async fn send(socket: &mut Socket, record: &Value) -> Result<(), String> {
    socket.send(Message::text(encode(record))).await.map_err(|e| e.to_string())
}

fn encode(record: &Value) -> String {
    format!("{record}{RECORD_SEPARATOR}")
}

/// The records in one WebSocket message. Unparseable ones are skipped.
fn records(text: &str) -> impl Iterator<Item = Value> + '_ {
    text.split(RECORD_SEPARATOR)
        .filter(|part| !part.is_empty())
        .filter_map(|part| serde_json::from_str(part).ok())
}

/// A `ControlV2` call. No invocation id: the hub doesn't answer it either way.
fn invocation(controls: &[Control], name: Option<&str>) -> Value {
    let shocks: Vec<_> = controls.iter().map(Control::hub_json).collect();
    json!({ "type": 1, "target": "ControlV2", "arguments": [shocks, name] })
}

fn on_record(app: &AppHandle, id: u64, record: &Value) {
    // Only invocations (type 1) carry anything. Pings (6) just count as hearing from it.
    if record.get("type").and_then(Value::as_u64) != Some(1) {
        return;
    }
    let arguments = record.get("arguments").and_then(Value::as_array);

    match record.get("target").and_then(Value::as_str) {
        // Right after connecting, before the first DeviceStatus, which lists the hubs online now.
        Some("Welcome") => super::hubs_online(app, id, Vec::new(), true),
        Some("DeviceStatus") => {
            let states = arguments
                .and_then(|args| args.first())
                .and_then(|states| serde_json::from_value::<Vec<DeviceOnlineState>>(states.clone()).ok())
                .unwrap_or_default();
            let changes = states.into_iter().map(|state| (state.device, state.online)).collect();
            super::hubs_online(app, id, changes, false);
        }
        // A hub was added, renamed or removed, or its shockers changed.
        Some("DeviceUpdate") => super::refresh_hubs(app),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openshock::ControlType;

    #[test]
    fn records_split_on_the_separator() {
        let text = "{\"type\":6}\u{1e}{\"type\":1,\"target\":\"Welcome\",\"arguments\":[\"abc\"]}\u{1e}";
        let parsed: Vec<_> = records(text).collect();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[1]["target"], "Welcome");
    }

    #[test]
    fn encode_ends_with_the_separator() {
        assert_eq!(encode(&json!({ "type": 6 })), "{\"type\":6}\u{1e}");
    }

    #[test]
    fn control_v2_uses_numeric_types() {
        let control = Control { id: "s1".into(), kind: ControlType::Vibrate, intensity: 40, duration: 1000 };
        let value = invocation(&[control], Some("ToolKitOSC · Sam"));
        assert_eq!(value["target"], "ControlV2");
        assert_eq!(value["arguments"][0][0]["type"], 2);
        assert_eq!(value["arguments"][0][0]["intensity"], 40);
        assert_eq!(value["arguments"][1], "ToolKitOSC · Sam");
    }

    #[test]
    fn device_status_parses() {
        let states: Vec<DeviceOnlineState> =
            serde_json::from_value(json!([{ "device": "d1", "online": true, "firmwareVersion": "1.2.3" }])).unwrap();
        assert_eq!(states[0].device, "d1");
        assert!(states[0].online);
    }
}
