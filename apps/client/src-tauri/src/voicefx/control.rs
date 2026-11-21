//! The Voice effects control (see shared-ui `VoiceFxControl`): a toggle or a timed button with
//! its effect and strength set, or all in one, where whoever uses it picks them.
use std::time::{Duration, Instant};

use serde_json::Value;
use tauri::AppHandle;

use super::holds::Source;
use super::model::Effect;

/// The longest a timed control plays, and waits before it can be used again.
pub const MAX_SECONDS: f64 = 600.0;

/// What a control and a command for it hold on, or `None` when the command turns it off.
/// Viewer commands were checked before (`server::limits`); your own clicks act as you.
#[derive(Debug, PartialEq)]
pub struct Wanted {
    pub effect: Effect,
    pub level: f64,
    /// How long it plays, then how long before it can be used again. `None`: until turned off.
    pub timed: Option<(Duration, Duration)>
}

pub fn wanted(control: &Value, command: &Value) -> Option<Wanted> {
    if command.get("value").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    let kind = control.get("kind").and_then(Value::as_str).unwrap_or("toggle");
    // All in one: the command says what to play. Otherwise the control does.
    let from = if kind == "pick" { command } else { control };
    let effect = from.get("effect").and_then(Value::as_str).and_then(Effect::parse)?;
    let level = from.get("level").and_then(Value::as_f64).unwrap_or(1.0).clamp(0.0, 1.0);
    let timed = kind == "timed" || (kind == "pick" && command.get("timed").and_then(Value::as_bool) == Some(true));
    let seconds = |key: &str, least: f64| Duration::from_secs_f64(control.get(key).and_then(Value::as_f64).unwrap_or(least).clamp(least, MAX_SECONDS));
    Some(Wanted { effect, level, timed: timed.then(|| (seconds("seconds", 1.0), seconds("cooldown", 0.0))) })
}

/// Runs a command on a Voice effects control. True when it was dropped: Stop everything is on, or
/// a timed control is still playing or cooling down.
pub fn run(app: &AppHandle, control: &Value, command: &Value, viewer_name: Option<String>) -> bool {
    let Some(id) = control.get("id").and_then(Value::as_str) else {
        return true;
    };
    let v = super::voicefx(app);
    let source = Source::Control(id.to_owned());
    let dropped = {
        let mut run = v.run.lock().unwrap();
        match wanted(control, command) {
            None => {
                run.holds.release(&source);
                false
            }
            Some(_) if crate::automations::is_paused(app) => true,
            Some(Wanted { effect, level, timed: None }) => {
                run.holds.hold(source, viewer_name, Some(effect), level, None);
                false
            }
            Some(Wanted { effect, level, timed: Some((plays, cooldown)) }) => {
                let now = Instant::now();
                if run.cooldowns.get(id).is_some_and(|until| *until > now) {
                    true
                } else {
                    run.cooldowns.insert(id.to_owned(), now + plays + cooldown);
                    run.holds.hold(source, viewer_name, Some(effect), level, Some(now + plays));
                    false
                }
            }
        }
    };
    v.wake.notify_one();
    dropped
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_toggle_plays_what_the_control_says() {
        let control = json!({ "kind": "toggle", "effect": "muffled", "level": 0.7 });
        // Whatever the command adds is ignored.
        let command = json!({ "value": true, "effect": "radio", "level": 1.0, "timed": true });
        assert_eq!(wanted(&control, &command), Some(Wanted { effect: Effect::Muffled, level: 0.7, timed: None }));
        assert_eq!(wanted(&control, &json!({ "value": false })), None);
    }

    #[test]
    fn a_timed_control_plays_for_its_length_then_cools_down() {
        let control = json!({ "kind": "timed", "effect": "underwater", "level": 0.8, "seconds": 20, "cooldown": 30 });
        let wanted = wanted(&control, &json!({ "value": true })).unwrap();
        assert_eq!(wanted.timed, Some((Duration::from_secs(20), Duration::from_secs(30))));
        assert_eq!(wanted.effect, Effect::Underwater);
    }

    #[test]
    fn all_in_one_plays_what_the_command_says() {
        let control = json!({ "kind": "pick", "effect": "muffled", "level": 0.2, "seconds": 10 });
        let held = wanted(&control, &json!({ "value": true, "effect": "radio", "level": 0.9 })).unwrap();
        assert_eq!(held, Wanted { effect: Effect::Radio, level: 0.9, timed: None });
        let timed = wanted(&control, &json!({ "value": true, "effect": "radio", "level": 0.9, "timed": true })).unwrap();
        assert_eq!(timed.timed, Some((Duration::from_secs(10), Duration::ZERO)));
        // No effect named: nothing to play.
        assert_eq!(wanted(&control, &json!({ "value": true })), None);
    }
}
