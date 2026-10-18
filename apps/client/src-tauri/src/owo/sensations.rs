//! The built-in sensations viewers and automations play (`OWO_SENSATIONS` in `types/controls.ts`).
//! Each is a few steps of a level over time. They don't use OWO's baked sensations, which need a
//! registered game ID: the tick sends each step as it comes, in the one packet it sends anyway.
use std::time::Duration;

/// Each step: how long, at what part of the sensation's level.
const STEPS: [(&str, &[(u64, f64)]); 4] = [
    ("tap", &[(300, 1.0)]),
    ("pulse", &[(200, 1.0), (200, 0.0), (200, 1.0), (200, 0.0), (200, 1.0)]),
    ("punch", &[(150, 1.0), (250, 0.5)]),
    ("heartbeat", &[(120, 1.0), (150, 0.0), (120, 0.7), (600, 0.0), (120, 1.0), (150, 0.0), (120, 0.7)])
];

fn steps(sensation: &str) -> Option<&'static [(u64, f64)]> {
    STEPS.iter().find(|(id, _)| *id == sensation).map(|(_, steps)| *steps)
}

pub fn exists(sensation: &str) -> bool {
    steps(sensation).is_some()
}

/// The part of the level `sensation` plays at `t`, or None once it's over.
pub fn level(sensation: &str, t: Duration) -> Option<f64> {
    let mut start = 0;
    for (ms, level) in steps(sensation)? {
        start += ms;
        if (t.as_millis() as u64) < start {
            return Some(*level);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    #[test]
    fn plays_its_steps_then_ends() {
        assert_eq!(level("tap", at(0)), Some(1.0));
        assert_eq!(level("tap", at(299)), Some(1.0));
        assert_eq!(level("tap", at(300)), None);
        assert_eq!(level("pulse", at(250)), Some(0.0), "between two pulses");
        assert_eq!(level("punch", at(200)), Some(0.5));
        assert_eq!(level("heartbeat", at(1379)), Some(0.7));
        assert_eq!(level("heartbeat", at(1380)), None);
    }

    #[test]
    fn knows_only_its_own() {
        assert!(exists("heartbeat"));
        assert!(!exists("explode"));
        assert_eq!(level("explode", at(0)), None);
    }
}
