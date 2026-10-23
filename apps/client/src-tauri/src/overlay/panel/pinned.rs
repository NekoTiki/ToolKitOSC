//! The panel's Pinned page: the controls pinned on the avatar you're wearing, as tiles. Their state
//! comes from the controls in `setup` (with the active profile's locks) and the live
//! OSC values; a press runs through `controls::run` as you, like a click in the app.
//!
//! A pressed tile shows its new state right away and keeps it until VRChat reports a value for
//! that parameter (see `Panel::expected`): VRChat may not echo what it receives over OSC.
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use super::{overlay, store};
use crate::server::limits::OFF_ID;
use crate::state::AppState;
use crate::types::{OscArg, OscMessage};
use crate::wrist::protocol::{Tile, TileKind};

/// Shown when nothing is pinned on this avatar yet.
const FIRST: usize = 4;

/// The parameters a control sets.
fn addresses(control: &Value) -> Vec<String> {
    let inputs = || control.get("inputs").and_then(Value::as_array).into_iter().flatten();
    match text(control, "type") {
        Some("boolean-group") => inputs().filter_map(|input| text(input, "inputAddress")).map(str::to_owned).collect(),
        Some("boolean-enum") => inputs()
            .flat_map(|input| ["true", "false"].map(|side| input.pointer(&format!("/inputAddress/{side}")).and_then(Value::as_array).cloned().unwrap_or_default()))
            .flatten()
            .filter_map(|address| address.as_str().map(str::to_owned))
            .collect(),
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
    let now = snapshot(app, control);
    let mut panel = overlay(app).panel.lock().unwrap();
    let (kind, before) = panel.expected.get(control_id)?;
    if *before == now {
        return Some(kind.clone());
    }
    panel.expected.remove(control_id);
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

/// A boolean-enum's input that's on: all its `true` parameters are.
fn current_input(app: &AppHandle, control: &Value) -> Option<usize> {
    control.get("inputs")?.as_array()?.iter().position(|input| {
        let on = input.pointer("/inputAddress/true").and_then(Value::as_array).cloned().unwrap_or_default();
        !on.is_empty() && on.iter().all(|address| address.as_str().and_then(|address| value(app, address)).is_some_and(|v| v >= 0.5))
    })
}

/// What a press steps through, by name, and which it's on now: an enum's options, or a
/// boolean-enum's inputs.
fn choices(app: &AppHandle, control: &Value) -> (Vec<String>, Option<usize>) {
    let (list, current) = match text(control, "type") {
        Some("boolean-enum") => ("inputs", current_input(app, control)),
        _ => ("options", current_option(app, control))
    };
    let names = control.get(list).and_then(Value::as_array).into_iter().flatten().map(|item| text(item, "name").unwrap_or_default().to_owned()).collect();
    (names, current)
}

/// A pattern control's choices: Off, then its patterns, by id and name.
fn patterns(control: &Value) -> Vec<(String, String)> {
    let allowed = control.get("allowedPatterns").and_then(Value::as_array).into_iter().flatten();
    std::iter::once((OFF_ID.to_owned(), "Off".to_owned()))
        .chain(allowed.map(|pattern| (text(pattern, "id").unwrap_or_default().to_owned(), text(pattern, "name").unwrap_or_default().to_owned())))
        .collect()
}

/// A bHaptics control's built-in effects the panel offers after Buzz.
fn effects(control: &Value) -> Vec<String> {
    control.get("effects").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).filter(|effect| crate::bhaptics::effects::exists(effect)).map(str::to_owned).collect()
}

/// "heartbeat" -> "Heartbeat", as the app's own labels.
fn label(id: &str) -> String {
    let mut chars = id.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

fn tile(app: &AppHandle, group_id: &str, control: &Value) -> Option<Tile> {
    let kind = match text(control, "type")? {
        "boolean" | "boolean-group" => TileKind::Toggle { on: is_on(app, control) },
        "enum" | "step-enum" | "boolean-enum" => {
            let (names, current) = choices(app, control);
            TileKind::Choice { option: current.and_then(|at| names.get(at).cloned()) }
        }
        "intiface-pattern" => {
            let patterns = patterns(control);
            let playing = crate::toys::patterns::value(app, text(control, "id")?);
            TileKind::Menu { current: patterns.iter().position(|(id, _)| *id == playing), options: patterns.into_iter().map(|(_, name)| name).collect() }
        }
        "bhaptics" => TileKind::Menu { options: std::iter::once("Buzz".to_owned()).chain(effects(control).iter().map(|effect| label(effect))).collect(), current: None },
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

/// The controls with these ids, in that order, each with its group's id;
/// or the first few when none of them is there. True for the second.
fn pick(groups: &[Value], ids: &[String]) -> (Vec<(String, Value)>, bool) {
    let all = || {
        groups.iter().flat_map(|group| {
            let group_id = text(group, "id").unwrap_or_default();
            group.get("controls").and_then(Value::as_array).into_iter().flatten().map(move |control| (group_id, control))
        })
    };
    let chosen: Vec<_> = ids.iter().filter_map(|id| all().find(|(_, control)| text(control, "id") == Some(id))).collect();
    let nothing_pinned = chosen.is_empty();
    let shown = if nothing_pinned { all().take(FIRST).collect() } else { chosen };
    (shown.into_iter().map(|(group_id, control)| (group_id.to_owned(), control.clone())).collect(), nothing_pinned)
}

/// The tiles for the controls pinned with `ids`, and whether they're the first few because nothing
/// is pinned.
pub fn tiles(app: &AppHandle, ids: &[String]) -> (Vec<Tile>, bool) {
    // Building a tile reads the live parameters, so it happens after the server's lock is let go.
    let (shown, nothing_pinned) = crate::setup::with_groups(app, |groups| pick(groups, ids));
    let tiles: Vec<Tile> = shown.iter().filter_map(|(group_id, control)| tile(app, group_id, control)).collect();
    // What a press showed only matters while its tile is on the page.
    overlay(app).panel.lock().unwrap().expected.retain(|id, _| tiles.iter().any(|tile| &tile.control_id == id));
    (tiles, nothing_pinned)
}

/// A tile pressed on the panel: what a click on it in the app would do. `pick` is what was picked
/// from a `TileKind::Menu`.
pub fn press(app: &AppHandle, group_id: &str, control_id: &str, level: Option<f64>, pick: Option<usize>) {
    let Some(control) = crate::setup::find_control(app, group_id, control_id) else {
        return;
    };
    let Some(tile) = tile(app, group_id, &control) else {
        return;
    };
    if tile.locked {
        return;
    }
    let kind = text(&control, "type").unwrap_or_default();
    let (choices, current) = choices(app, &control);
    let next_choice = current.map_or(0, |at| (at + 1) % choices.len().max(1));
    if let (TileKind::Menu { .. }, Some(pick)) = (&tile.kind, pick) {
        return pick_from_menu(app, group_id, control_id, &control, pick);
    }
    let shown = match &tile.kind {
        TileKind::Toggle { on } => Some(TileKind::Toggle { on: !on }),
        TileKind::Choice { .. } => Some(TileKind::Choice { option: choices.get(next_choice).cloned() }),
        TileKind::Level { .. } if kind == "slider" => Some(TileKind::Level { value: level.unwrap_or(0.0).clamp(0.0, 1.0) }),
        _ => None
    };
    let value = match &tile.kind {
        TileKind::Toggle { on } => match kind {
            // The command carries the parameter's value, which `reverse` flips.
            "boolean" => json!(!on != control.get("reverse").and_then(Value::as_bool).unwrap_or(false)),
            _ => json!(!on)
        },
        TileKind::Choice { .. } if choices.is_empty() => return,
        TileKind::Choice { .. } => json!(next_choice),
        TileKind::Level { .. } => json!(level.unwrap_or(0.0).clamp(0.0, 1.0)),
        TileKind::Press | TileKind::Hold => Value::Null,
        // Opening the list happens on the panel; only a pick comes here.
        TileKind::Menu { .. } | TileKind::AppOnly => return
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
        let now = snapshot(app, &control);
        let mut panel = overlay(app).panel.lock().unwrap();
        let before = panel.expected.remove(control_id).map_or(now, |(_, before)| before);
        panel.expected.insert(control_id.to_owned(), (shown, before));
    }
    crate::controls::run(app, &control, &command, None);
}

/// Something picked from a `TileKind::Menu`: a pattern to play (or Off), or a bHaptics Buzz or
/// effect.
fn pick_from_menu(app: &AppHandle, group_id: &str, control_id: &str, control: &Value, pick: usize) {
    let kind = text(control, "type").unwrap_or_default();
    let command = |value: Value| json!({ "type": kind, "groupId": group_id, "controlId": control_id, "value": value });
    match kind {
        "intiface-pattern" => {
            if let Some((id, _)) = patterns(control).get(pick) {
                crate::controls::run(app, control, &command(json!(id)), None);
            }
        }
        // Buzz: a short tap on every motor of each of its positions, at the control's highest.
        "bhaptics" if pick == 0 => {
            let positions: Vec<crate::bhaptics::model::Position> = control.get("positions").cloned().and_then(|p| serde_json::from_value(p).ok()).unwrap_or_default();
            for position in positions {
                let mut tap = command(json!("tap"));
                tap["position"] = json!(position);
                tap["motors"] = json!((0..position.motors()).collect::<Vec<_>>());
                crate::controls::run(app, control, &tap, None);
            }
        }
        "bhaptics" => {
            if let Some(effect) = effects(control).get(pick - 1) {
                let mut play = command(json!("effect"));
                play["effect"] = json!(effect);
                crate::controls::run(app, control, &play, None);
            }
        }
        _ => {}
    }
}

/// Pins or unpins a control of the avatar you're wearing.
pub fn set_pinned(app: &AppHandle, control_id: &str, pin: bool) -> Result<(), String> {
    let avatar = avatar_id(app).ok_or("Wear the avatar in VRChat first")?;
    store::with(app, |settings| {
        let list = settings.pinned.entry(avatar).or_default();
        let had = list.iter().any(|id| id == control_id);
        if pin == had {
            return (Ok(()), false);
        }
        if pin {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn groups() -> Vec<Value> {
        let controls = |ids: &[&str]| ids.iter().map(|id| json!({ "id": id, "name": id })).collect::<Vec<_>>();
        vec![json!({ "id": "a", "controls": controls(&["ears", "tail", "lush"]) }), json!({ "id": "b", "controls": controls(&["shock", "wings", "hat", "boots"]) })]
    }

    fn ids(picked: &[(String, Value)]) -> Vec<(&str, &str)> {
        picked.iter().map(|(group, control)| (group.as_str(), text(control, "id").unwrap())).collect()
    }

    #[test]
    fn pinned_controls_come_in_pin_order_and_skip_missing_ones() {
        let (picked, nothing_pinned) = pick(&groups(), &["shock".into(), "gone".into(), "ears".into()]);
        assert!(!nothing_pinned);
        assert_eq!(ids(&picked), [("b", "shock"), ("a", "ears")]);
    }

    #[test]
    fn any_number_can_be_pinned() {
        let all: Vec<String> = ["ears", "tail", "lush", "shock", "wings", "hat", "boots"].map(String::from).to_vec();
        assert_eq!(pick(&groups(), &all).0.len(), 7);
    }

    #[test]
    fn nothing_pinned_shows_the_first_four() {
        let (picked, nothing_pinned) = pick(&groups(), &["gone".into()]);
        assert!(nothing_pinned);
        assert_eq!(ids(&picked), [("a", "ears"), ("a", "tail"), ("a", "lush"), ("b", "shock")]);
    }
}
