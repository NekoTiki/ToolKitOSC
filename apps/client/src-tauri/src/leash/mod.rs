//! Avatar leash: someone grabs the leash on your avatar and pulls, and you walk toward them, the
//! job OSCLeash does. The leash's `_IsGrabbed` / `_Stretch` and the compass contacts arrive live
//! from `osc::udp`, `engine` turns them into `/input/*` every tick while a leash is held, and
//! `leash.json` (see `store`) keeps each avatar's walk and run points.
//!
//! `TKOSC/Leash` (bool) turns it on and off from your expression menu. `TKOSC/Panic` and Stop
//! everything let go at once, and nothing moves you until you resume.
mod engine;
pub mod model;
mod store;
pub mod commands;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Notify;

use crate::automations::model::LeashEvent;
use crate::state::AppState;
use crate::types::{OscArg, OscMessage};
use engine::{Checks, Detected, Directions, Engine, Frame, Motion, Output};

/// While a leash is held, or zeros are still to go out.
const TICK: Duration = Duration::from_millis(20);
/// Otherwise: only the page and the checks to keep up. A grab wakes the loop right away.
const IDLE: Duration = Duration::from_millis(100);
/// `leash-state` goes out at most this often.
const VIEW_EVERY: Duration = Duration::from_millis(100);
/// Moving and nothing from VRChat for this long: avatar change, VRChat closed or the connection
/// lost. You stop until it sends again. Walking always changes something (velocity, the stretch).
const SILENCE: Duration = Duration::from_millis(500);
/// Zeros sent after letting go: a lost one would leave you walking.
const RELEASE_ZEROS: u8 = 2;
const TEST_PULL: Duration = Duration::from_secs(1);
const TEST_SPEED: f64 = 0.5;

const SWITCH_PARAMETER: &str = "TKOSC/Leash";
const PANIC_PARAMETER: &str = "TKOSC/Panic";

#[derive(Default)]
pub struct Leash {
    engine: Mutex<Engine>,
    /// `None` until read from disk (see `store`).
    settings: Mutex<Option<model::LeashSettings>>,
    task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    /// Runs the next tick now: a grab, a switch, a test pull.
    wake: Notify,
    run: Mutex<Run>,
    /// The last `leash-state` sent, and when.
    sent: Mutex<Option<(Instant, View)>>
}

#[derive(Default)]
struct Run {
    /// The avatar `engine` has detected, so a new one is detected once its details are in.
    detected_for: Option<String>,
    /// When VRChat last sent a packet (see `SILENCE`).
    last_packet: Option<Instant>,
    /// Something other than zeros went out since the last release.
    sending: bool,
    /// LookHorizontal went out non-zero, so the release zeroes it too.
    looked: bool,
    release: u8,
    test_until: Option<Instant>,
    frame: Frame,
    /// Whether you were running last tick, for the automation events.
    running: bool,
    blocked: Option<Blocked>,
    status: String
}

fn leash(app: &AppHandle) -> &Leash {
    &app.state::<AppState>().inner().leash
}

/// Why a held leash doesn't move you right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Blocked {
    /// Stop everything is on.
    Paused,
    /// "Leash moves me" is off, or `TKOSC/Leash`.
    Off,
    /// Nothing from VRChat for a moment.
    Silent
}

/// What the Leash page shows.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub avatar_id: Option<String>,
    pub avatar_name: Option<String>,
    pub detected: Detected,
    pub checks: Checks,
    pub held: Vec<String>,
    pub active: Option<String>,
    pub stretch: f64,
    pub directions: Directions,
    /// What goes to VRChat now.
    pub output: Output,
    pub blocked: Option<Blocked>,
    pub testing: bool
}

fn round(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

pub fn view(app: &AppHandle) -> View {
    let (avatar_id, avatar_name) = avatar(app);
    let (detected, checks) = {
        let engine = leash(app).engine.lock().unwrap();
        (engine.detected().clone(), engine.checks())
    };
    let run = leash(app).run.lock().unwrap();
    let frame = &run.frame;
    let output = match run.blocked {
        Some(_) if frame.output.holds() => Output::still(Motion::Grabbed),
        _ => frame.output
    };
    let testing = run.test_until.is_some();
    View {
        avatar_id,
        avatar_name,
        detected,
        checks,
        held: frame.held.clone(),
        active: frame.active.clone(),
        stretch: round(frame.stretch),
        directions: frame.directions,
        output: if testing && !output.holds() {
            Output { vertical: TEST_SPEED, motion: Motion::Walking, ..Output::default() }
        } else {
            Output { vertical: round(output.vertical), horizontal: round(output.horizontal), look: round(output.look), ..output }
        },
        blocked: run.blocked,
        testing
    }
}

fn avatar(app: &AppHandle) -> (Option<String>, Option<String>) {
    let details = app.state::<AppState>().avatar_details.lock().unwrap().clone();
    (details.as_ref().map(|d| d.id.clone()), details.map(|d| d.name))
}

/// VRChat sent a packet, live (not the avatar poll).
pub fn packet_received(app: &AppHandle) {
    leash(app).run.lock().unwrap().last_packet = Some(Instant::now());
}

/// A live avatar parameter from VRChat. Values read when an avatar loads never get here, so an
/// avatar never loads walking.
pub fn on_parameter(app: &AppHandle, message: &OscMessage) {
    let Some(name) = message.address.strip_prefix("/avatar/parameters/") else {
        return;
    };
    let value = match message.args.first() {
        Some(OscArg::Number(value)) => engine::Value::Number(*value),
        Some(OscArg::Bool(on)) => engine::Value::Bool(*on),
        _ => return
    };
    if leash(app).task.lock().unwrap().is_none() {
        return;
    }
    let on = match value {
        engine::Value::Bool(on) => on,
        engine::Value::Number(value) => value != 0.0
    };

    match name {
        SWITCH_PARAMETER => set_enabled(app, on),
        // The webview's Stop everything follows and keeps it paused; this lets go even before.
        PANIC_PARAMETER if on => let_go(app),
        _ => {
            if leash(app).engine.lock().unwrap().on_parameter(name, value) {
                leash(app).wake.notify_one();
            }
        }
    }
}

/// "Leash moves me": from the page, `TKOSC/Leash` or an automation.
pub fn set_enabled(app: &AppHandle, on: bool) {
    store::with(app, |settings| {
        let changed = settings.enabled != on;
        settings.enabled = on;
        ((), changed)
    });
    leash(app).wake.notify_one();
}

/// Another avatar: its leashes are found once its details are in.
pub fn avatar_changed(app: &AppHandle) {
    leash(app).engine.lock().unwrap().reset();
    {
        let mut run = leash(app).run.lock().unwrap();
        run.detected_for = None;
        run.test_until = None;
    }
    leash(app).wake.notify_one();
}

/// Lets go of every leash and stops you. A leash moves you again once grabbed again.
fn let_go(app: &AppHandle) {
    leash(app).engine.lock().unwrap().let_go();
    leash(app).run.lock().unwrap().test_until = None;
    leash(app).wake.notify_one();
}

/// Walks you forward for a moment, to check the direction without a second person.
pub fn test_pull(app: &AppHandle) -> Result<(), String> {
    if crate::automations::is_paused(app) {
        return Err("Stop everything is on. Resume first.".into());
    }
    if !store::get(app).enabled {
        return Err("The leash is turned off.".into());
    }
    leash(app).run.lock().unwrap().test_until = Some(Instant::now() + TEST_PULL);
    leash(app).wake.notify_one();
    Ok(())
}

fn detect_if_needed(app: &AppHandle) {
    let details = app.state::<AppState>().avatar_details.lock().unwrap().clone();
    let id = details.as_ref().map(|d| d.id.clone());
    let mut run = leash(app).run.lock().unwrap();
    if run.detected_for == id {
        return;
    }
    let detected = details.map(|d| engine::detect(&d.parameters)).unwrap_or_default();
    leash(app).engine.lock().unwrap().set_detected(detected);
    run.detected_for = id;
}

fn number(message: Option<&OscMessage>) -> Option<f64> {
    match message?.args.first()? {
        OscArg::Number(value) => Some(*value),
        OscArg::Bool(on) => Some(if *on { 1.0 } else { 0.0 }),
        OscArg::Str(_) => None
    }
}

/// One tick. True while it has something to send, so the next one comes soon.
fn tick(app: &AppHandle) -> bool {
    let now = Instant::now();
    detect_if_needed(app);
    let settings = store::get(app);
    let (avatar_id, _) = avatar(app);
    let avatar = settings.for_avatar(avatar_id.as_deref());

    let frame = {
        let state = app.state::<AppState>();
        let messages = state.messages.lock().unwrap();
        let read = |name: &str| number(messages.get(&format!("/avatar/parameters/{name}")));
        leash(app).engine.lock().unwrap().tick(now, &avatar, read)
    };

    let mut run = leash(app).run.lock().unwrap();
    if run.test_until.is_some_and(|until| now >= until) {
        run.test_until = None;
    }
    let silent = !run.last_packet.is_some_and(|at| now.duration_since(at) < SILENCE);
    let blocked = if crate::automations::is_paused(app) {
        Some(Blocked::Paused)
    } else if !settings.enabled {
        Some(Blocked::Off)
    } else if frame.output.holds() && silent {
        Some(Blocked::Silent)
    } else {
        None
    };

    let output = match blocked {
        Some(_) => Output::default(),
        None if frame.output.holds() => frame.output,
        None if run.test_until.is_some() => Output { vertical: TEST_SPEED, motion: Motion::Walking, ..Output::default() },
        None => Output::default()
    };

    let turning = avatar.turning.enabled || run.looked;
    let busy = if output.holds() {
        send(app, &output, turning);
        run.sending = true;
        run.looked = output.look != 0.0;
        run.release = RELEASE_ZEROS;
        true
    } else if run.sending && run.release > 0 {
        send(app, &Output::default(), turning);
        run.release -= 1;
        if run.release == 0 {
            run.sending = false;
            run.looked = false;
        }
        true
    } else {
        false
    };

    let running = output.motion == Motion::Running;
    let events = [
        (run.frame.active.is_none() && frame.active.is_some(), LeashEvent::Grabbed),
        (!run.running && running, LeashEvent::Running),
        (run.frame.active.is_some() && frame.active.is_none(), LeashEvent::Released)
    ];
    run.running = running;
    run.frame = frame;
    run.blocked = blocked;
    let status = status_text(app, &run, &settings);
    let status_changed = run.status != status.text;
    if status_changed {
        run.status = status.text;
    }
    drop(run);

    if status_changed {
        crate::addons::changed(app);
    }
    for (happened, event) in events {
        if happened {
            crate::automations::leash_event(app, event);
        }
    }
    emit_view(app, false);
    busy
}

fn send(app: &AppHandle, output: &Output, turning: bool) {
    let state = app.state::<AppState>();
    let mut messages = vec![
        ("/input/Vertical", rosc::OscType::Float(output.vertical as f32)),
        ("/input/Horizontal", rosc::OscType::Float(output.horizontal as f32))
    ];
    if turning {
        messages.push(("/input/LookHorizontal", rosc::OscType::Float(output.look as f32)));
    }
    messages.push(("/input/Run", rosc::OscType::Int(i32::from(output.run))));
    for (address, value) in messages {
        let message = rosc::OscMessage { addr: address.to_owned(), args: vec![value] };
        if let Err(err) = crate::osc::send_message(&state, message) {
            tracing::debug!("Couldn't send {address}: {err}");
        }
    }
}

fn emit_view(app: &AppHandle, now: bool) {
    let view = view(app);
    let mut sent = leash(app).sent.lock().unwrap();
    let due = match &*sent {
        Some((at, last)) => *last != view && (now || at.elapsed() >= VIEW_EVERY),
        None => true
    };
    if due {
        let _ = app.emit("leash-state", &view);
        *sent = Some((Instant::now(), view));
    }
}

fn start(app: &AppHandle) {
    let mut task = leash(app).task.lock().unwrap();
    if task.is_some() {
        return;
    }
    let app = app.clone();
    *task = Some(tauri::async_runtime::spawn(async move {
        loop {
            let wait = if tick(&app) { TICK } else { IDLE };
            tokio::select! {
                _ = tokio::time::sleep(wait) => {}
                _ = leash(&app).wake.notified() => {}
            }
        }
    }));
}

fn stop(app: &AppHandle) {
    if let Some(task) = leash(app).task.lock().unwrap().take() {
        task.abort();
    }
    leash(app).engine.lock().unwrap().reset();
    let was_sending = {
        let mut run = leash(app).run.lock().unwrap();
        let was_sending = run.sending;
        *run = Run::default();
        was_sending
    };
    if was_sending {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            for _ in 0..RELEASE_ZEROS {
                send(&app, &Output::default(), true);
                tokio::time::sleep(TICK).await;
            }
        });
    }
    emit_view(app, true);
}

fn status_text(app: &AppHandle, run: &Run, settings: &model::LeashSettings) -> crate::addons::Status {
    use crate::addons::{State, Status};
    if !settings.enabled {
        return Status::new(State::Warn, "Turned off");
    }
    if run.detected_for.is_none() {
        return Status::new(State::Ok, "Waiting for an avatar");
    }
    let engine = leash(app).engine.lock().unwrap();
    let detected = engine.detected();
    if detected.leashes.is_empty() {
        Status::new(State::Ok, "No leash on this avatar")
    } else if detected.compass.is_none() || !detected.problems.is_empty() {
        Status::new(State::Warn, "The leash setup needs a look")
    } else if run.frame.active.is_some() {
        Status::new(State::Ok, "Leash grabbed")
    } else {
        Status::new(State::Ok, "Ready")
    }
}

/// The avatar leash as an add-on (see `crate::addons`).
pub struct Addon;

impl crate::addons::Addon for Addon {
    fn manifest(&self) -> crate::addons::Manifest {
        crate::addons::Manifest {
            id: "leash",
            name: "Avatar leash",
            group: "movement",
            pitch: "Someone pulls the leash on your avatar and you walk toward them, like OSCLeash",
            platforms: &["windows", "linux"],
            badge: None,
            needs: &[],
            control_types: &[],
            // Its step, and its When.
            automation_kinds: &["leash-enabled", "leash"],
            pages: &["leash"]
        }
    }

    fn start(&self, app: &AppHandle) {
        start(app);
    }

    fn stop(&self, app: &AppHandle) {
        stop(app);
    }

    fn status(&self, app: &AppHandle) -> crate::addons::Status {
        if leash(app).task.lock().unwrap().is_none() {
            return crate::addons::Status::new(crate::addons::State::Off, "Off");
        }
        let settings = store::get(app);
        let run = leash(app).run.lock().unwrap();
        status_text(app, &run, &settings)
    }

    fn stop_everything(&self, app: &AppHandle) -> crate::addons::StopFuture {
        let_go(app);
        Box::pin(async { Ok(()) })
    }

    fn was_in_use(&self, _app: &AppHandle) -> bool {
        false
    }
}
