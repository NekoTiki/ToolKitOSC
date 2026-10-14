//! Control steps of automations, run here instead of in the webview: set, toggle, press, pulse,
//! hold and follow, as `useAutomationRunner.ts` did. Automations act as you, so profile locks and
//! limits don't apply; Stop everything does, and nothing new starts while it's on.
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Manager};

use super::engine::{AppRun, Phase};
use super::model::{Action, ControlRun};
use crate::state::AppState;

/// Follows send at most this often, like the board follows.
const FOLLOW_MIN_GAP: Duration = Duration::from_millis(50);

#[derive(Default)]
struct Follow {
    sent_at: Option<Instant>,
    /// A send is already waiting for the gap to pass; it takes the latest value.
    waiting: bool,
    value: f64
}

#[derive(Default)]
struct Runs {
    /// What each running hold changed, by automation and step, to put back on `stop`.
    held: HashMap<String, Value>,
    pulses: HashMap<String, JoinHandle<()>>,
    follows: HashMap<String, Follow>
}

static RUNS: Mutex<Option<Runs>> = Mutex::new(None);

fn with<T>(f: impl FnOnce(&mut Runs) -> T) -> T {
    f(RUNS.lock().unwrap().get_or_insert_with(Runs::default))
}

/// Holds and pulses belong to the old avatar's controls: forget them.
pub fn forget() {
    with(|runs| {
        runs.held.clear();
        for (_, pulse) in runs.pulses.drain() {
            pulse.abort();
        }
        runs.follows.clear();
    });
}

fn paused(app: &AppHandle) -> bool {
    super::is_paused(app)
}

fn send(app: &AppHandle, control: &Value, value: Option<Value>) {
    let kind = control.get("type").cloned().unwrap_or(Value::Null);
    let command = match value {
        Some(value) => json!({ "type": kind, "value": value }),
        None => json!({ "type": kind })
    };
    crate::controls::run(app, control, &command, None);
}

fn arg(app: &AppHandle, address: Option<&str>) -> Option<Value> {
    let state = app.state::<AppState>();
    let arg = state.messages.lock().unwrap().get(address?)?.args.first()?.clone();
    serde_json::to_value(arg).ok()
}

fn truthy(value: Option<Value>) -> bool {
    match value {
        Some(Value::Bool(on)) => on,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|n| n != 0.0),
        Some(Value::String(s)) => !s.is_empty(),
        _ => false
    }
}

/// What the control is at now, in the shape its command takes.
fn current(app: &AppHandle, control: &Value) -> Value {
    let text = |key: &str| control.get(key).and_then(Value::as_str);
    match text("type").unwrap_or_default() {
        "boolean" => json!(truthy(arg(app, text("inputAddress")))),
        "boolean-group" => {
            let first = control.pointer("/inputs/0/inputAddress").and_then(Value::as_str);
            json!(truthy(arg(app, first)))
        }
        "enum" | "step-enum" => {
            let value = arg(app, text("inputAddress")).and_then(|v| v.as_f64().or_else(|| v.as_bool().map(|b| b as u8 as f64)));
            let index = control
                .get("options")
                .and_then(Value::as_array)
                .and_then(|options| options.iter().position(|option| option.get("value").and_then(Value::as_f64) == value));
            json!(index.unwrap_or(0))
        }
        "slider" => json!(arg(app, text("inputAddress")).and_then(|v| v.as_f64()).unwrap_or(0.0)),
        "intiface-toy" => json!(crate::controls::toy_level(text("id").unwrap_or_default())),
        "intiface-pattern" => json!(crate::toys::patterns::value(app, text("id").unwrap_or_default())),
        _ => json!(0)
    }
}

fn follow(app: &AppHandle, key: &str, control: &Value, value: f64) {
    let wait = with(|runs| {
        let state = runs.follows.entry(key.to_owned()).or_default();
        state.value = value;
        if state.waiting {
            return None;
        }
        let wait = state.sent_at.map(|at| FOLLOW_MIN_GAP.saturating_sub(at.elapsed())).unwrap_or_default();
        if wait.is_zero() {
            state.sent_at = Some(Instant::now());
        } else {
            state.waiting = true;
        }
        Some(wait)
    });

    match wait {
        Some(wait) if wait.is_zero() => send(app, control, Some(json!(value))),
        Some(wait) => {
            let (app, key, control) = (app.clone(), key.to_owned(), control.clone());
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(wait).await;
                let value = with(|runs| {
                    let state = runs.follows.get_mut(&key)?;
                    state.waiting = false;
                    state.sent_at = Some(Instant::now());
                    Some(state.value)
                });
                if let Some(value) = value.filter(|_| !paused(&app)) {
                    send(&app, &control, Some(json!(value)));
                }
            });
        }
        None => {}
    }
}

/// Runs one Control step. Returns false for any other action, which still goes to the webview.
pub fn run(app: &AppHandle, run: &AppRun) -> bool {
    let Action::Control { group_id, control_id, run: step, .. } = &run.action else {
        return false;
    };
    let key = format!("{}:{}", run.automation_id, run.step);

    // Nothing new starts while Stop everything is on; holds may still let go.
    if paused(app) && run.phase != Phase::Stop {
        return true;
    }
    let Some(control) = crate::server::find_control(app, group_id, control_id) else {
        return true;
    };
    if control.get("unavailable").and_then(Value::as_bool).unwrap_or(false) {
        return true;
    }
    let kind = control.get("type").and_then(Value::as_str).unwrap_or_default();

    match (&run.phase, step) {
        (Phase::Stop, _) => {
            let Some(before) = with(|runs| runs.held.remove(&key)) else {
                return true;
            };
            // Stop everything already switched toys and patterns off; don't start them again.
            if paused(app) && (kind == "intiface-toy" || kind == "intiface-pattern") {
                return true;
            }
            send(app, &control, Some(before));
        }
        (Phase::Follow { value }, ControlRun::Follow { min, max }) => follow(app, &key, &control, min + *value as f64 * (max - min)),
        (Phase::Follow { .. }, _) => {}
        (_, ControlRun::Set { value }) => send(app, &control, Some(value.clone())),
        (_, ControlRun::Toggle) => {
            let on = current(app, &control).as_bool().unwrap_or(false);
            send(app, &control, Some(json!(!on)));
        }
        (_, ControlRun::Press) => send(app, &control, None),
        (_, ControlRun::Pulse { value, ms }) => {
            if let Some(old) = with(|runs| runs.pulses.remove(&key)) {
                old.abort();
            }
            send(app, &control, Some(json!(value)));
            let (task_app, task_key, task_control, ms) = (app.clone(), key.clone(), control.clone(), *ms);
            let task = tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_millis(ms as u64)).await;
                with(|runs| runs.pulses.remove(&task_key));
                if !paused(&task_app) {
                    send(&task_app, &task_control, Some(json!(0)));
                }
            });
            with(|runs| runs.pulses.insert(key, task));
        }
        // A hold's value goes out on start (and on a test run); `stop` is handled above.
        (_, ControlRun::Hold { value }) => {
            let before = current(app, &control);
            with(|runs| {
                runs.held.entry(key).or_insert(before);
            });
            send(app, &control, Some(value.clone()));
        }
        (_, ControlRun::Follow { .. }) => {}
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truthy_matches_javascript() {
        assert!(truthy(Some(json!(true))));
        assert!(truthy(Some(json!(1))));
        assert!(!truthy(Some(json!(0))));
        assert!(!truthy(None));
    }
}
