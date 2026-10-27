//! Runs a command on a control: the one place that turns "this viewer set the slider to 0.4" into
//! OSC, toy levels, patterns or a shock. Viewer commands (after `server::commands` has checked
//! them), your own clicks in the app and automations all end up here. It never applies limits:
//! that's the caller's job, and your own clicks and automations act as you.
//!
//! Controls and commands are JSON as the webview stores them (see `types/controls.ts`).
use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};

use crate::server::limits::{OFF_ID, VIEWER_LIVE_ID, VIEWER_PATTERN_ID};
use crate::state::AppState;
use crate::toys::{self, patterns, ActuatorRef};
use crate::types::{OscArg, OscCommand};

/// What happened, for the activity log and `command-accepted`.
#[derive(Debug, Default)]
pub struct Outcome {
    /// What a shock rolled (intensity, duration, shocker names). `None` for anything else, or a
    /// shock that didn't go out.
    pub shock: Option<Value>,
    /// The run number of a pattern it started (`pattern-ended` carries it).
    pub pattern_run: Option<u64>,
    /// It only kept something going (a bHaptics hold), or was dropped during an OWO control's
    /// cooldown: nothing to log, announce or show.
    pub live: bool
}

fn send(app: &AppHandle, address: &str, arg: Value) {
    let Ok(arg) = serde_json::from_value::<OscArg>(arg) else {
        return;
    };
    let state = app.state::<AppState>();
    if let Err(err) = crate::osc::send(&state, &OscCommand { address: address.to_owned(), args: vec![arg] }) {
        tracing::warn!("Couldn't send {address}: {err}");
    }
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn index(value: Option<&Value>) -> Option<usize> {
    let value = value?.as_f64()?;
    (value.fract() == 0.0 && value >= 0.0).then_some(value as usize)
}

fn actuators(control: &Value) -> Vec<ActuatorRef> {
    control.get("actuators").cloned().and_then(|a| serde_json::from_value(a).ok()).unwrap_or_default()
}

// shortcut: one fixed hold for every Button, make it a setting if an avatar needs longer.
const PRESS_HOLD: std::time::Duration = std::time::Duration::from_millis(250);

/// Sets a Button's parameter to `on`, long enough for the avatar to see it, then back.
fn press(app: &AppHandle, address: &str, on: bool) {
    send(app, address, json!(on));
    let (app, address) = (app.clone(), address.to_owned());
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(PRESS_HOLD).await;
        send(&app, &address, json!(!on));
    });
}

/// Each toy control's level as last set, for automations that hold and put it back.
static TOY_LEVELS: Mutex<Option<HashMap<String, f64>>> = Mutex::new(None);

pub fn toy_level(control_id: &str) -> f64 {
    TOY_LEVELS.lock().unwrap().as_ref().and_then(|levels| levels.get(control_id).copied()).unwrap_or(0.0)
}

/// Shows a toy control's level on its tile, and on the share page (the webview forwards it).
fn report_toy(app: &AppHandle, control_id: &str, level: f64) {
    TOY_LEVELS.lock().unwrap().get_or_insert_with(HashMap::new).insert(control_id.to_owned(), level);
    let _ = app.emit("toy-value", json!({ "controlId": control_id, "value": level }));
}

/// Runs `command` on `control`. `viewer_name` is who it's for when that may be shown: the OpenShock
/// log and a viewer's pattern on the share page say it.
pub fn run(app: &AppHandle, control: &Value, command: &Value, viewer_name: Option<String>) -> Outcome {
    let mut outcome = Outcome::default();
    let (Some(kind), Some(control_kind)) = (text(command, "type"), text(control, "type")) else {
        return outcome;
    };
    // Commands for a control whose add-on is off do nothing, whoever sends them.
    if kind != control_kind || !crate::addons::control_type_live(app, kind) {
        return outcome;
    }
    let value = command.get("value");
    let control_id = text(control, "id").unwrap_or_default();

    match kind {
        "button" => {
            if let Some(address) = text(control, "inputAddress") {
                press(app, address, !control.get("reverse").and_then(Value::as_bool).unwrap_or(false));
            }
        }
        "boolean" => {
            if let (Some(address), Some(value)) = (text(control, "inputAddress"), value) {
                send(app, address, value.clone());
            }
        }
        "boolean-group" => {
            for input in control.get("inputs").and_then(Value::as_array).into_iter().flatten() {
                if let (Some(address), Some(value)) = (text(input, "inputAddress"), value) {
                    send(app, address, value.clone());
                }
            }
        }
        "boolean-enum" => {
            let input = index(value).and_then(|at| control.get("inputs")?.as_array()?.get(at));
            if let Some(input) = input {
                for (side, on) in [("true", true), ("false", false)] {
                    let addresses = input.pointer(&format!("/inputAddress/{side}")).and_then(Value::as_array);
                    for address in addresses.into_iter().flatten().filter_map(Value::as_str) {
                        send(app, address, json!(on));
                    }
                }
            }
        }
        "enum" | "step-enum" => {
            let option = index(value).and_then(|at| control.get("options")?.as_array()?.get(at));
            if let (Some(address), Some(option)) = (text(control, "inputAddress"), option.and_then(|o| o.get("value"))) {
                send(app, address, option.clone());
            }
        }
        "slider" => {
            if let (Some(address), Some(value)) = (text(control, "inputAddress"), value) {
                send(app, address, value.clone());
            }
        }
        "preset" => run_preset(app, text(control, "presetId").unwrap_or_default()),
        "open-shock-shocker" => outcome.shock = crate::openshock::fire_control(app, control, viewer_name),
        // Toys only move while Intiface is connected.
        "intiface-toy" if crate::intiface::connected(app) => {
            let level = value.and_then(Value::as_f64).unwrap_or(0.0);
            let actuators = actuators(control);
            report_toy(app, control_id, level);
            // Driving these directly wins over any pattern on them. Released first, or a pattern
            // tick could land after the new level.
            patterns::release(app, &actuators, None);
            toys::set(app, &actuators, level);
        }
        "intiface-pattern" if crate::intiface::connected(app) => {
            outcome.pattern_run = run_pattern(app, control, command, control_id, viewer_name);
        }
        "bhaptics" => outcome.live = crate::bhaptics::control::run(app, control, command),
        "owo" => outcome.live = crate::owo::control::run(app, control, command),
        _ => {}
    }

    outcome
}

fn run_pattern(app: &AppHandle, control: &Value, command: &Value, control_id: &str, viewer_name: Option<String>) -> Option<u64> {
    let actuators = actuators(control);
    let id = text(command, "value")?;

    match id {
        // A viewer recording: the live level goes straight to the toy.
        VIEWER_LIVE_ID => {
            patterns::stop(app, control_id);
            if patterns::value(app, control_id) != OFF_ID {
                patterns::report(app, control_id, OFF_ID);
            }
            patterns::release(app, &actuators, None);
            toys::set(app, &actuators, command.get("level").and_then(Value::as_f64).unwrap_or(0.0));
            None
        }
        VIEWER_PATTERN_ID => {
            let samples: Vec<f64> = command.get("samples").cloned().and_then(|s| serde_json::from_value(s).ok()).unwrap_or_default();
            let stop_after = control.pointer("/viewerPatterns/stopsAfterMs").and_then(Value::as_f64).map(|ms| ms.max(0.0) as u64);
            let source = patterns::Source { id: VIEWER_PATTERN_ID.to_owned(), samples: Some(samples) };
            patterns::play(app, control_id, source, actuators, stop_after, viewer_name)
        }
        _ => patterns::play(app, control_id, patterns::Source { id: id.to_owned(), samples: None }, actuators, None, None)
    }
}

/// A preset of the avatar you're wearing: every parameter it saved, sent again.
fn run_preset(app: &AppHandle, preset_id: &str) {
    let state = app.state::<AppState>();
    let Some(avatar_id) = state.avatar_details.lock().unwrap().as_ref().map(|d| d.id.clone()) else {
        return;
    };
    let Ok(store) = crate::presets::load(app, &avatar_id) else {
        return;
    };
    let Some(preset) = store.presets.iter().find(|preset| preset.id == preset_id) else {
        return;
    };
    for parameter in &preset.parameters {
        if let Err(err) = crate::osc::send(&state, &OscCommand { address: parameter.address.clone(), args: vec![parameter.value.clone()] }) {
            tracing::warn!("Couldn't send {}: {err}", parameter.address);
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlRunResult {
    /// What a shock rolled, when one went out.
    shock: Option<serde_json::Value>,
    /// The run number of a pattern it started.
    pattern_run: Option<u64>
}

/// Runs a command on a control as you: your own clicks, previews and automations. No limits apply.
#[tauri::command]
pub fn control_run(
    app: AppHandle,
    control: serde_json::Value,
    command: serde_json::Value,
    viewer_name: Option<String>
) -> ControlRunResult {
    let outcome = run(&app, &control, &command, viewer_name);
    ControlRunResult { shock: outcome.shock, pattern_run: outcome.pattern_run }
}
