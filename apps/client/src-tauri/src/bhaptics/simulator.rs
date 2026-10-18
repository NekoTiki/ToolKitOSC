//! A pretend bHaptics Player for development builds, so the whole chain from VRChat to the socket
//! runs without a vest. It listens next to the real one's port, says a vest, a Tactal and both
//! arms are connected, and answers what it's sent the way Player does, with which keys play.
//!
//! Development builds only, three times over: this module is `#[cfg(debug_assertions)]` (a release
//! build doesn't contain it), `set_simulated` refuses in a release, and Settings › bHaptics only
//! shows its switch when `import.meta.env.DEV`.
use std::collections::HashMap;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

pub const PORT: u16 = 15882;
const POSITIONS: [&str; 4] = ["Vest", "Head", "ForearmL", "ForearmR"];

/// Starts listening, once. Errors (the port is taken) are logged, and the link then never
/// connects.
pub fn start() {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        tauri::async_runtime::spawn(async {
            let listener = match TcpListener::bind(("127.0.0.1", PORT)).await {
                Ok(listener) => listener,
                Err(err) => {
                    tracing::error!("The simulated bHaptics Player couldn't listen on {PORT}: {err}");
                    return;
                }
            };
            tracing::info!("Simulated bHaptics Player on port {PORT}");
            while let Ok((stream, _)) = listener.accept().await {
                tauri::async_runtime::spawn(serve(stream));
            }
        });
    });
}

fn response(active: &HashMap<String, Instant>) -> String {
    let now = Instant::now();
    let keys: Vec<&String> = active.iter().filter(|(_, until)| **until > now).map(|(key, _)| key).collect();
    json!({ "ConnectedDeviceCount": 3, "ConnectedPositions": POSITIONS, "ActiveKeys": keys, "RegisteredKeys": [], "Status": {} }).to_string()
}

async fn serve(stream: tokio::net::TcpStream) {
    let Ok(mut socket) = tokio_tungstenite::accept_async(stream).await else {
        return;
    };
    let mut active: HashMap<String, Instant> = HashMap::new();
    let mut status = tokio::time::interval(Duration::from_millis(500));
    loop {
        tokio::select! {
            _ = status.tick() => {
                if socket.send(Message::text(response(&active))).await.is_err() {
                    return;
                }
            }
            message = socket.next() => {
                let text = match message {
                    Some(Ok(Message::Text(text))) => text,
                    Some(Ok(_)) => continue,
                    _ => return
                };
                let Ok(packet) = serde_json::from_str::<Value>(text.as_str()) else { continue };
                for submit in packet["Submit"].as_array().into_iter().flatten() {
                    let key = submit["key"].as_str().unwrap_or_default().to_owned();
                    match submit["type"].as_str() {
                        Some("frame") => {
                            let ms = submit["Frame"]["durationMillis"].as_u64().unwrap_or(0);
                            active.insert(key, Instant::now() + Duration::from_millis(ms));
                        }
                        Some("turnOff") => {
                            active.remove(&key);
                        }
                        Some("turnOffAll") => active.clear(),
                        _ => {}
                    }
                }
                if socket.send(Message::text(response(&active))).await.is_err() {
                    return;
                }
            }
        }
    }
}
