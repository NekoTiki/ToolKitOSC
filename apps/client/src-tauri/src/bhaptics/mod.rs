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
pub mod control;
pub mod effects;
mod launch;
mod link;
pub mod model;
mod protocol;
#[cfg(debug_assertions)]
mod simulator;
mod store;
mod touch;
pub mod commands;

use std::collections::{BTreeMap, VecDeque};
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
pub use touch::Levels;
use touch::Touch;

const TICK: Duration = Duration::from_millis(100);
/// `bhaptics-state` goes out at most this often.
const VIEW_EVERY: Duration = Duration::from_millis(100);
/// No OSC from VRChat for this long and touch lets go: VRChat closed, or OSC stopped mid-touch.
const STALE_AFTER: Duration = Duration::from_secs(15);
const SWITCH_PARAMETER: &str = "TKOSC/bHaptics";
/// Copy diagnostics keeps what was sent this long.
const HISTORY: Duration = Duration::from_secs(30);
/// Test motors plays each motor this long, at this level.
const TEST_STEP: Duration = Duration::from_millis(400);
const TEST_LEVEL: f64 = 0.6;
const TEST_ID: &str = "test";

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
    switch_sent: Mutex<Option<bool>>,
    /// The worn avatar's check, by avatar id.
    detected: Mutex<Option<(String, touch::Detected)>>,
    /// What was sent lately, for Copy diagnostics.
    history: Mutex<VecDeque<(Instant, String)>>,
    /// Test motors is walking this position.
    testing: Mutex<Option<(Position, tauri::async_runtime::JoinHandle<()>)>>,
    /// Viewers' holds going on now (see `control`).
    holds: Mutex<std::collections::HashMap<String, control::Hold>>,
    /// What avatar haptics' SPS zones play on your gear (see `crate::haptics`).
    zones: Mutex<Levels>,
    /// Effects playing now, by who started them.
    effects: Mutex<std::collections::HashMap<String, (u64, tauri::async_runtime::JoinHandle<()>)>>
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
    /// The positions those devices play.
    pub connected: Vec<Position>,
    /// 0–100 per motor, for positions that play now.
    pub playing: BTreeMap<Position, Vec<u8>>,
    /// Positions avatar touch drives now.
    pub touched: Vec<Position>,
    /// Keys another app plays on Player right now. While we read touch, that's usually Player's
    /// own VRChat handling, and the vest gets every touch twice.
    pub other_keys: Vec<String>,
    /// The worn avatar and what it has for bHaptics.
    pub avatar: Option<AvatarCheck>,
    /// The position Test motors is walking.
    pub testing: Option<Position>
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvatarCheck {
    pub id: String,
    pub name: String,
    #[serde(flatten)]
    pub detected: touch::Detected
}

/// The worn avatar's check, worked out again only when the avatar changes.
fn avatar_check(app: &AppHandle) -> Option<AvatarCheck> {
    let (id, name, detected) = {
        let details = app.state::<AppState>().avatar_details.lock().unwrap().clone()?;
        let mut cached = bhaptics(app).detected.lock().unwrap();
        if cached.as_ref().is_none_or(|(id, _)| *id != details.id) {
            let names = details.parameters.iter().map(|parameter| parameter.name.as_str());
            *cached = Some((details.id.clone(), touch::detect(names, SWITCH_PARAMETER)));
        }
        (details.id, details.name, cached.as_ref().unwrap().1.clone())
    };
    Some(AvatarCheck { id, name, detected })
}

pub fn connected(app: &AppHandle) -> bool {
    bhaptics(app).inner.lock().unwrap().status == Some(Status::Connected)
}

pub fn view(app: &AppHandle) -> View {
    let (status, installed, simulated, devices, other_keys) = {
        let inner = bhaptics(app).inner.lock().unwrap();
        let other_keys = inner.player.active_keys.iter().filter(|key| !key.starts_with("TKOSC_")).cloned().collect();
        (inner.status.unwrap_or(Status::Disabled), inner.installed, inner.simulated, inner.player.connected_positions.clone(), other_keys)
    };
    let connected = Position::ALL.into_iter().filter(|position| devices.iter().any(|device| device == position.device())).collect();
    let avatar = avatar_check(app);
    let testing = bhaptics(app).testing.lock().unwrap().as_ref().map(|(position, _)| *position);
    let arbiter = bhaptics(app).arbiter.lock().unwrap();
    View { status, installed, simulated, devices, connected, playing: arbiter.playing().clone(), touched: arbiter.touched(), other_keys, avatar, testing }
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
    if !name.starts_with("bOSC") && !name.starts_with("bHaptics") && name != SWITCH_PARAMETER {
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

/// Avatar haptics' levels for your gear this tick, from SPS zones. They count as avatar touch.
pub fn set_zones(app: &AppHandle, levels: Levels) {
    *bhaptics(app).zones.lock().unwrap() = levels;
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

/// Plays `effect` on `positions` at up to `level`, under `id`: one already playing under the same
/// id stops first. Held back on positions touch drives, like anything else asked.
pub fn play_effect(app: &AppHandle, id: &str, effect: &str, positions: Vec<Position>, level: f64) {
    if !effects::exists(effect) || positions.is_empty() {
        return;
    }
    static RUNS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let run = RUNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if let Some((_, old)) = bhaptics(app).effects.lock().unwrap().remove(id) {
        old.abort();
    }
    let_go(app, id);
    let task = {
        let (app, id, effect) = (app.clone(), id.to_owned(), effect.to_owned());
        let seed = rand::random::<u64>();
        tauri::async_runtime::spawn(async move {
            let started = Instant::now();
            let mut ticker = interval(TICK);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                ticker.tick().await;
                let now = Instant::now();
                let Some(levels) = effects::levels(&effect, &positions, now.duration_since(started), seed) else {
                    break;
                };
                for (position, motors) in levels {
                    // Lasts past the next tick, so the effect doesn't flicker between them.
                    ask(&app, &id, position, motors.iter().map(|m| m * level).collect(), now + TICK + TICK / 2);
                }
            }
            // Unless another run has taken its place since.
            let mut playing = bhaptics(&app).effects.lock().unwrap();
            if playing.get(&id).is_some_and(|(other, _)| *other == run) {
                playing.remove(&id);
            }
        })
    };
    bhaptics(app).effects.lock().unwrap().insert(id.to_owned(), (run, task));
}

/// Runs a bHaptics automation step. Pulses each get their own lease, so steps close together
/// overlap rather than cut each other short.
pub fn run_step(app: &AppHandle, action: &crate::automations::model::Action) {
    use crate::automations::model::Action;
    static PULSES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    match action {
        Action::BhapticsPulse { position, motors, intensity, ms } => {
            let level = intensity.clamp(0.0, 1.0);
            let mut levels = vec![if motors.is_empty() { level } else { 0.0 }; position.motors()];
            let count = position.motors();
            motors.iter().filter(|m| **m < count).for_each(|m| levels[*m] = level);
            let id = format!("automation:{}", PULSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
            ask(app, &id, *position, levels, Instant::now() + Duration::from_millis(u64::from(*ms)));
        }
        Action::BhapticsEffect { effect, positions, intensity } => {
            play_effect(app, &format!("automation:{effect}"), effect, positions.clone(), intensity.clamp(0.0, 1.0));
        }
        _ => {}
    }
}

/// Walks every motor of `position`, one at a time, so you can feel the order is right. Starting
/// again on another position stops the first.
pub fn test_motors(app: &AppHandle, position: Position) -> Result<(), String> {
    if crate::automations::is_paused(app) {
        return Err("Stop everything is on".into());
    }
    let task = {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            for motor in 0..position.motors() {
                let mut levels = vec![0.0; position.motors()];
                levels[motor] = TEST_LEVEL;
                let_go(&app, TEST_ID);
                ask(&app, TEST_ID, position, levels, Instant::now() + TEST_STEP);
                tokio::time::sleep(TEST_STEP).await;
            }
            let_go(&app, TEST_ID);
            bhaptics(&app).testing.lock().unwrap().take();
            emit_view(&app, true);
        })
    };
    if let Some((_, old)) = bhaptics(app).testing.lock().unwrap().replace((position, task)) {
        old.abort();
    }
    emit_view(app, true);
    Ok(())
}

fn stop_test(app: &AppHandle) {
    if let Some((_, task)) = bhaptics(app).testing.lock().unwrap().take() {
        task.abort();
    }
    let_go(app, TEST_ID);
}

/// One line per submit, for Copy diagnostics: `VestFront 6:70 9:35`, `turnOff Head`.
fn describe(submit: &Submit) -> String {
    match submit {
        Submit::Frame { position, intensities } => {
            let dots: Vec<String> = intensities.iter().enumerate().filter(|(_, i)| **i > 0).map(|(m, i)| format!("{m}:{i}")).collect();
            format!("{} {}", position.name(), dots.join(" "))
        }
        Submit::TurnOff(position) => format!("turnOff {}", position.name()),
        Submit::TurnOffAll => "turnOffAll".into()
    }
}

fn remember(app: &AppHandle, submits: &[Submit]) {
    let now = Instant::now();
    let mut history = bhaptics(app).history.lock().unwrap();
    history.extend(submits.iter().map(|submit| (now, describe(submit))));
    while history.front().is_some_and(|(at, _)| now.duration_since(*at) > HISTORY) {
        history.pop_front();
    }
}

/// What a problem found at someone else's place needs to be fixed back home: Player's answers,
/// the settings, the worn avatar's check and the last 30 s of frames.
pub fn diagnostics(app: &AppHandle) -> String {
    let view = view(app);
    let player = bhaptics(app).inner.lock().unwrap().player.clone();
    let settings = store::get(app);
    let now = Instant::now();
    let history: Vec<String> = bhaptics(app)
        .history
        .lock()
        .unwrap()
        .iter()
        .map(|(at, line)| format!("  -{:>5} ms  {line}", now.duration_since(*at).as_millis()))
        .collect();
    format!(
        "ToolKitOSC {} · bHaptics

Status: {:?}{}
Player: {} device(s), positions {:?}, active keys {:?}
(Player's version isn't reported on this API.)

Settings:
{}

Avatar:
{}

Sent in the last 30 s ({} lines):
{}
",
        app.package_info().version,
        view.status,
        if view.simulated { " (simulated Player)" } else { "" },
        player.connected_device_count,
        player.connected_positions,
        player.active_keys,
        pretty(&settings),
        pretty(&view.avatar),
        history.len(),
        history.join("
")
    )
}

fn pretty(value: &impl Serialize) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
}

fn send(app: &AppHandle, submits: &[Submit]) {
    if submits.is_empty() {
        return;
    }
    remember(app, submits);
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
    let mut touch = if !paused && settings.enabled && settings.touch_source == TouchSource::Toolkitosc {
        bhaptics(app).touch.lock().unwrap().levels(settings.touch_from, now)
    } else {
        Levels::new()
    };
    // SPS zones are ours to play whoever reads bHaptics' own touch: Player doesn't know them.
    if !paused && settings.enabled {
        for (position, levels) in bhaptics(app).zones.lock().unwrap().iter() {
            let motors = touch.entry(*position).or_insert_with(|| vec![0.0; position.motors()]);
            for (motor, level) in motors.iter_mut().zip(levels) {
                *motor = motor.max(*level);
            }
        }
    }
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
    stop_test(app);
    bhaptics(app).holds.lock().unwrap().clear();
    for (_, (_, task)) in bhaptics(app).effects.lock().unwrap().drain() {
        task.abort();
    }
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
            badges: &["beta"],
            needs: &[],
            needs_any: &[],
            control_types: &["bhaptics"],
            automation_kinds: &["bhaptics-pulse", "bhaptics-effect"],
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
