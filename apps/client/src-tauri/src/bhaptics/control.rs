//! The bHaptics control (`BhapticsControl` in `types/controls.ts`): viewers holding, dragging and
//! tapping on the share page, and you doing the same on your own tile.
//!
//! A hold is a lease: each message keeps the motors under the finger on for `HOLD_LEASE`, so a
//! closed tab or a lost connection stops them almost at once. The page sends one every 100 ms.
use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde_json::Value;
use tauri::AppHandle;

use super::arbiter::{spread, Spread};
use super::model::Position;

const HOLD_LEASE: Duration = Duration::from_millis(300);
/// Hold messages closer together than this are dropped: the page sends one every 100 ms.
const MIN_HOLD_GAP: Duration = Duration::from_millis(80);
/// A hold not heard from in this long is forgotten.
const FORGET_AFTER: Duration = Duration::from_secs(5);

#[derive(Debug, Default)]
pub struct Hold {
    started: Option<Instant>,
    last: Option<Instant>
}

fn number(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|n| n.is_finite())
}

fn millis(value: &Value, key: &str, default: u64, min: u64, max: u64) -> Duration {
    Duration::from_millis(number(value, key).map_or(default, |ms| (ms.max(0.0) as u64).clamp(min, max)))
}

/// What a hold message does, without the app, so it can be tested.
#[derive(Debug, PartialEq)]
pub enum HoldStep {
    /// Keep these motors on: the first message of the hold, or a later one.
    Play { first: bool },
    /// Too soon after the last one: what plays keeps playing.
    Early,
    /// Ended, or past the longest hold: its motors stop.
    Over
}

/// The holds going on now, by key.
pub fn hold_step(holds: &mut HashMap<String, Hold>, key: &str, ending: bool, max_hold: Duration, now: Instant) -> HoldStep {
    holds.retain(|_, hold| hold.last.is_some_and(|last| now.duration_since(last) < FORGET_AFTER));
    if ending {
        holds.remove(key);
        return HoldStep::Over;
    }
    let hold = holds.entry(key.to_owned()).or_default();
    let started = *hold.started.get_or_insert(now);
    let first = hold.last.is_none();
    if now.duration_since(started) > max_hold {
        // Kept, so the next message doesn't start it over.
        hold.last = Some(now);
        return HoldStep::Over;
    }
    if hold.last.is_some_and(|last| now.duration_since(last) < MIN_HOLD_GAP) {
        return HoldStep::Early;
    }
    hold.last = Some(now);
    HoldStep::Play { first }
}

/// Runs a checked command on a bHaptics control. True when it's a hold still going on: those only
/// move the motors, with nothing to log, announce or show.
pub fn run(app: &AppHandle, control: &Value, command: &Value) -> bool {
    let Some(control_id) = control.get("id").and_then(Value::as_str) else {
        return false;
    };
    let Some(position) = command.get("position").cloned().and_then(|p| serde_json::from_value::<Position>(p).ok()) else {
        return false;
    };
    let motors: Vec<usize> = command
        .get("motors")
        .and_then(Value::as_array)
        .map(|motors| motors.iter().filter_map(Value::as_u64).map(|m| m as usize).filter(|m| *m < position.motors()).collect())
        .unwrap_or_default();
    let cap = number(control, "maxIntensity").unwrap_or(1.0).clamp(0.0, 1.0);
    let level = number(command, "level").unwrap_or(cap).clamp(0.0, cap);
    let mode = control.get("spread").cloned().and_then(|s| serde_json::from_value(s).ok()).unwrap_or(Spread::Soft);
    let now = Instant::now();

    match command.get("value").and_then(Value::as_str) {
        Some("tap") if !motors.is_empty() => {
            let pulse = millis(control, "pulseMs", 400, 100, 2000);
            super::ask(app, &format!("tap:{control_id}"), position, spread(position, &motors, mode, level), now + pulse);
            false
        }
        Some("hold") => {
            let Some(hold_id) = number(command, "holdId") else {
                return true;
            };
            let key = format!("hold:{control_id}:{hold_id}");
            let max_hold = millis(control, "maxHoldMs", 10_000, 2_000, 30_000);
            let step = hold_step(&mut super::bhaptics(app).holds.lock().unwrap(), &key, motors.is_empty(), max_hold, now);
            match step {
                HoldStep::Play { first } => {
                    // A drag onto the other side carries on there: the first side stops now.
                    super::let_go(app, &key);
                    super::ask(app, &key, position, spread(position, &motors, mode, level), now + HOLD_LEASE);
                    !first
                }
                HoldStep::Early => true,
                HoldStep::Over => {
                    super::let_go(app, &key);
                    true
                }
            }
        }
        _ => false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX: Duration = Duration::from_secs(10);

    #[test]
    fn only_the_first_message_of_a_hold_is_new() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let mut holds = HashMap::new();
        assert_eq!(hold_step(&mut holds, "a", false, MAX, at(0)), HoldStep::Play { first: true });
        assert_eq!(hold_step(&mut holds, "a", false, MAX, at(100)), HoldStep::Play { first: false });
        assert_eq!(hold_step(&mut holds, "a", false, MAX, at(150)), HoldStep::Early, "faster than the page sends");
        assert_eq!(hold_step(&mut holds, "a", true, MAX, at(250)), HoldStep::Over);
        assert!(holds.is_empty());
    }

    #[test]
    fn a_hold_is_cut_off_after_the_longest() {
        let start = Instant::now();
        let mut holds = HashMap::new();
        hold_step(&mut holds, "a", false, MAX, start);
        for s in 1..=10 {
            assert_eq!(hold_step(&mut holds, "a", false, MAX, start + Duration::from_secs(s)), HoldStep::Play { first: false });
        }
        assert_eq!(hold_step(&mut holds, "a", false, MAX, start + Duration::from_millis(10_100)), HoldStep::Over);
        assert_eq!(hold_step(&mut holds, "a", false, MAX, start + Duration::from_millis(10_300)), HoldStep::Over, "and stays cut off");
    }

    #[test]
    fn forgotten_holds_are_dropped() {
        let start = Instant::now();
        let mut holds = HashMap::new();
        hold_step(&mut holds, "a", false, MAX, start);
        hold_step(&mut holds, "b", false, MAX, start + Duration::from_secs(6));
        assert_eq!(holds.len(), 1);
    }
}
