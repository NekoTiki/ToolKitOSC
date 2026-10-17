//! bHaptics gear (vests, Tactal, sleeves, gloves) through bHaptics Player. Avatars set up with
//! bHaptics' VRChat package say which motor is touched (see `touch`), and the tick sends that to
//! Player as frames (see `protocol`, `link`). Player keeps doing the Bluetooth, battery and
//! firmware side.
//!
//! Every writer goes through `arbiter`: avatar touch first, per position, then whatever viewers,
//! effects and automations ask for through leases that run out by themselves. Frames last 150 ms
//! and the tick sends them every 100 ms, so motors stop on their own if the app hangs.
//!
//! `TKOSC/bHaptics` (bool) turns avatar touch on and off from your expression menu, and follows
//! the switch in the app.
mod arbiter;
mod launch;
mod link;
pub mod model;
mod protocol;
#[cfg(debug_assertions)]
mod simulator;
mod store;
mod touch;
pub mod commands;

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;
use tokio::time::{interval, MissedTickBehavior};

use crate::state::AppState;
use crate::types::{OscArg, OscCommand, OscMessage};
use arbiter::{Arbiter, Leases, PRIORITY};
use model::{Position, TouchSource};
use protocol::Submit;
use touch::{Levels, Touch};

const TICK: Duration = Duration::from_millis(100);
/// `bhaptics-state` goes out at most this often.
const VIEW_EVERY: Duration = Duration::from_millis(100);
/// No OSC from VRChat for this long and touch lets go: VRChat closed, or OSC stopped mid-touch.
const STALE_AFTER: Duration = Duration::from_secs(15);
const SWITCH_PARAMETER: &str = "TKOSC/bHaptics";

/// - `connecting`: an attempt is in flight.
/// - `connected`: Player answered. Frames only go out then.
/// - `not-running`: nothing listens on Player's port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Disabled,
    Connecting,
    Connected,
    NotRunning
}

pub enum LinkCommand {
    /// One packet, as text.
    Send(String),
    Shutdown
}

#[derive(Default)]
struct Inner {
    status: Option<Status>,
    player: protocol::Response,
    installed: bool,
    /// Talks to the simulated Player instead (development builds only).
    simulated: bool,
    link: Option<(u64, mpsc::UnboundedSender<LinkCommand>)>,
    next_link_id: u64
}

#[derive(Default)]
pub struct Bhaptics {
    inner: Mutex<Inner>,
    /// `None` until read from disk (see `store`).
    settings: Mutex<Option<model::BhapticsSettings>>,
    touch: Mutex<Touch>,
    leases: Mutex<Leases>,
    arbiter: Mutex<Arbiter>,
    task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    /// When VRChat last sent anything (see `STALE_AFTER`).
    last_osc: Mutex<Option<Instant>>,
    /// The last `bhaptics-state` sent, and when.
    sent: Mutex<Option<(Instant, View)>>,
    /// The last `TKOSC/bHaptics` sent. `None` sends it once the worn avatar has it.
    switch_sent: Mutex<Option<bool>>
}

fn bhaptics(app: &AppHandle) -> &Bhaptics {
    &app.state::<AppState>().inner().bhaptics
}

/// What the app shows: Player's connection and what each position plays.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub status: Status,
    /// Whether Start bHaptics Player can find an install.
    pub installed: bool,
    pub simulated: bool,
    /// Player's names for what's connected: `Vest`, `Head`, `ForearmL`…
    pub devices: Vec<String>,
    /// 0–100 per motor, for positions that play now.
    pub playing: BTreeMap<Position, Vec<u8>>,
    /// Positions avatar touch drives now.
    pub touched: Vec<Position>,
    /// Keys another app plays on Player right now. While we read touch, that's usually Player's
    /// own VRChat handling, and the vest gets every touch twice.
    pub other_keys: Vec<String>
}

pub fn view(app: &AppHandle) -> View {
    let (status, installed, simulated, devices, other_keys) = {
        let inner = bhaptics(app).inner.lock().unwrap();
        let other_keys = inner.player.active_keys.iter().filter(|key| !key.starts_with("TKOSC_")).cloned().collect();
        (inner.status.unwrap_or(Status::Disabled), inner.installed, inner.simulated, inner.player.connected_positions.clone(), other_keys)
    };
    let arbiter = bhaptics(app).arbiter.lock().unwrap();
    View { status, installed, simulated, devices, playing: arbiter.playing().clone(), touched: arbiter.touched(), other_keys }
}

fn emit_view(app: &AppHandle, now: bool) {
    let view = view(app);
    let mut sent = bhaptics(app).sent.lock().unwrap();
    let due = match &*sent {
        Some((at, last)) => *last != view && (now || at.elapsed() >= VIEW_EVERY),
        None => true
    };
    if due {
        let _ = app.emit("bhaptics-state", &view);
        *sent = Some((Instant::now(), view));
    }
}

/// VRChat sent something live: it's still there.
pub fn osc_alive(app: &AppHandle) {
    *bhaptics(app).last_osc.lock().unwrap() = Some(Instant::now());
}

/// A live avatar parameter from VRChat. Values read when an avatar loads never get here, so an
/// avatar never loads buzzing.
pub fn on_parameter(app: &AppHandle, message: &OscMessage) {
    let Some(name) = message.address.strip_prefix("/avatar/parameters/") else {
        return;
    };
    if !name.starts_with("bOSC/") && !name.starts_with("bHaptics") && name != SWITCH_PARAMETER {
        return;
    }
    let value = match message.args.first() {
        Some(OscArg::Number(value)) => *value,
        Some(OscArg::Bool(on)) => if *on { 1.0 } else { 0.0 },
        _ => return
    };
    if bhaptics(app).task.lock().unwrap().is_none() {
        return;
    }

    if name == SWITCH_PARAMETER {
        let on = value != 0.0;
        store::with(app, |settings| {
            let changed = settings.enabled != on;
            settings.enabled = on;
            ((), changed)
        });
        return;
    }
    bhaptics(app).touch.lock().unwrap().on_parameter(name, value, Instant::now());
}

/// Another avatar: every motor stops.
pub fn avatar_changed(app: &AppHandle) {
    bhaptics(app).touch.lock().unwrap().clear();
    // Its menu shows the switch as it is here, not as the avatar saved it.
    *bhaptics(app).switch_sent.lock().unwrap() = None;
}

/// Asks for these levels (0–1 per motor) on `position` until `until`, under `id`. Held back while
/// touch drives that position.
pub fn ask(app: &AppHandle, id: &str, position: Position, levels: Vec<f64>, until: Instant) {
    bhaptics(app).leases.lock().unwrap().set(id, position, levels, until);
}

/// Lets go of everything `id` asked for.
pub fn let_go(app: &AppHandle, id: &str) {
    bhaptics(app).leases.lock().unwrap().remove(id);
}

fn send(app: &AppHandle, submits: &[Submit]) {
    if submits.is_empty() {
        return;
    }
    let inner = bhaptics(app).inner.lock().unwrap();
    if inner.status != Some(Status::Connected) {
        return;
    }
    if let Some((_, tx)) = &inner.link {
        let _ = tx.send(LinkCommand::Send(protocol::packet(submits)));
    }
}

fn tick(app: &AppHandle) {
    let now = Instant::now();
    let stale = !bhaptics(app).last_osc.lock().unwrap().is_some_and(|at| at.elapsed() < STALE_AFTER);
    if stale {
        bhaptics(app).touch.lock().unwrap().clear();
    }

    let settings = store::get(app);
    send_switch(app, settings.enabled);
    // Stop everything keeps every motor still until you resume.
    let paused = crate::automations::is_paused(app);
    let touch = if !paused && settings.enabled && settings.touch_source == TouchSource::Toolkitosc {
        bhaptics(app).touch.lock().unwrap().levels(settings.touch_from, now)
    } else {
        Levels::new()
    };
    let asks = {
        let mut leases = bhaptics(app).leases.lock().unwrap();
        if paused {
            leases.clear();
        }
        leases.levels(now)
    };
    let submits = bhaptics(app).arbiter.lock().unwrap().tick(&touch, &asks, |position| settings.strength(position), PRIORITY, now);
    send(app, &submits);
    emit_view(app, false);
}

/// `TKOSC/bHaptics`, so the expression menu follows the switch in the app. Only once the worn
/// avatar has it: its parameters may arrive after the avatar change.
fn send_switch(app: &AppHandle, enabled: bool) {
    let mut sent = bhaptics(app).switch_sent.lock().unwrap();
    if *sent == Some(enabled) {
        return;
    }
    let state = app.state::<AppState>();
    let has_parameter = state
        .avatar_details
        .lock()
        .unwrap()
        .as_ref()
        .is_some_and(|details| details.parameters.iter().any(|parameter| parameter.name == SWITCH_PARAMETER));
    if !has_parameter {
        return;
    }
    *sent = Some(enabled);
    let command = OscCommand { address: format!("/avatar/parameters/{SWITCH_PARAMETER}"), args: vec![OscArg::Bool(enabled)] };
    if let Err(err) = crate::osc::send(&state, &command) {
        tracing::warn!("Couldn't send {SWITCH_PARAMETER}: {err}");
    }
}

fn is_current(inner: &Inner, id: u64) -> bool {
    inner.link.as_ref().is_some_and(|(link_id, _)| *link_id == id)
}

// Called by the link task `id`. Each one is ignored once that task has been replaced.

fn set_status(app: &AppHandle, id: u64, status: Status) {
    {
        let mut inner = bhaptics(app).inner.lock().unwrap();
        if !is_current(&inner, id) {
            return;
        }
        if status != Status::Connected {
            inner.player = protocol::Response::default();
        }
        inner.status = Some(status);
    }
    emit_view(app, true);
    crate::addons::changed(app);
}

fn set_player(app: &AppHandle, id: u64, player: protocol::Response) {
    let devices_changed = {
        let mut inner = bhaptics(app).inner.lock().unwrap();
        if !is_current(&inner, id) {
            return;
        }
        let changed = inner.player.connected_positions != player.connected_positions;
        inner.player = player;
        changed
    };
    emit_view(app, true);
    if devices_changed {
        crate::addons::changed(app);
    }
}

fn port(simulated: bool) -> u16 {
    #[cfg(debug_assertions)]
    if simulated {
        simulator::start();
        return simulator::PORT;
    }
    let _ = simulated;
    protocol::PORT
}

/// Stops the current link and starts a new one while the add-on is on.
fn restart(app: &AppHandle, on: bool) {
    let old = {
        let mut inner = bhaptics(app).inner.lock().unwrap();
        let old = inner.link.take();
        inner.player = protocol::Response::default();
        if on {
            inner.next_link_id += 1;
            let id = inner.next_link_id;
            let url = protocol::url(port(inner.simulated));
            inner.link = Some((id, link::spawn(app.clone(), id, url)));
            inner.status = Some(Status::Connecting);
        } else {
            inner.status = Some(Status::Disabled);
        }
        old
    };
    if let Some((_, tx)) = old {
        let _ = tx.send(LinkCommand::Shutdown);
    }
    emit_view(app, true);
    crate::addons::changed(app);
}

/// Opens the local install. The link picks Player up once it listens.
pub fn launch_player() -> Result<(), String> {
    launch::start()
}

/// Development builds: talk to the simulated Player instead of the real one.
pub fn set_simulated(app: &AppHandle, simulated: bool) -> Result<(), String> {
    if !cfg!(debug_assertions) && simulated {
        return Err("The simulated bHaptics Player is only in development builds".into());
    }
    bhaptics(app).inner.lock().unwrap().simulated = simulated;
    restart(app, true);
    Ok(())
}

/// Every motor stops now: what's asked is dropped, touch lets go, and Player turns everything off.
fn stop_all(app: &AppHandle) {
    bhaptics(app).leases.lock().unwrap().clear();
    bhaptics(app).arbiter.lock().unwrap().clear();
    send(app, &[Submit::TurnOffAll]);
    emit_view(app, true);
}

fn start(app: &AppHandle) {
    bhaptics(app).inner.lock().unwrap().installed = launch::available();
    restart(app, true);
    let mut task = bhaptics(app).task.lock().unwrap();
    if task.is_some() {
        return;
    }
    let app = app.clone();
    *task = Some(tauri::async_runtime::spawn(async move {
        let mut ticker = interval(TICK);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            tick(&app);
        }
    }));
}

fn stop(app: &AppHandle) {
    if let Some(task) = bhaptics(app).task.lock().unwrap().take() {
        task.abort();
    }
    bhaptics(app).touch.lock().unwrap().clear();
    *bhaptics(app).switch_sent.lock().unwrap() = None;
    stop_all(app);
    restart(app, false);
}

/// bHaptics gear as an add-on (see `crate::addons`).
pub struct Addon;

impl crate::addons::Addon for Addon {
    fn manifest(&self) -> crate::addons::Manifest {
        crate::addons::Manifest {
            id: "bhaptics",
            name: "bHaptics",
            group: "gear",
            pitch: "Your bHaptics vest, Tactal and sleeves play avatar touch, through bHaptics Player",
            platforms: &["windows"],
            // Not tried on a real vest yet: motor order and effects may still change.
            badge: Some("beta"),
            needs: &[],
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
        use crate::addons::{State, Status as S};
        let inner = bhaptics(app).inner.lock().unwrap();
        let devices = inner.player.connected_positions.len();
        match inner.status.unwrap_or(Status::Disabled) {
            Status::Disabled => S::new(State::Off, "Off"),
            Status::Connecting => S::new(State::Busy, "Connecting…"),
            Status::NotRunning => S::new(State::Warn, "bHaptics Player isn't running"),
            Status::Connected if devices == 0 => S::new(State::Ok, "Connected · no devices yet"),
            Status::Connected => S::new(State::Ok, format!("Connected · {devices} device{}", if devices == 1 { "" } else { "s" }))
        }
    }

    fn stop_everything(&self, app: &AppHandle) -> crate::addons::StopFuture {
        stop_all(app);
        Box::pin(async { Ok(()) })
    }

    fn was_in_use(&self, _app: &AppHandle) -> bool {
        false
    }
}
