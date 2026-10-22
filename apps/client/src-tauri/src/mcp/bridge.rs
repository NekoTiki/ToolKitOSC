//! Asking the webview: controls, profiles, viewer groups, viewers and Activity live there
//! (localStorage and IndexedDB), and so does the approval pop-up. Rust emits `mcp-request` with an
//! id, `useMcpBridge.ts` answers through `mcp_respond`, and a request nobody answers in time fails
//! instead of hanging the assistant.
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter};
use tokio::sync::oneshot;

/// For reads. The webview answers in milliseconds unless it's gone.
pub const READ_TIMEOUT: Duration = Duration::from_secs(5);
/// For the approval pop-up: left alone this long, it counts as Deny.
pub const APPROVAL_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Default)]
pub struct Bridge {
    pending: Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>,
    next_id: AtomicU64
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Request<'a> {
    id: u64,
    kind: &'a str,
    /// The assistant asking, as the host sees it.
    client: &'a str,
    args: Value
}

/// Asks the webview, and waits for its answer or `timeout`.
pub async fn request(app: &AppHandle, kind: &str, client: &str, args: Value, timeout: Duration) -> Result<Value, String> {
    let bridge = &super::mcp(app).bridge;
    let id = bridge.next_id.fetch_add(1, Ordering::Relaxed);
    let (tx, rx) = oneshot::channel();
    bridge.pending.lock().unwrap().insert(id, tx);
    if let Err(err) = app.emit("mcp-request", Request { id, kind, client, args }) {
        bridge.pending.lock().unwrap().remove(&id);
        return Err(err.to_string());
    }
    match tokio::time::timeout(timeout, rx).await {
        Ok(Ok(answer)) => answer,
        Ok(Err(_)) => Err("ToolKitOSC's window went away".to_owned()),
        Err(_) => {
            bridge.pending.lock().unwrap().remove(&id);
            Err("ToolKitOSC's window didn't answer in time".to_owned())
        }
    }
}

/// The webview's answer to request `id`. One that came too late is dropped.
pub fn respond(app: &AppHandle, id: u64, answer: Result<Value, String>) {
    if let Some(tx) = super::mcp(app).bridge.pending.lock().unwrap().remove(&id) {
        let _ = tx.send(answer);
    }
}
