//! The OWO Skin, a shirt that stimulates ten muscle groups with small electrical pulses, through
//! the OWO app on your PC or phone. Avatars set up with vrc-owo-suit's Unity package say which
//! muscle is touched (see `touch`), and the tick sends that to the OWO app (see `protocol`,
//! `link`). It replaces vrc-owo-suit: close that, since ToolKitOSC already has VRChat's OSC.
//!
//! Every writer goes through `arbiter`, which merges them into one sensation: a new one replaces
//! the one playing. It goes out as soon as what plays changes (so viewers' sensations keep their
//! shape), and again every 0.25 s. Each lasts 0.3 s, so the suit goes quiet on its own if the app
//! hangs, and a STOP ends it at once when nothing plays anymore.
//!
//! `TKOSC/OWO` (bool) turns avatar touch on and off from your expression menu, and follows the
//! switch in the app.
mod arbiter;
pub mod control;
mod link;
pub mod model;
mod protocol;
pub mod sensations;
#[cfg(debug_assertions)]
mod simulator;
mod store;
mod touch;
pub mod commands;

use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;
use tokio::time::{interval, MissedTickBehavior};

use crate::state::AppState;
use crate::types::{OscArg, OscCommand, OscMessage};
use arbiter::{Arbiter, Leases, Pace, PRIORITY};
use model::{Muscle, Side};
use protocol::Packet;
pub use touch::Levels;
use touch::Touch;

const TICK: Duration = Duration::from_millis(50);
/// What plays goes out again this often, even unchanged.
const RESEND: Duration = Duration::from_millis(250);
/// How long each sensation lasts, in tenths of a second: a little longer than the tick.
const SENSATION_TENTHS: u8 = 3;
/// `owo-state` goes out at most this often.
const VIEW_EVERY: Duration = Duration::from_millis(100);
/// No OSC from VRChat for this long and touch lets go: VRChat closed, or OSC stopped mid-touch.
const STALE_AFTER: Duration = Duration::from_secs(15);
const SWITCH_PARAMETER: &str = "TKOSC/OWO";
/// Test each muscle plays each one this long, at this level (before its strength).
const TEST_STEP: Duration = Duration::from_millis(600);
const TEST_LEVEL: f64 = 0.5;
const TEST_ID: &str = "test";

/// - `connecting`: looking for the OWO app.
/// - `connected`: it answered. Sensations only go out then.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Disabled,
    Connecting,
    Connected
}

pub enum LinkCommand {
    Send(Packet),
    Shutdown
}

#[derive(Default)]
struct Inner {
    status: Option<Status>,
    /// Where the OWO app answered from.
    peer: Option<SocketAddr>,
    /// Talks to the OWO tester instead (development builds only).
    simulated: bool,
    link: Option<(u64, mpsc::UnboundedSender<LinkCommand>)>,
    /// The link pings this, so a new address in the settings starts a new link.
    target: String,
    next_link_id: u64
}

#[derive(Default)]
pub struct Owo {
    inner: Mutex<Inner>,
    /// `None` until read from disk (see `store`).
    settings: Mutex<Option<model::OwoSettings>>,
    touch: Mutex<Touch>,
    leases: Mutex<Leases>,
    arbiter: Mutex<Arbiter>,
    task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    /// When VRChat last sent anything (see `STALE_AFTER`).
    last_osc: Mutex<Option<Instant>>,
    /// The last `owo-state` sent, and when.
    sent: Mutex<Option<(Instant, View)>>,
    /// The last `TKOSC/OWO` sent. `None` sends it once the worn avatar has it.
    switch_sent: Mutex<Option<bool>>,
    /// The worn avatar's check, by avatar id.
    detected: Mutex<Option<(String, touch::Detected)>>,
    /// Test each muscle is on this muscle.
    testing: Mutex<Option<(Muscle, tauri::async_runtime::JoinHandle<()>)>>,
    /// When the last packet went out, and what it played.
    pace: Mutex<Pace>,
    /// Sensations playing now, by who started them.
    sensations: Mutex<std::collections::HashMap<String, (u64, tauri::async_runtime::JoinHandle<()>)>>,
    /// When each control's cooldown ends (see `control`).
    cooldowns: Mutex<std::collections::HashMap<String, Instant>>
}

fn owo(app: &AppHandle) -> &Owo {
    &app.state::<AppState>().inner().owo
}

/// What the app shows: the OWO app's connection and what each muscle plays.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub status: Status,
    /// Where the OWO app answered from.
    pub address: Option<String>,
    pub simulated: bool,
    /// 0–100, for muscles that play now.
    pub playing: std::collections::BTreeMap<Muscle, u8>,
    /// Sides avatar touch drives now.
    pub touched: Vec<Side>,
    /// The worn avatar and what it has for the OWO Skin.
    pub avatar: Option<AvatarCheck>,
    /// The muscle Test each muscle is on.
    pub testing: Option<Muscle>
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
    let details = app.state::<AppState>().avatar_details.lock().unwrap().clone()?;
    let mut cached = owo(app).detected.lock().unwrap();
    if cached.as_ref().is_none_or(|(id, _)| *id != details.id) {
        let names = details.parameters.iter().map(|parameter| parameter.name.as_str());
        *cached = Some((details.id.clone(), touch::detect(names, SWITCH_PARAMETER)));
    }
    Some(AvatarCheck { id: details.id, name: details.name, detected: cached.as_ref().unwrap().1.clone() })
}

pub fn view(app: &AppHandle) -> View {
    let (status, address, simulated) = {
        let inner = owo(app).inner.lock().unwrap();
        (inner.status.unwrap_or(Status::Disabled), inner.peer.map(|peer| peer.ip().to_string()), inner.simulated)
    };
    let avatar = avatar_check(app);
    let testing = owo(app).testing.lock().unwrap().as_ref().map(|(muscle, _)| *muscle);
    let arbiter = owo(app).arbiter.lock().unwrap();
    let playing = arbiter.playing().iter().map(|(muscle, level)| (*muscle, (level * 100.0).round() as u8)).collect();
    View { status, address, simulated, playing, touched: arbiter.touched(), avatar, testing }
}

fn emit_view(app: &AppHandle, now: bool) {
    let view = view(app);
    let mut sent = owo(app).sent.lock().unwrap();
    let due = match &*sent {
        Some((at, last)) => *last != view && (now || at.elapsed() >= VIEW_EVERY),
        None => true
    };
    if due {
        let _ = app.emit("owo-state", &view);
        *sent = Some((Instant::now(), view));
    }
}

/// VRChat sent something live: it's still there.
pub fn osc_alive(app: &AppHandle) {
    *owo(app).last_osc.lock().unwrap() = Some(Instant::now());
}

/// A live avatar parameter from VRChat. Values read when an avatar loads never get here, so an
/// avatar never loads with a muscle firing.
pub fn on_parameter(app: &AppHandle, message: &OscMessage) {
    let Some(name) = message.address.strip_prefix("/avatar/parameters/") else {
        return;
    };
    if !name.starts_with("owo_suit_") && name != SWITCH_PARAMETER {
        return;
    }
    let on = match message.args.first() {
        Some(OscArg::Bool(on)) => *on,
        Some(OscArg::Number(value)) => *value != 0.0,
        _ => return
    };
    if owo(app).task.lock().unwrap().is_none() {
        return;
    }

    if name == SWITCH_PARAMETER {
        store::with(app, |settings| {
            let changed = settings.enabled != on;
            settings.enabled = on;
            ((), changed)
        });
        return;
    }
    owo(app).touch.lock().unwrap().on_parameter(name, on, Instant::now());
}

/// Another avatar: every muscle stops.
pub fn avatar_changed(app: &AppHandle) {
    owo(app).touch.lock().unwrap().clear();
    // Its menu shows the switch as it is here, not as the avatar saved it.
    *owo(app).switch_sent.lock().unwrap() = None;
}

/// Asks for these levels (0–1 per muscle) until `until`, under `id`. Held back on a side touch
/// drives.
pub fn ask(app: &AppHandle, id: &str, levels: Levels, until: Instant) {
    owo(app).leases.lock().unwrap().set(id, levels, until);
}

/// Lets go of everything `id` asked for.
pub fn let_go(app: &AppHandle, id: &str) {
    owo(app).leases.lock().unwrap().remove(id);
}

/// Plays `sensation` on `muscles` at up to `level`, cut short after `longest`, under `id`: one
/// already playing under the same id stops first. Held back on a side touch drives, like anything
/// else asked.
pub fn play_sensation(app: &AppHandle, id: &str, sensation: &str, muscles: Vec<Muscle>, level: f64, longest: Duration) {
    if !sensations::exists(sensation) || muscles.is_empty() {
        return;
    }
    static RUNS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let run = RUNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if let Some((_, old)) = owo(app).sensations.lock().unwrap().remove(id) {
        old.abort();
    }
    let task = {
        let (app, id, sensation) = (app.clone(), id.to_owned(), sensation.to_owned());
        let level = level.clamp(0.0, 1.0);
        tauri::async_runtime::spawn(async move {
            let started = Instant::now();
            let mut ticker = interval(TICK);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                ticker.tick().await;
                let now = Instant::now();
                let t = now.duration_since(started);
                let Some(part) = sensations::level(&sensation, t).filter(|_| t < longest) else {
                    break;
                };
                // Lasts past the next tick, so the sensation doesn't flicker between them.
                ask(&app, &id, muscles.iter().map(|muscle| (*muscle, part * level)).collect(), now + TICK * 2);
            }
            let_go(&app, &id);
            // Unless another run has taken its place since.
            let mut playing = owo(&app).sensations.lock().unwrap();
            if playing.get(&id).is_some_and(|(other, _)| *other == run) {
                playing.remove(&id);
            }
        })
    };
    owo(app).sensations.lock().unwrap().insert(id.to_owned(), (run, task));
}

/// Runs an OWO automation step. Each gets its own lease, so steps close together overlap rather
/// than cut each other short.
pub fn run_step(app: &AppHandle, action: &crate::automations::model::Action) {
    static STEPS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    if let crate::automations::model::Action::OwoSensation { sensation, muscles, intensity } = action {
        let id = format!("automation:{}", STEPS.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
        play_sensation(app, &id, sensation, muscles.clone(), *intensity, Duration::MAX);
    }
}

/// Walks every muscle, one at a time, so you can feel each one is where it should be.
pub fn test_muscles(app: &AppHandle) -> Result<(), String> {
    if crate::automations::is_paused(app) {
        return Err("Stop everything is on".into());
    }
    let task = {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            for muscle in Muscle::ALL {
                if let Some((testing, _)) = owo(&app).testing.lock().unwrap().as_mut() {
                    *testing = muscle;
                }
                ask(&app, TEST_ID, Levels::from([(muscle, TEST_LEVEL)]), Instant::now() + TEST_STEP);
                emit_view(&app, true);
                tokio::time::sleep(TEST_STEP).await;
            }
            let_go(&app, TEST_ID);
            owo(&app).testing.lock().unwrap().take();
            emit_view(&app, true);
        })
    };
    if let Some((_, old)) = owo(app).testing.lock().unwrap().replace((Muscle::ALL[0], task)) {
        old.abort();
    }
    Ok(())
}

fn stop_test(app: &AppHandle) {
    if let Some((_, task)) = owo(app).testing.lock().unwrap().take() {
        task.abort();
    }
    let_go(app, TEST_ID);
}

fn send(app: &AppHandle, packet: Packet) {
    let inner = owo(app).inner.lock().unwrap();
    if inner.status != Some(Status::Connected) {
        return;
    }
    if let Some((_, tx)) = &inner.link {
        let _ = tx.send(LinkCommand::Send(packet));
    }
}

/// The avatar's `owo_intensity`, when it has one and you use it.
fn avatar_intensity(app: &AppHandle, settings: &model::OwoSettings) -> f64 {
    if !settings.use_avatar_intensity || !avatar_check(app).is_some_and(|avatar| avatar.detected.intensity) {
        return 1.0;
    }
    let address = format!("/avatar/parameters/{}", touch::INTENSITY_PARAMETER);
    match app.state::<AppState>().messages.lock().unwrap().get(&address).and_then(|message| message.args.first()) {
        Some(OscArg::Number(value)) => value.clamp(0.0, 1.0),
        _ => 1.0
    }
}

fn tick(app: &AppHandle) {
    let now = Instant::now();
    let stale = !owo(app).last_osc.lock().unwrap().is_some_and(|at| at.elapsed() < STALE_AFTER);
    if stale {
        owo(app).touch.lock().unwrap().clear();
    }

    let settings = store::get(app);
    if target(app, &settings) != owo(app).inner.lock().unwrap().target {
        restart(app, true);
    }
    send_switch(app, settings.enabled);
    // Stop everything keeps every muscle still until you resume.
    let paused = crate::automations::is_paused(app);
    let touch = if !paused && settings.enabled {
        let level = avatar_intensity(app, &settings);
        owo(app).touch.lock().unwrap().levels(level, now)
    } else {
        Levels::new()
    };
    let asks = {
        let mut leases = owo(app).leases.lock().unwrap();
        if paused {
            leases.clear();
        }
        leases.levels(now)
    };
    let playing = owo(app).arbiter.lock().unwrap().tick(&touch, &asks, |muscle| settings.strength(muscle), PRIORITY, now).clone();
    match owo(app).pace.lock().unwrap().due(&playing, RESEND, now) {
        Some(arbiter::Send::Play) => send(app, protocol::sensation(settings.frequency, SENSATION_TENTHS, &playing).unwrap_or(Packet::Stop)),
        Some(arbiter::Send::Stop) => send(app, Packet::Stop),
        None => {}
    }
    emit_view(app, false);
}

/// `TKOSC/OWO`, so the expression menu follows the switch in the app. Only once the worn avatar
/// has it: its parameters may arrive after the avatar change.
fn send_switch(app: &AppHandle, enabled: bool) {
    let mut sent = owo(app).switch_sent.lock().unwrap();
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

/// Called by the link task `id`. Ignored once that task has been replaced.
fn set_status(app: &AppHandle, id: u64, status: Status, peer: Option<SocketAddr>) {
    {
        let mut inner = owo(app).inner.lock().unwrap();
        if !inner.link.as_ref().is_some_and(|(link_id, _)| *link_id == id) {
            return;
        }
        inner.status = Some(status);
        inner.peer = peer;
    }
    emit_view(app, true);
    crate::addons::changed(app);
}

/// Where the link pings: the OWO tester, the typed address, or everyone on the network.
fn target(app: &AppHandle, settings: &model::OwoSettings) -> String {
    #[cfg(debug_assertions)]
    if owo(app).inner.lock().unwrap().simulated {
        simulator::start();
        return format!("127.0.0.1:{}", simulator::PORT);
    }
    let _ = app;
    link::target(settings.address.as_deref())
}

/// Stops the current link and starts a new one while the add-on is on.
fn restart(app: &AppHandle, on: bool) {
    let target = target(app, &store::get(app));
    let old = {
        let mut inner = owo(app).inner.lock().unwrap();
        let old = inner.link.take();
        inner.peer = None;
        if on {
            inner.next_link_id += 1;
            let id = inner.next_link_id;
            inner.link = Some((id, link::spawn(app.clone(), id, target.clone())));
            inner.target = target;
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

/// Development builds: talk to the OWO tester instead of the OWO app.
pub fn set_simulated(app: &AppHandle, simulated: bool) -> Result<(), String> {
    if !cfg!(debug_assertions) && simulated {
        return Err("The OWO tester is only in development builds".into());
    }
    owo(app).inner.lock().unwrap().simulated = simulated;
    restart(app, true);
    Ok(())
}

/// Development builds: the packets the OWO tester got.
#[cfg(debug_assertions)]
pub fn tester_log() -> Vec<simulator::Entry> {
    simulator::log()
}

/// Every muscle stops now: what's asked is dropped, touch lets go, and the OWO app stops what plays.
fn stop_all(app: &AppHandle) {
    stop_test(app);
    for (_, (_, task)) in owo(app).sensations.lock().unwrap().drain() {
        task.abort();
    }
    owo(app).leases.lock().unwrap().clear();
    owo(app).arbiter.lock().unwrap().clear();
    owo(app).pace.lock().unwrap().clear();
    send(app, Packet::Stop);
    emit_view(app, true);
}

fn start(app: &AppHandle) {
    restart(app, true);
    let mut task = owo(app).task.lock().unwrap();
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
    if let Some(task) = owo(app).task.lock().unwrap().take() {
        task.abort();
    }
    owo(app).touch.lock().unwrap().clear();
    *owo(app).switch_sent.lock().unwrap() = None;
    stop_all(app);
    // The link says STOP and goodbye on its way out.
    restart(app, false);
}

/// The OWO Skin as an add-on (see `crate::addons`).
pub struct Addon;

impl crate::addons::Addon for Addon {
    fn manifest(&self) -> crate::addons::Manifest {
        crate::addons::Manifest {
            id: "owo",
            name: "OWO",
            group: "gear",
            pitch: "Your OWO Skin feels avatar touch, through the OWO app on your PC or phone",
            platforms: &["windows", "linux"],
            // Electrical muscle stimulation: the same care as shockers. Not tried on a real suit yet.
            badges: &["electrical", "beta"],
            needs: &[],
            needs_any: &[],
            control_types: &["owo"],
            automation_kinds: &["owo-sensation"],
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
        let inner = owo(app).inner.lock().unwrap();
        match inner.status.unwrap_or(Status::Disabled) {
            Status::Disabled => S::new(State::Off, "Off"),
            Status::Connecting => S::new(State::Warn, "Looking for the OWO app…"),
            Status::Connected => S::new(State::Ok, format!("Connected · {}", inner.peer.map(|peer| peer.ip().to_string()).unwrap_or_default()))
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
