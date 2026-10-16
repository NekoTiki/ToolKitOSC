//! Avatar haptics: the SPS sockets, plugs and touch zones on the avatar you wear drive your toys, the
//! job OscGoesBrrr does. Contacts arrive live from `osc::udp`, `engine` turns them into one level
//! per actuator every tick, and `haptics.json` (see `store`) says which toy follows what: a global
//! setup, and per-avatar overrides.
mod depth;
mod engine;
pub mod model;
mod store;
mod zones;
pub mod commands;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::{interval, MissedTickBehavior};

use crate::state::AppState;
use crate::types::{OscArg, OscMessage};
use engine::{Engine, OutputLevel, ZoneView};
use model::HapticsSettings;

const TICK: Duration = Duration::from_millis(50);
/// `haptics-state` goes out at most this often.
const VIEW_EVERY: Duration = Duration::from_millis(100);

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
    touched: Mutex<Option<Vec<crate::toys::Touched>>>
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
    pub outputs: Vec<OutputLevel>
}

fn avatar_id(app: &AppHandle) -> Option<String> {
    app.state::<AppState>().avatar_details.lock().unwrap().as_ref().map(|details| details.id.clone())
}

pub fn view(app: &AppHandle) -> View {
    View {
        avatar_id: avatar_id(app),
        zones: haptics(app).engine.lock().unwrap().zones(),
        outputs: haptics(app).levels.lock().unwrap().clone()
    }
}

/// A live avatar parameter from VRChat. Values read when an avatar loads never get here, so they
/// never drive a toy.
pub fn on_parameter(app: &AppHandle, message: &OscMessage) {
    let Some(name) = message.address.strip_prefix("/avatar/parameters/") else {
        return;
    };
    if !(name.starts_with("OGB/") || name.starts_with("VFH/") || name.starts_with("TPS_Internal/")) {
        return;
    }
    let value = match message.args.first() {
        Some(OscArg::Number(value)) => zones::Value::Number(*value),
        Some(OscArg::Bool(on)) => zones::Value::Bool(*on),
        _ => return
    };
    if haptics(app).task.lock().unwrap().is_some() {
        haptics(app).engine.lock().unwrap().on_parameter(name, value);
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
}

fn tick(app: &AppHandle) {
    let settings = store::get(app);
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
    crate::toys::set_touch(app, touches, priority);

    *haptics(app).levels.lock().unwrap() = levels;
    emit_view(app, false);
    emit_touched(app);
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
