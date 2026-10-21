//! The panel's Pinned page: up to 6 controls of the avatar you're wearing, as tiles. Their state
//! comes from the controls the webview last pushed (with the active profile's locks) and the live
//! OSC values; a press runs through `controls::run` as you, like a click in the app.
//!
//! A pressed tile shows its new state right away and keeps it until VRChat reports a value for
//! that parameter (see `EXPECTED`): VRChat may not echo what it receives over OSC.
use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use super::store;
use crate::state::AppState;
use crate::types::{OscArg, OscMessage};
use crate::wrist::protocol::{Tile, TileKind};

pub const MAX_PINNED: usize = 6;
/// Shown when nothing is pinned on this avatar yet.
const FIRST: usize = 4;

/// Control id -> what a press made it, and its parameters' values from before the press. Shown
/// until one of those values changes.
static EXPECTED: Mutex<Option<HashMap<String, (TileKind, Vec<Option<OscMessage>>)>>> = Mutex::new(None);

/// The parameters a control sets.
fn addresses(control: &Value) -> Vec<String> {
    match text(control, "type") {
        Some("boolean-group") => control.get("inputs").and_then(Value::as_array).into_iter().flatten().filter_map(|input| text(input, "inputAddress")).map(str::to_owned).collect(),
        _ => text(control, "inputAddress").map(str::to_owned).into_iter().collect()
    }
}

fn snapshot(app: &AppHandle, control: &Value) -> Vec<Option<OscMessage>> {
    let state = app.state::<AppState>();
    let messages = state.messages.lock().unwrap();
    addresses(control).iter().map(|address| messages.get(address).cloned()).collect()
}

/// What the last press made it, while VRChat hasn't reported a value since.
fn expected(app: &AppHandle, control_id: &str, control: &Value) -> Option<TileKind> {
    let mut guard = EXPECTED.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    let (kind, before) = map.get(control_id)?;
    if *before == snapshot(app, control) {
        return Some(kind.clone());
    }
    map.remove(control_id);
    None
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

pub fn avatar_id(app: &AppHandle) -> Option<String> {
    app.state::<AppState>().avatar_details.lock().unwrap().as_ref().map(|details| details.id.clone())
}

/// A parameter's live value as a number.
fn value(app: &AppHandle, address: &str) -> Option<f64> {
    let state = app.state::<AppState>();
    let messages = state.messages.lock().unwrap();
    match messages.get(address)?.args.first()? {
        OscArg::Bool(on) => Some(if *on { 1.0 } else { 0.0 }),
        OscArg::Number(n) => Some(*n),
        OscArg::Str(_) => None
    }
}

fn is_on(app: &AppHandle, control: &Value) -> bool {
    match text(control, "type") {
        Some("boolean") => {
            let on = text(control, "inputAddress").and_then(|address| value(app, address)).is_some_and(|v| v >= 0.5);
            on != control.get("reverse").and_then(Value::as_bool).unwrap_or(false)
        }
        Some("boolean-group") => {
            let inputs = control.get("inputs").and_then(Value::as_array).cloned().unwrap_or_default();
            !inputs.is_empty() && inputs.iter().all(|input| text(input, "inputAddress").and_then(|address| value(app, address)).is_some_and(|v| v >= 0.5))
        }
        _ => false
    }
}

/// The option whose value the parameter has now.
fn current_option(app: &AppHandle, control: &Value) -> Option<usize> {
    let current = value(app, text(control, "inputAddress")?)?;
    control.get("options")?.as_array()?.iter().position(|option| option.get("value").and_then(Value::as_f64) == Some(current))
}

fn tile(app: &AppHandle, group_id: &str, control: &Value) -> Option<Tile> {
    let kind = match text(control, "type")? {
        "boolean" | "boolean-group" => TileKind::Toggle { on: is_on(app, control) },
        "enum" | "step-enum" => {
            let option = current_option(app, control).and_then(|at| control.pointer(&format!("/options/{at}/name"))?.as_str().map(str::to_owned));
            TileKind::Choice { option }
        }
        "slider" => TileKind::Level { value: text(control, "inputAddress").and_then(|address| value(app, address)).unwrap_or(0.0) },
        "intiface-toy" => TileKind::Level { value: crate::controls::toy_level(text(control, "id")?) },
        "preset" => TileKind::Press,
        "open-shock-shocker" | "owo" => TileKind::Hold,
        _ => TileKind::AppOnly
    };
    let kind = expected(app, text(control, "id")?, control).unwrap_or(kind);
    let flag = |key: &str| control.get(key).and_then(Value::as_bool).unwrap_or(false);
    let live = crate::addons::control_type_live(app, text(control, "type")?);
    Some(Tile {
        group_id: group_id.to_owned(),
        control_id: text(control, "id")?.to_owned(),
        name: text(control, "name").unwrap_or_default().to_owned(),
        kind,
        locked: flag("locked") || flag("unavailable") || !live
    })
}

/// Every control of the avatar you're wearing, with its group's id, in the app's order.
fn controls(app: &AppHandle) -> Vec<(String, Value)> {
    crate::server::control_groups(app)
        .iter()
        .flat_map(|group| {
            let group_id = text(group, "id").unwrap_or_default().to_owned();
            group.get("controls").and_then(Value::as_array).cloned().unwrap_or_default().into_iter().map(move |control| (group_id.clone(), control))
        })
        .collect()
}

/// The pinned tiles, and whether they're the first few because nothing is pinned.
pub fn pinned(app: &AppHandle) -> (Vec<Tile>, bool) {
    let all = controls(app);
    let ids = avatar_id(app).and_then(|avatar| store::get(app).pinned.get(&avatar).cloned()).unwrap_or_default();
    let chosen: Vec<&(String, Value)> = ids.iter().filter_map(|id| all.iter().find(|(_, control)| text(control, "id") == Some(id))).take(MAX_PINNED).collect();
    let nothing_pinned = chosen.is_empty();
    let shown: Vec<&(String, Value)> = if nothing_pinned { all.iter().take(FIRST).collect() } else { chosen };
    (shown.into_iter().filter_map(|(group_id, control)| tile(app, group_id, control)).collect(), nothing_pinned)
}

/// A tile pressed on the panel: what a click on it in the app would do.
pub fn press(app: &AppHandle, group_id: &str, control_id: &str, level: Option<f64>) {
    let Some(control) = crate::server::find_control(app, group_id, control_id) else {
        return;
    };
    let Some(tile) = tile(app, group_id, &control) else {
        return;
    };
    if tile.locked {
        return;
    }
    let kind = text(&control, "type").unwrap_or_default();
    let options = control.get("options").and_then(Value::as_array).cloned().unwrap_or_default();
    let next_option = current_option(app, &control).map_or(0, |at| (at + 1) % options.len().max(1));
    let shown = match &tile.kind {
        TileKind::Toggle { on } => Some(TileKind::Toggle { on: !on }),
        TileKind::Choice { .. } => Some(TileKind::Choice { option: options.get(next_option).and_then(|o| text(o, "name")).map(str::to_owned) }),
        TileKind::Level { .. } if kind == "slider" => Some(TileKind::Level { value: level.unwrap_or(0.0).clamp(0.0, 1.0) }),
        _ => None
    };
    let value = match &tile.kind {
        TileKind::Toggle { on } => match kind {
            // The command carries the parameter's value, which `reverse` flips.
            "boolean" => json!(!on != control.get("reverse").and_then(Value::as_bool).unwrap_or(false)),
            _ => json!(!on)
        },
        TileKind::Choice { .. } if options.is_empty() => return,
        TileKind::Choice { .. } => json!(next_option),
        TileKind::Level { .. } => json!(level.unwrap_or(0.0).clamp(0.0, 1.0)),
        TileKind::Press | TileKind::Hold => Value::Null,
        TileKind::AppOnly => return
    };
    let mut command = json!({ "type": kind, "groupId": group_id, "controlId": control_id });
    if !value.is_null() {
        command["value"] = value;
    }
    // The OWO Skin plays the control's first sensation on all its muscles.
    if kind == "owo" {
        command["value"] = control.pointer("/sensations/0").cloned().unwrap_or(Value::Null);
        command["muscles"] = control.get("muscles").cloned().unwrap_or(json!([]));
    }
    if let Some(shown) = shown {
        // The values from before, so the press shows until VRChat reports the parameter. A press
        // on a tile already showing what it expects keeps the original values.
        let before = EXPECTED.lock().unwrap().get_or_insert_with(HashMap::new).remove(control_id).map(|(_, before)| before);
        let before = before.unwrap_or_else(|| snapshot(app, &control));
        EXPECTED.lock().unwrap().get_or_insert_with(HashMap::new).insert(control_id.to_owned(), (shown, before));
    }
    crate::controls::run(app, &control, &command, None);
}

/// Pins or unpins a control of the avatar you're wearing. At most `MAX_PINNED`.
pub fn set_pinned(app: &AppHandle, control_id: &str, pin: bool) -> Result<(), String> {
    let avatar = avatar_id(app).ok_or("Wear the avatar in VRChat first")?;
    store::with(app, |settings| {
        let list = settings.pinned.entry(avatar).or_default();
        let had = list.iter().any(|id| id == control_id);
        if pin == had {
            return (Ok(()), false);
        }
        if pin {
            if list.len() >= MAX_PINNED {
                return (Err(format!("Up to {MAX_PINNED} controls fit on your wrist. Unpin one first.")), false);
            }
            list.push(control_id.to_owned());
        } else {
            list.retain(|id| id != control_id);
        }
        (Ok(()), true)
    })
}

/// The controls pinned on the avatar you're wearing.
pub fn pinned_ids(app: &AppHandle) -> Vec<String> {
    avatar_id(app).and_then(|avatar| store::get(app).pinned.get(&avatar).cloned()).unwrap_or_default()
}
