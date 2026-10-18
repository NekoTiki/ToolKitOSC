//! The OWO control (`OwoControl` in `types/controls.ts`): viewers playing a sensation on a muscle
//! from the share page, and you doing the same on your own tile.
//!
//! It's electrical, so like a shocker each control has a cooldown of at least 2 s: a tap during
//! it is dropped without being logged.
use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde_json::Value;
use tauri::AppHandle;

use super::model::Muscle;

pub const MIN_COOLDOWN: Duration = Duration::from_secs(2);

fn millis(value: &Value, key: &str, default: u64, min: u64, max: u64) -> Duration {
    let ms = value.get(key).and_then(Value::as_f64).filter(|ms| ms.is_finite());
    Duration::from_millis(ms.map_or(default, |ms| (ms.max(0.0) as u64).clamp(min, max)))
}

/// Starts `id`'s cooldown unless it's still running. False when it is.
pub fn cooled(cooldowns: &mut HashMap<String, Instant>, id: &str, cooldown: Duration, now: Instant) -> bool {
    if cooldowns.get(id).is_some_and(|end| now < *end) {
        return false;
    }
    cooldowns.insert(id.to_owned(), now + cooldown.max(MIN_COOLDOWN));
    true
}

/// Runs a checked command on an OWO control. True when it was dropped during the cooldown: nothing
/// to log, announce or show.
pub fn run(app: &AppHandle, control: &Value, command: &Value) -> bool {
    let Some(control_id) = control.get("id").and_then(Value::as_str) else {
        return false;
    };
    let Some(sensation) = command.get("value").and_then(Value::as_str) else {
        return false;
    };
    let muscles: Vec<Muscle> = command.get("muscles").cloned().and_then(|m| serde_json::from_value(m).ok()).unwrap_or_default();
    let cooldown = millis(control, "cooldown", 2000, 0, 600_000);
    if !cooled(&mut super::owo(app).cooldowns.lock().unwrap(), control_id, cooldown, Instant::now()) {
        return true;
    }
    // Viewers' commands come checked, with their level; your own tile's don't, and play at the cap.
    let cap = control.get("maxIntensity").and_then(Value::as_f64).unwrap_or(0.0).clamp(0.0, 1.0);
    let level = command.get("level").and_then(Value::as_f64).unwrap_or(cap).clamp(0.0, cap);
    let longest = millis(control, "maxDurationMs", 500, 100, 2000);
    super::play_sensation(app, &format!("tap:{control_id}"), sensation, muscles, level, longest);
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tap_during_the_cooldown_is_dropped() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let mut cooldowns = HashMap::new();
        assert!(cooled(&mut cooldowns, "c", Duration::from_secs(3), at(0)));
        assert!(!cooled(&mut cooldowns, "c", Duration::from_secs(3), at(2999)));
        assert!(cooled(&mut cooldowns, "other", Duration::from_secs(3), at(10)), "each control has its own");
        assert!(cooled(&mut cooldowns, "c", Duration::from_secs(3), at(3000)));
    }

    #[test]
    fn never_shorter_than_two_seconds() {
        let start = Instant::now();
        let mut cooldowns = HashMap::new();
        assert!(cooled(&mut cooldowns, "c", Duration::ZERO, start));
        assert!(!cooled(&mut cooldowns, "c", Duration::ZERO, start + Duration::from_millis(1999)));
    }
}
