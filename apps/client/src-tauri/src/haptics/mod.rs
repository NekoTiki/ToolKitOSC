//! Avatar haptics: the SPS sockets, plugs and touch zones on the avatar you wear drive your toys, the
//! job OscGoesBrrr does. Contacts arrive live from `osc::udp`, `engine` turns them into one level
//! per actuator every tick, and `haptics.json` (see `store`) says which toy follows what: a global
//! setup, and per-avatar overrides.
//!
//! Two avatar parameters: `TKOSC/Haptics` (bool) turns haptics on and off from your expression
//! menu, and follows the switch on the page, and `TKOSC/HapticsLevel` (float) gets the strongest level back, for shaders or sounds.
mod depth;
mod engine;
pub mod model;
mod store;
pub(crate) mod zones;
pub mod commands;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::{interval, MissedTickBehavior};

use crate::state::AppState;
use crate::types::{OscArg, OscCommand, OscMessage};
use engine::{Engine, OutputLevel, ZoneView};
use model::HapticsSettings;

const TICK: Duration = Duration::from_millis(50);
/// `haptics-state` goes out at most this often.
const VIEW_EVERY: Duration = Duration::from_millis(100);
/// No OSC from VRChat for this long and every zone reads 0: VRChat closed, or OSC stopped
/// mid-touch. A contact held still sends nothing, so this can't be per contact.
const STALE_AFTER: Duration = Duration::from_secs(15);
/// Contacts moved this recently: shown as a hint while no toy follows them (see `View::ogb_hint`).
const CONTACTS_RECENT: Duration = Duration::from_secs(10);
/// `TKOSC/HapticsLevel` goes out at most this often.
const LEVEL_EVERY: Duration = Duration::from_millis(100);

/// SPS sockets and plugs only turn their contacts on while a haptics app says it's there, every
/// few seconds (touch zones always send). OscGoesBrrr's parameter, which SPS listens for.
const APP_PARAMETER: &str = "OGB_ENABLED";
const APP_EVERY: Duration = Duration::from_secs(5);
const SWITCH_PARAMETER: &str = "TKOSC/Haptics";
const LEVEL_PARAMETER: &str = "TKOSC/HapticsLevel";

#[derive(Default)]
pub struct Haptics {
    engine: Mutex<Engine>,
    /// `None` until read from disk (see `store`).
    settings: Mutex<Option<HapticsSettings>>,
    task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    levels: Mutex<Vec<OutputLevel>>,
    /// The last `haptics-state` sent, and when.
    sent: Mutex<Option<(Instant, View)>>,
    /// The last `toys-state` sent.
    touched: Mutex<Option<Vec<crate::toys::Touched>>>,
    /// When VRChat last sent anything (see `STALE_AFTER`).
    last_osc: Mutex<Option<Instant>>,
    /// When a contact parameter last changed.
    last_contact: Mutex<Option<Instant>>,
    /// The last `TKOSC/HapticsLevel` sent, and when.
    level_sent: Mutex<Option<(Instant, f64)>>,
    /// The last `OGB_ENABLED` sent, and when. `None` sends it on the next tick.
    app_sent: Mutex<Option<(Instant, bool)>>,
    /// The last `TKOSC/Haptics` sent. `None` sends it once the worn avatar has it.
    switch_sent: Mutex<Option<bool>>
}

fn haptics(app: &AppHandle) -> &Haptics {
    &app.state::<AppState>().inner().haptics
}

/// What the Haptics page shows: the worn avatar's zones and each set-up actuator's level.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub avatar_id: Option<String>,
    pub zones: Vec<ZoneView>,
    pub outputs: Vec<OutputLevel>,
    /// Avatar contacts are moving but no toy follows them here: OscGoesBrrr may still be running,
    /// and once a toy is set up both apps would drive it.
    pub ogb_hint: bool
}

fn avatar_id(app: &AppHandle) -> Option<String> {
    app.state::<AppState>().avatar_details.lock().unwrap().as_ref().map(|details| details.id.clone())
}

pub fn view(app: &AppHandle) -> View {
    View {
        avatar_id: avatar_id(app),
        zones: haptics(app).engine.lock().unwrap().zones(),
        outputs: haptics(app).levels.lock().unwrap().clone(),
        ogb_hint: ogb_hint(app)
    }
}

fn ogb_hint(app: &AppHandle) -> bool {
    let recent = haptics(app).last_contact.lock().unwrap().is_some_and(|at| at.elapsed() < CONTACTS_RECENT);
    recent && !store::get(app).active_outputs(avatar_id(app).as_deref()).iter().any(|output| !output.sources.is_empty())
}

/// VRChat sent something live: it's still there.
pub fn osc_alive(app: &AppHandle) {
    *haptics(app).last_osc.lock().unwrap() = Some(Instant::now());
}

/// A live avatar parameter from VRChat. Values read when an avatar loads never get here, so they
/// never drive a toy.
pub fn on_parameter(app: &AppHandle, message: &OscMessage) {
    let Some(name) = message.address.strip_prefix("/avatar/parameters/") else {
        return;
    };
    let contact = name.starts_with("OGB/") || name.starts_with("VFH/") || name.starts_with("TPS_Internal/");
    if !contact && name != SWITCH_PARAMETER {
        return;
    }
    let value = match message.args.first() {
        Some(OscArg::Number(value)) => zones::Value::Number(*value),
        Some(OscArg::Bool(on)) => zones::Value::Bool(*on),
        _ => return
    };
    if haptics(app).task.lock().unwrap().is_none() {
        return;
    }

    if name == SWITCH_PARAMETER {
        let on = match value {
            zones::Value::Bool(on) => on,
            zones::Value::Number(value) => value != 0.0
        };
        store::with(app, |settings| {
            let changed = settings.enabled != on;
            settings.enabled = on;
            ((), changed)
        });
        return;
    }

    if haptics(app).engine.lock().unwrap().on_parameter(name, value) {
        *haptics(app).last_contact.lock().unwrap() = Some(Instant::now());
    }
}

/// Intiface's toys changed: a toy seen for the first time gets the default sources.
pub fn devices_changed(app: &AppHandle) {
    if haptics(app).task.lock().unwrap().is_none() {
        return;
    }
    let actuators: Vec<model::Actuator> = crate::intiface::devices(app)
        .into_iter()
        .flat_map(|device| {
            device.actuators.into_iter().map(move |actuator| model::Actuator {
                device_index: device.index,
                actuator_index: actuator.index,
                actuator_type: actuator.actuator_type,
                device_name: device.name.clone(),
                actuator_description: actuator.description
            })
        })
        .collect();
    store::with(app, |settings| ((), settings.add_missing(actuators)));
}

/// Another avatar: its zones start from nothing.
pub fn avatar_changed(app: &AppHandle) {
    haptics(app).engine.lock().unwrap().clear();
    // The new avatar's sockets and plugs hear we're here right away.
    *haptics(app).app_sent.lock().unwrap() = None;
    // Its menu shows the switch as it is here, not as the avatar saved it.
    *haptics(app).switch_sent.lock().unwrap() = None;
}

/// Tells the avatar's sockets and plugs whether haptics are on: right away when that changes or
/// the avatar does, then again every few seconds while on.
fn send_app_enabled(app: &AppHandle, enabled: bool) {
    {
        let mut sent = haptics(app).app_sent.lock().unwrap();
        let due = match *sent {
            None => true,
            Some((at, last)) => last != enabled || (enabled && at.elapsed() >= APP_EVERY)
        };
        if !due {
            return;
        }
        *sent = Some((Instant::now(), enabled));
    }
    let state = app.state::<AppState>();
    let command = OscCommand { address: format!("/avatar/parameters/{APP_PARAMETER}"), args: vec![OscArg::Bool(enabled)] };
    if let Err(err) = crate::osc::send(&state, &command) {
        tracing::debug!("Couldn't send {APP_PARAMETER}: {err}");
    }
}

fn tick(app: &AppHandle) {
    let stale = !haptics(app).last_osc.lock().unwrap().is_some_and(|at| at.elapsed() < STALE_AFTER);
    if stale {
        haptics(app).engine.lock().unwrap().clear();
    }

    let settings = store::get(app);
    send_app_enabled(app, settings.enabled_for(avatar_id(app).as_deref()));
    send_switch(app, settings.enabled);
    let outputs = settings.active_outputs(avatar_id(app).as_deref());
    let levels = haptics(app).engine.lock().unwrap().tick(Instant::now(), &outputs);

    // Stop everything keeps haptics off until you resume.
    let touches = if crate::automations::is_paused(app) {
        Vec::new()
    } else {
        outputs
            .iter()
            .zip(&levels)
            .filter(|(output, _)| !output.sources.is_empty())
            .map(|(output, level)| crate::toys::Touch {
                actuator: crate::toys::ActuatorRef {
                    device_index: output.actuator.device_index,
                    actuator_index: output.actuator.actuator_index,
                    actuator_type: output.actuator.actuator_type.clone()
                },
                level: level.level
            })
            .collect()
    };
    let priority = crate::toys::Priority {
        threshold: settings.priority.threshold.clamp(0.0, 1.0),
        grace: Duration::from_millis(settings.priority.grace_ms)
    };
    let strongest = touches.iter().map(|touch| touch.level).fold(0.0, f64::max);
    crate::toys::set_touch(app, touches, priority);
    send_level(app, strongest);

    *haptics(app).levels.lock().unwrap() = levels;
    emit_view(app, false);
    emit_touched(app);
}

/// `TKOSC/Haptics`, so the expression menu follows the switch on the page. Only once the worn avatar
/// has it: its parameters may arrive after the avatar change.
fn send_switch(app: &AppHandle, enabled: bool) {
    let mut sent = haptics(app).switch_sent.lock().unwrap();
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

/// `TKOSC/HapticsLevel`, when the worn avatar has it and the level changed.
fn send_level(app: &AppHandle, level: f64) {
    let level = (level * 100.0).round() / 100.0;
    {
        let mut sent = haptics(app).level_sent.lock().unwrap();
        if sent.is_some_and(|(at, last)| last == level || at.elapsed() < LEVEL_EVERY) {
            return;
        }
        *sent = Some((Instant::now(), level));
    }
    let state = app.state::<AppState>();
    let has_parameter = state
        .avatar_details
        .lock()
        .unwrap()
        .as_ref()
        .is_some_and(|details| details.parameters.iter().any(|parameter| parameter.name == LEVEL_PARAMETER));
    if !has_parameter {
        return;
    }
    let command = OscCommand { address: format!("/avatar/parameters/{LEVEL_PARAMETER}"), args: vec![OscArg::Number(level)] };
    if let Err(err) = crate::osc::send(&state, &command) {
        tracing::warn!("Couldn't send {LEVEL_PARAMETER}: {err}");
    }
}

/// Which actuators touch drives, for the tiles: a toy or pattern control on one of them reads
/// Paused, on the share page too (without saying why).
fn emit_touched(app: &AppHandle) {
    let touched = crate::toys::touched(app);
    let mut sent = haptics(app).touched.lock().unwrap();
    if sent.as_ref() != Some(&touched) {
        let _ = app.emit("toys-state", serde_json::json!({ "touched": touched }));
        *sent = Some(touched);
    }
}

pub fn touched(app: &AppHandle) -> Vec<crate::toys::Touched> {
    crate::toys::touched(app)
}

fn emit_view(app: &AppHandle, now: bool) {
    let view = view(app);
    let mut sent = haptics(app).sent.lock().unwrap();
    let due = match &*sent {
        Some((at, last)) => *last != view && (now || at.elapsed() >= VIEW_EVERY),
        None => true
    };
    if due {
        let _ = app.emit("haptics-state", &view);
        *sent = Some((Instant::now(), view));
    }
}

fn start(app: &AppHandle) {
    {
        let mut task = haptics(app).task.lock().unwrap();
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
    // Toys already connected before haptics were turned on.
    devices_changed(app);
}

fn stop(app: &AppHandle) {
    if let Some(task) = haptics(app).task.lock().unwrap().take() {
        task.abort();
    }
    haptics(app).engine.lock().unwrap().clear();
    haptics(app).levels.lock().unwrap().clear();
    send_app_enabled(app, false);
    *haptics(app).switch_sent.lock().unwrap() = None;
    // Touch lets go of every toy.
    crate::toys::set_touch(app, Vec::new(), crate::toys::Priority { threshold: 1.0, grace: Duration::ZERO });
    emit_view(app, true);
    emit_touched(app);
}

/// Avatar haptics as an add-on (see `crate::addons`). It drives Intiface toys, so it needs them.
pub struct Addon;

impl crate::addons::Addon for Addon {
    fn manifest(&self) -> crate::addons::Manifest {
        crate::addons::Manifest {
            id: "haptics",
            name: "Avatar haptics",
            group: "toys",
            pitch: "Your toys respond when someone touches your SPS avatar, like OscGoesBrrr",
            platforms: &["windows", "linux"],
            badge: None,
            needs: &["intiface"],
            control_types: &[],
            automation_kinds: &[],
            pages: &["haptics"]
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
        if haptics(app).task.lock().unwrap().is_none() {
            return Status::new(State::Off, "Off");
        }
        let settings = store::get(app);
        let set_up = settings.outputs.iter().chain(settings.avatars.values().flat_map(|avatar| &avatar.outputs)).any(|o| !o.sources.is_empty());
        let zones = haptics(app).engine.lock().unwrap().zones().len();
        match (set_up, zones) {
            (_, _) if !settings.enabled => Status::new(State::Warn, "Turned off"),
            (false, _) => Status::new(State::Ok, "No toy follows your avatar yet"),
            (true, 0) => Status::new(State::Ok, "Waiting for avatar contacts"),
            (true, zones) => Status::new(State::Ok, format!("{zones} zone{} on this avatar", if zones == 1 { "" } else { "s" }))
        }
    }

    fn stop_everything(&self, _app: &AppHandle) -> crate::addons::StopFuture {
        // The toys themselves stop with Intiface's part.
        Box::pin(async { Ok(()) })
    }

    fn was_in_use(&self, _app: &AppHandle) -> bool {
        false
    }
}
