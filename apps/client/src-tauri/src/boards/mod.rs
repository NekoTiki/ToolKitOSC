//! ESP32 boards running the ToolKitOSC firmware: found over mDNS (`discovery`), added with a press
//! of their BOOT button (`pairing`), then kept connected over a WebSocket (`link`, one task per
//! board). The protocol is the firmware repo's PROTOCOL.md (`protocol`).
//!
//! Everything the UI shows comes from `Boards::snapshot`, re-emitted as `boards-changed` after
//! every change, so the frontend never has to merge partial updates.
mod discovery;
mod link;
mod pairing;
pub mod protocol;
mod store;

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{mpsc, oneshot};

pub use pairing::{pair, Target};
use protocol::OutputPin;
use store::BoardRecord;

use crate::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LinkStatus {
    Connecting,
    Online,
    Offline,
    /// The board refused our token: it was factory reset, unpaired on the board, or paired to
    /// another PC since. Only pairing again fixes it, so the link stops retrying.
    NeedsPairing,
    /// The board speaks another protocol version.
    UpdateFirmware
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardView {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub fw: Option<String>,
    pub chip: Option<String>,
    pub board: Option<String>,
    pub outputs: Vec<OutputPin>,
    pub status: LinkStatus,
    /// Unix ms, while offline.
    pub offline_since: Option<i64>
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundBoard {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub fw: Option<String>,
    pub proto: Option<u32>,
    /// Some app holds a token for it (TXT `paired=1`).
    pub paired: bool,
    /// Already in this app's list.
    pub added: bool,
    #[serde(skip)]
    fullname: String
}

#[derive(Debug, Clone, Serialize)]
pub struct BoardsSnapshot {
    pub boards: Vec<BoardView>,
    pub found: Vec<FoundBoard>
}

#[derive(Debug)]
pub enum LinkCommand {
    /// mDNS announced the board at a (possibly new) address: reconnect now if offline.
    Address,
    /// Tell the board to forget its token, then stop.
    Unpair,
    Shutdown
}

#[derive(Default)]
struct Inner {
    records: Vec<BoardRecord>,
    status: HashMap<String, (LinkStatus, Option<i64>)>,
    found: HashMap<String, FoundBoard>,
    links: HashMap<String, mpsc::UnboundedSender<LinkCommand>>,
    pairing: Option<oneshot::Sender<()>>
}

#[derive(Default)]
pub struct Boards {
    inner: Mutex<Inner>
}

fn boards(app: &AppHandle) -> &Boards {
    &app.state::<AppState>().inner().boards
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Loads the added boards, connects to each, and starts browsing for boards on the network.
pub fn start(app: AppHandle) {
    let records = store::load(&app);
    tracing::info!("{} ESP32 board(s) added", records.len());

    let ids: Vec<String> = records.iter().map(|r| r.id.clone()).collect();
    boards(&app).inner.lock().unwrap().records = records;

    for id in ids {
        start_link(&app, &id);
    }

    discovery::start(app);
}

pub fn snapshot(app: &AppHandle) -> BoardsSnapshot {
    let inner = boards(app).inner.lock().unwrap();

    let views = inner
        .records
        .iter()
        .map(|r| {
            let (status, offline_since) = inner.status.get(&r.id).copied().unwrap_or((LinkStatus::Connecting, None));
            BoardView {
                id: r.id.clone(),
                name: r.name.clone(),
                host: r.host.clone(),
                port: r.port,
                fw: r.fw.clone(),
                chip: r.chip.clone(),
                board: r.board.clone(),
                outputs: r.outputs.clone(),
                status,
                offline_since
            }
        })
        .collect();

    let mut found: Vec<FoundBoard> = inner
        .found
        .values()
        .map(|f| FoundBoard { added: inner.records.iter().any(|r| r.id == f.id), ..f.clone() })
        .collect();
    found.sort_by(|a, b| a.name.cmp(&b.name));

    BoardsSnapshot { boards: views, found }
}

fn emit(app: &AppHandle) {
    let _ = app.emit("boards-changed", snapshot(app));
}

fn start_link(app: &AppHandle, id: &str) {
    let tx = link::spawn(app.clone(), id.to_string());
    if let Some(old) = boards(app).inner.lock().unwrap().links.insert(id.to_string(), tx) {
        let _ = old.send(LinkCommand::Shutdown);
    }
}

fn set_status(app: &AppHandle, id: &str, status: LinkStatus) {
    {
        let mut inner = boards(app).inner.lock().unwrap();
        let previous = inner.status.get(id).copied();
        if previous.map(|(s, _)| s) == Some(status) {
            return;
        }

        // Keep the time it first went offline across reconnect attempts.
        let since = match (status, previous) {
            (LinkStatus::Offline, Some((LinkStatus::Offline, since))) => since,
            (LinkStatus::Offline, Some((LinkStatus::Connecting, since))) => since.or(Some(now_ms())),
            (LinkStatus::Offline, _) => Some(now_ms()),
            (LinkStatus::Connecting, Some((_, since))) => since,
            _ => None
        };
        inner.status.insert(id.to_string(), (status, since));
    }
    emit(app);
}

fn record(app: &AppHandle, id: &str) -> Option<BoardRecord> {
    boards(app).inner.lock().unwrap().records.iter().find(|r| r.id == id).cloned()
}

/// Saves what the board said about itself in `welcome`.
fn apply_welcome(app: &AppHandle, id: &str, welcome: &protocol::Welcome) {
    let records = {
        let mut inner = boards(app).inner.lock().unwrap();
        let Some(r) = inner.records.iter_mut().find(|r| r.id == id) else {
            return;
        };
        r.name = welcome.name.clone();
        r.fw = Some(welcome.fw.clone());
        r.chip = Some(welcome.chip.clone());
        r.board = Some(welcome.board.clone());
        r.outputs = welcome.outputs.clone();
        inner.records.clone()
    };
    store::save(app, &records);
}

/// Adds the board (or replaces it, when pairing an already added board again) and connects to it.
fn add_record(app: &AppHandle, record: BoardRecord) {
    let id = record.id.clone();
    let records = {
        let mut inner = boards(app).inner.lock().unwrap();
        inner.records.retain(|r| r.id != id);
        inner.records.push(record);
        inner.status.remove(&id);
        inner.records.clone()
    };
    store::save(app, &records);
    start_link(app, &id);
    emit(app);
}

pub fn remove(app: &AppHandle, id: &str) {
    let records = {
        let mut inner = boards(app).inner.lock().unwrap();
        if let Some(link) = inner.links.remove(id) {
            let _ = link.send(LinkCommand::Unpair);
        }
        inner.records.retain(|r| r.id != id);
        inner.status.remove(id);
        inner.records.clone()
    };
    store::save(app, &records);
    tracing::info!("Removed ESP32 board {id}");
    emit(app);
}

fn found_update(app: &AppHandle, found: FoundBoard) {
    let mut moved = None;
    {
        let mut inner = boards(app).inner.lock().unwrap();

        if let Some(r) = inner.records.iter_mut().find(|r| r.id == found.id) {
            if r.host != found.host || r.port != found.port {
                tracing::info!("ESP32 board {} is now at {}:{}", r.id, found.host, found.port);
                r.host = found.host.clone();
                r.port = found.port;
                moved = Some(inner.records.clone());
            }
        }
        if let Some(link) = inner.links.get(&found.id) {
            let _ = link.send(LinkCommand::Address);
        }

        inner.found.insert(found.id.clone(), found);
    }

    if let Some(records) = moved {
        store::save(app, &records);
    }
    emit(app);
}

fn found_remove(app: &AppHandle, fullname: &str) {
    let removed = {
        let mut inner = boards(app).inner.lock().unwrap();
        let before = inner.found.len();
        inner.found.retain(|_, f| f.fullname != fullname);
        inner.found.len() != before
    };
    if removed {
        emit(app);
    }
}

fn found_address(app: &AppHandle, id: &str) -> Option<(String, u16, String)> {
    let inner = boards(app).inner.lock().unwrap();
    inner.found.get(id).map(|f| (f.host.clone(), f.port, f.name.clone()))
}

/// Cancels the pairing in progress, if any.
pub fn cancel_pairing(app: &AppHandle) {
    if let Some(cancel) = boards(app).inner.lock().unwrap().pairing.take() {
        let _ = cancel.send(());
    }
}
