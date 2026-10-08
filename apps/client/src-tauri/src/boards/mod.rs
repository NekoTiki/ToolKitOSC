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
pub use store::OutputConfig;
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
    /// Pins the board offers as outputs.
    pub outputs: Vec<OutputPin>,
    /// The outputs set up in this app, None until first saved.
    pub config: Option<Vec<OutputConfig>>,
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
    Shutdown,
    /// Send the saved output config.
    Config,
    /// Pulse this pin briefly.
    Test(u8),
    /// Every output off.
    Stop,
    /// From an automation.
    Output(protocol::OutputMessage)
}

/// How long a Test switches an output on.
const TEST_PULSE_MS: u32 = 500;
const MIN_MAX_ON_MS: u32 = 100;
const MAX_MAX_ON_MS: u32 = 3_600_000;

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
                config: r.config.clone(),
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
fn add_record(app: &AppHandle, mut record: BoardRecord) {
    let id = record.id.clone();
    let records = {
        let mut inner = boards(app).inner.lock().unwrap();
        // Pairing an added board again keeps its outputs.
        if let Some(old) = inner.records.iter().find(|r| r.id == id) {
            record.config = old.config.clone();
        }
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

fn link(app: &AppHandle, id: &str) -> Option<mpsc::UnboundedSender<LinkCommand>> {
    boards(app).inner.lock().unwrap().links.get(id).cloned()
}

fn is_online(app: &AppHandle, id: &str) -> bool {
    boards(app).inner.lock().unwrap().status.get(id).map(|(s, _)| *s) == Some(LinkStatus::Online)
}

/// Saves the board's outputs and sends them to it (now if online, otherwise when it connects).
pub fn save_outputs(app: &AppHandle, id: &str, outputs: Vec<OutputConfig>) -> Result<(), String> {
    let outputs: Vec<OutputConfig> = outputs
        .into_iter()
        .map(|o| OutputConfig { label: o.label.trim().to_string(), ..o })
        .collect();

    let records = {
        let mut inner = boards(app).inner.lock().unwrap();
        let record = inner
            .records
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or("That board isn't added any more.")?;

        for (i, output) in outputs.iter().enumerate() {
            if output.label.is_empty() {
                return Err("Every output needs a name.".into());
            }
            if !record.outputs.iter().any(|p| p.pin == output.pin) {
                return Err(format!("GPIO {} isn't one of the pins this board offers.", output.pin));
            }
            if outputs[..i].iter().any(|o| o.pin == output.pin) {
                return Err(format!("GPIO {} is used by two outputs.", output.pin));
            }
            if !(MIN_MAX_ON_MS..=MAX_MAX_ON_MS).contains(&output.max_on_ms) {
                return Err(format!(
                    "The max on-time of \"{}\" must be between {MIN_MAX_ON_MS} and {MAX_MAX_ON_MS} ms.",
                    output.label
                ));
            }
        }

        record.config = Some(outputs);
        inner.records.clone()
    };

    store::save(app, &records);
    if let Some(link) = link(app, id) {
        let _ = link.send(LinkCommand::Config);
    }
    emit(app);
    Ok(())
}

/// Pulses a saved output briefly, to check the wiring.
pub fn test_output(app: &AppHandle, id: &str, pin: u8) -> Result<(), String> {
    let saved = record(app, id)
        .and_then(|r| r.config)
        .is_some_and(|config| config.iter().any(|o| o.pin == pin));
    if !saved {
        return Err("Save this output before testing it.".into());
    }
    if !is_online(app, id) {
        return Err("The board is offline.".into());
    }

    link(app, id)
        .ok_or_else(|| "The board is offline.".to_string())?
        .send(LinkCommand::Test(pin))
        .map_err(|_| "The board is offline.".to_string())
}

/// Every output of every board off (Stop everything).
pub fn stop_all(app: &AppHandle) {
    let links: Vec<_> = boards(app).inner.lock().unwrap().links.values().cloned().collect();
    for link in links {
        let _ = link.send(LinkCommand::Stop);
    }
}

#[derive(Debug, Clone, Serialize)]
struct BoardError {
    id: String,
    code: String,
    pin: Option<u8>
}

/// An `err` from a board, for the UI to explain why something did nothing.
fn emit_error(app: &AppHandle, id: &str, code: String, pin: Option<u8>) {
    let _ = app.emit("board-error", BoardError { id: id.to_string(), code, pin });
}

fn config_message(app: &AppHandle, id: &str) -> Option<protocol::Outgoing<'static>> {
    let config = record(app, id)?.config?;
    Some(protocol::Outgoing::Config {
        outputs: config
            .iter()
            .map(|o| protocol::ConfigOutput { pin: o.pin, active_low: o.active_low, max_on_ms: o.max_on_ms })
            .collect()
    })
}

/// What automations need to know about an output.
pub struct OutputInfo {
    pub label: String,
    pub pwm: bool,
    pub max_on_ms: u32
}

/// A saved output of an added board, or None if it doesn't exist (any more).
pub fn output_info(app: &AppHandle, board_id: &str, pin: u8) -> Option<OutputInfo> {
    let record = record(app, board_id)?;
    let output = record.config?.into_iter().find(|o| o.pin == pin)?;
    Some(OutputInfo {
        label: output.label,
        pwm: record.outputs.iter().any(|p| p.pin == pin && p.pwm),
        max_on_ms: output.max_on_ms
    })
}

/// Sends an automation's command to a board. False when the board is offline: the command is
/// dropped, never queued, since a fan starting late is worse than not at all.
pub fn send_output(app: &AppHandle, board_id: &str, message: protocol::OutputMessage) -> bool {
    if !is_online(app, board_id) {
        return false;
    }
    link(app, board_id).is_some_and(|link| link.send(LinkCommand::Output(message)).is_ok())
}
