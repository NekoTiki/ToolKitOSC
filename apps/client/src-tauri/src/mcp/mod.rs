//! AI assistants: a local MCP server (Model Context Protocol) that Claude Code, Claude Desktop,
//! Cursor and other MCP clients connect to, to read your avatar and the app's state and, as the
//! levels in Settings › Assistants allow, play with your controls. It listens on 127.0.0.1 only,
//! behind a key kept in the password store (see `crate::secrets`), and only while the add-on is
//! on. Every call passes `guard` first. `mcp.json` (see `store`) holds the port and the levels.
mod bridge;
pub mod commands;
mod guard;
pub mod model;
mod server;
mod store;
mod tools;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio_util::sync::CancellationToken;

use crate::state::AppState;

/// The key's name in the password store.
const KEY_SECRET: &str = "mcp-key";

/// An assistant connected now.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Client {
    id: u64,
    /// From its MCP handshake, e.g. "claude-code".
    name: String,
    /// Unix ms.
    since: u64,
    calls: u32,
    last_tool: Option<String>
}

#[derive(Default)]
pub struct Mcp {
    /// `None` until read from disk (see `store`).
    settings: Mutex<Option<model::McpSettings>>,
    /// Loaded when the server starts, so a request never waits on the password store.
    key: Mutex<Option<String>>,
    /// Stops the server.
    running: Mutex<Option<CancellationToken>>,
    /// Why the server couldn't start, e.g. the port is taken.
    error: Mutex<Option<String>>,
    clients: Mutex<Vec<Client>>,
    next_client: AtomicU64,
    /// Requests waiting for the webview (see `bridge`).
    bridge: bridge::Bridge
}

fn mcp(app: &AppHandle) -> &Mcp {
    &app.state::<AppState>().inner().mcp
}

fn new_key() -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>())
}

/// The saved key, or a new one saved now. Blocks on the password store.
fn load_key(app: &AppHandle) -> Result<String, String> {
    if let Some(key) = crate::secrets::get(app, KEY_SECRET) {
        return Ok(key);
    }
    let key = new_key();
    crate::secrets::set(app, KEY_SECRET, Some(&key))?;
    Ok(key)
}

fn start(app: &AppHandle) {
    let token = CancellationToken::new();
    if let Some(previous) = mcp(app).running.lock().unwrap().replace(token.clone()) {
        previous.cancel();
    }
    *mcp(app).error.lock().unwrap() = None;
    let port = store::get(app).port;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let handle = app.clone();
        let key = tauri::async_runtime::spawn_blocking(move || load_key(&handle)).await.map_err(|e| e.to_string()).and_then(|key| key);
        let result = match key {
            Ok(key) => {
                *mcp(&app).key.lock().unwrap() = Some(key);
                server::serve(app.clone(), port, token.clone()).await
            }
            Err(err) => Err(format!("Couldn't read the key: {err}"))
        };
        if let Err(err) = result {
            tracing::error!("MCP server: {err}");
            // Only if this run is still the current one: a restart may have replaced it.
            if !token.is_cancelled() {
                *mcp(&app).error.lock().unwrap() = Some(err);
                crate::addons::changed(&app);
            }
        }
    });
}

fn stop(app: &AppHandle) {
    if let Some(token) = mcp(app).running.lock().unwrap().take() {
        token.cancel();
    }
    mcp(app).clients.lock().unwrap().clear();
    clients_changed(app);
}

/// Restarts the server after a port change, when it's on.
fn restart(app: &AppHandle) {
    if crate::addons::is_on(app, "mcp") {
        start(app);
        crate::addons::changed(app);
    }
}

fn clients_changed(app: &AppHandle) {
    let _ = app.emit("mcp-clients", clients(app));
    crate::addons::changed(app);
}

pub fn clients(app: &AppHandle) -> Vec<Client> {
    mcp(app).clients.lock().unwrap().clone()
}

/// An assistant finished its handshake. Returns its id, for `note_call` and `disconnected`.
fn connected(app: &AppHandle, name: String) -> u64 {
    let id = mcp(app).next_client.fetch_add(1, Ordering::Relaxed);
    tracing::info!("MCP: {name} connected");
    let since = chrono::Utc::now().timestamp_millis() as u64;
    mcp(app).clients.lock().unwrap().push(Client { id, name, since, calls: 0, last_tool: None });
    clients_changed(app);
    id
}

fn disconnected(app: &AppHandle, id: u64) {
    let removed = {
        let mut clients = mcp(app).clients.lock().unwrap();
        let before = clients.len();
        clients.retain(|client| client.id != id);
        clients.len() != before
    };
    if removed {
        clients_changed(app);
    }
}

fn note_call(app: &AppHandle, id: u64, tool: &str) {
    if let Some(client) = mcp(app).clients.lock().unwrap().iter_mut().find(|client| client.id == id) {
        client.calls += 1;
        client.last_tool = Some(tool.to_owned());
    }
    let _ = app.emit("mcp-clients", clients(app));
}

/// AI assistants as an add-on (see `crate::addons`).
pub struct Addon;

impl crate::addons::Addon for Addon {
    fn manifest(&self) -> crate::addons::Manifest {
        crate::addons::Manifest {
            id: "mcp",
            name: "AI assistants",
            group: "assistants",
            pitch: "Let Claude, Cursor and other AI assistants on this PC read your avatar and play with your controls, within limits you set",
            platforms: &["windows", "linux"],
            badges: &["beta"],
            needs: &[],
            needs_any: &[],
            control_types: &[],
            automation_kinds: &[],
            pages: &[]
        }
    }

    fn start(&self, app: &AppHandle) {
        start(app);
    }

    fn stop(&self, app: &AppHandle) {
        stop(app);
    }

    fn status(&self, app: &AppHandle) -> crate::addons::Status {
        use crate::addons::{State, Status};
        if let Some(err) = mcp(app).error.lock().unwrap().clone() {
            return Status::new(State::Error, err);
        }
        let names: Vec<String> = clients(app).into_iter().map(|client| client.name).collect();
        match names.as_slice() {
            [] => Status::new(State::Ok, "No assistant connected"),
            [name] => Status::new(State::Ok, format!("{name} connected")),
            _ => Status::new(State::Ok, format!("{} assistants connected", names.len()))
        }
    }

    fn stop_everything(&self, _app: &AppHandle) -> crate::addons::StopFuture {
        Box::pin(async { Ok(()) })
    }

    fn was_in_use(&self, _app: &AppHandle) -> bool {
        false
    }
}
