//! Pattern playback: a pattern is a level over time, sampled every 50 ms and sent through the
//! arbiter, on a tokio timer so a hidden window doesn't slow it. The maths is the same as the
//! webview's `useIntifacePatterns.ts` always had, so patterns feel the same.
//!
//! Runners are keyed by control id (the Patterns page and Activity previews use their own ids).
//! Only one thing drives an actuator at a time: starting a pattern stops any other runner on the
//! same actuators, and that runner's control reads "off" again.
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::{interval, Instant, MissedTickBehavior};

use super::saved;
use super::ActuatorRef;
use crate::state::AppState;

pub const SAMPLE_MS: f64 = 50.0;
pub const OFF_ID: &str = "off";
const TICK: Duration = Duration::from_millis(50);

/// What to play: a built-in or saved pattern by id, or a recording the caller holds (a viewer's
/// own pattern, a take not saved yet).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    /// Reported as the control's pattern value.
    pub id: String,
    /// When set, these are played instead of looking `id` up.
    #[serde(default)]
    pub samples: Option<Vec<f64>>
}

enum Wave {
    Builtin(fn(f64) -> f64),
    /// Intensities 0–100, one per 50 ms, looped.
    Samples(Vec<f64>)
}

impl Wave {
    fn value(&self, t: f64) -> f64 {
        match self {
            Wave::Builtin(value) => value(t),
            Wave::Samples(samples) if samples.is_empty() => 0.0,
            Wave::Samples(samples) => samples[(t / SAMPLE_MS).floor() as usize % samples.len()] / 100.0
        }
    }
}

fn seeded_random(seed: f64) -> f64 {
    let x = (seed * 12.9898).sin() * 43758.5453;
    x - x.floor()
}

/// A random level every `step_ms`, eased from one to the next instead of jumping.
fn smooth_noise(t: f64, step_ms: f64, seed_offset: f64) -> f64 {
    let position = t / step_ms;
    let step = position.floor();
    let f = position - step;
    let eased = f * f * (3.0 - 2.0 * f);
    let from = seeded_random(step + seed_offset);
    from + (seeded_random(step + 1.0 + seed_offset) - from) * eased
}

pub(crate) fn builtin(id: &str) -> Option<fn(f64) -> f64> {
    Some(match id {
        "pulse" => |t| ((t / 250.0).sin() + 1.0) / 2.0,
        "wave" => |t| ((t / 2000.0).sin() + 1.0) / 2.0,
        "escalate" => |t| (t % 8000.0) / 8000.0,
        // A slow drift with a little faster wobble on top, kept off the extremes.
        "random" => |t| 0.15 + 0.75 * (0.75 * smooth_noise(t, 1200.0, 0.0) + 0.25 * smooth_noise(t, 400.0, 1000.0)),
        _ => return None
    })
}

/// Who started a pattern, and when it stops by itself, for the share page.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    #[serde(skip_serializing_if = "Option::is_none")]
    by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ends_at: Option<u64>
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PatternValue {
    control_id: String,
    value: String,
    info: Option<Info>
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Ended {
    control_id: String,
    run: u64
}

struct Runner {
    run: u64,
    actuators: Vec<ActuatorRef>,
    task: JoinHandle<()>
}

#[derive(Default)]
pub struct Patterns {
    runners: Mutex<HashMap<String, Runner>>,
    /// Each control's pattern value as last reported, so "off" isn't reported again and again.
    values: Mutex<HashMap<String, String>>,
    next_run: Mutex<u64>
}

fn patterns(app: &AppHandle) -> &Patterns {
    &app.state::<AppState>().inner().patterns
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or_default()
}

/// A control's pattern now, `off` when nothing was reported.
pub fn value(app: &AppHandle, control_id: &str) -> String {
    patterns(app).values.lock().unwrap().get(control_id).cloned().unwrap_or_else(|| OFF_ID.to_owned())
}

/// Reports a control's pattern value without playing anything, e.g. "off" while a viewer records.
pub fn report(app: &AppHandle, control_id: &str, value: &str) {
    emit_value(app, control_id, value, None);
}

fn emit_value(app: &AppHandle, control_id: &str, value: &str, info: Option<Info>) {
    patterns(app).values.lock().unwrap().insert(control_id.to_owned(), value.to_owned());
    let _ = app.emit("pattern-value", PatternValue { control_id: control_id.to_owned(), value: value.to_owned(), info });
}

fn overlaps(a: &[ActuatorRef], b: &[ActuatorRef]) -> bool {
    a.iter().any(|x| b.iter().any(|y| x.device_index == y.device_index && x.actuator_index == y.actuator_index))
}

/// Stops a control's runner without touching its toys or its value. Returns whether one ran.
pub fn stop(app: &AppHandle, control_id: &str) -> bool {
    let runner = patterns(app).runners.lock().unwrap().remove(control_id);
    match runner {
        Some(runner) => {
            runner.task.abort();
            let _ = app.emit("pattern-ended", Ended { control_id: control_id.to_owned(), run: runner.run });
            true
        }
        None => false
    }
}

/// Stops every runner but `except` that drives any of `actuators`, and reports those controls as
/// off. Call it before driving these actuators any other way.
pub fn release(app: &AppHandle, actuators: &[ActuatorRef], except: Option<&str>) {
    let overlapping: Vec<String> = patterns(app)
        .runners
        .lock()
        .unwrap()
        .iter()
        .filter(|(id, runner)| Some(id.as_str()) != except && overlaps(&runner.actuators, actuators))
        .map(|(id, _)| id.clone())
        .collect();

    for id in overlapping {
        stop(app, &id);
        emit_value(app, &id, OFF_ID, None);
    }
}

/// Stops a control's pattern and zeroes its toys.
pub fn off(app: &AppHandle, control_id: &str, actuators: &[ActuatorRef]) {
    stop(app, control_id);
    super::set(app, actuators, 0.0);
    emit_value(app, control_id, OFF_ID, None);
}

/// Plays `source` on `actuators` for this control, after claiming them. Returns the run's number,
/// which `pattern-ended` carries when it stops, or `None` for an unknown pattern.
pub fn play(
    app: &AppHandle,
    control_id: &str,
    source: Source,
    actuators: Vec<ActuatorRef>,
    stop_after_ms: Option<u64>,
    by: Option<String>
) -> Option<u64> {
    if source.id == OFF_ID && source.samples.is_none() {
        off(app, control_id, &actuators);
        return None;
    }

    let wave = match source.samples {
        Some(samples) => Wave::Samples(samples),
        None => match builtin(&source.id) {
            Some(value) => Wave::Builtin(value),
            None => Wave::Samples(saved::samples(app, &source.id)?)
        }
    };

    stop(app, control_id);
    release(app, &actuators, Some(control_id));

    let run = {
        let mut next = patterns(app).next_run.lock().unwrap();
        *next += 1;
        *next
    };

    let task_app = app.clone();
    let task_actuators = actuators.clone();
    let task_control = control_id.to_owned();
    let task = tauri::async_runtime::spawn(async move {
        let started = Instant::now();
        let mut ticker = interval(TICK);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            let elapsed = started.elapsed();
            if stop_after_ms.is_some_and(|ms| elapsed >= Duration::from_millis(ms)) {
                off(&task_app, &task_control, &task_actuators);
                return;
            }
            super::set(&task_app, &task_actuators, wave.value(elapsed.as_secs_f64() * 1000.0));
        }
    });

    patterns(app).runners.lock().unwrap().insert(control_id.to_owned(), Runner { run, actuators, task });

    let info = (by.is_some() || stop_after_ms.is_some())
        .then(|| Info { by, ends_at: stop_after_ms.map(|ms| now_ms() + ms) });
    emit_value(app, control_id, &source.id, info);
    Some(run)
}

/// Stop everything: every runner stops, its toys go to 0 and its control reads off.
pub fn stop_all(app: &AppHandle) {
    let running: Vec<(String, Vec<ActuatorRef>)> =
        patterns(app).runners.lock().unwrap().iter().map(|(id, runner)| (id.clone(), runner.actuators.clone())).collect();
    for (id, actuators) in running {
        off(app, &id, &actuators);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Computed with the webview's own functions (useIntifacePatterns.ts) at 0, 50, 777, 5000 and
    /// 12345 ms.
    const EXPECTED: [(&str, [f64; 5]); 4] = [
        ("pulse", [0.5, 0.599334665398, 0.516793167958, 0.956472625364, 0.112889783861]),
        ("wave", [0.5, 0.512498697957, 0.689400312171, 0.799236072052, 0.444770279872]),
        ("escalate", [0.0, 0.00625, 0.097125, 0.625, 0.543125]),
        ("random", [0.326884533342, 0.323438726507, 0.533938827016, 0.408962552886, 0.724547313526])
    ];

    #[test]
    fn builtins_match_the_webview() {
        for (id, values) in EXPECTED {
            let value = builtin(id).unwrap();
            for (t, expected) in [0.0, 50.0, 777.0, 5000.0, 12345.0].into_iter().zip(values) {
                assert!((value(t) - expected).abs() < 1e-9, "{id} at {t}: {} vs {expected}", value(t));
            }
        }
    }

    #[test]
    fn samples_loop_every_50_ms() {
        let wave = Wave::Samples(vec![0.0, 50.0, 100.0]);
        assert_eq!(wave.value(0.0), 0.0);
        assert_eq!(wave.value(60.0), 0.5);
        assert_eq!(wave.value(149.0), 1.0);
        assert_eq!(wave.value(150.0), 0.0);
    }
}
