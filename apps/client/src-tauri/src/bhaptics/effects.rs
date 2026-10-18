//! The built-in effects: short sequences played across your gear by the tick, from the bHaptics
//! control's buttons and from automations. Every timing is here, to tune after trying them on a
//! real vest.
use std::time::Duration;

use super::model::Position;
use super::touch::Levels;

pub const EFFECTS: [&str; 3] = ["heartbeat", "rain", "sweep"];

pub fn exists(effect: &str) -> bool {
    EFFECTS.contains(&effect)
}

const HEARTBEAT_MS: u128 = 2700;
/// One beat every this long, each a double thump: on, off, on.
const HEARTBEAT_CYCLE_MS: u128 = 900;
const HEARTBEAT_THUMPS: [(u128, u128); 2] = [(0, 120), (220, 340)];
/// The upper chest, left of centre as you face the wearer, front and back alike.
const HEARTBEAT_VEST: [usize; 4] = [1, 2, 5, 6];

const RAIN_MS: u128 = 3000;
const RAIN_STEP_MS: u128 = 100;
const RAIN_LEVEL: f64 = 0.8;

const SWEEP_MS: u128 = 1500;
const SWEEP_TRAIL: f64 = 0.35;

/// A small, fixed mix, so rain falls the same way in tests and differently for each position.
fn mix(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    x ^ (x >> 33)
}

/// What `effect` plays on `positions` at `t` after it started, 0 to 1 per motor. `None` once it's
/// over. `seed` changes where rain falls.
pub fn levels(effect: &str, positions: &[Position], t: Duration, seed: u64) -> Option<Levels> {
    let t = t.as_millis();
    let mut levels = Levels::new();
    match effect {
        "heartbeat" => {
            if t >= HEARTBEAT_MS {
                return None;
            }
            let at = t % HEARTBEAT_CYCLE_MS;
            let on = HEARTBEAT_THUMPS.iter().any(|(from, to)| (*from..*to).contains(&at));
            for position in positions {
                let mut motors = vec![0.0; position.motors()];
                if on {
                    match position {
                        Position::VestFront | Position::VestBack => HEARTBEAT_VEST.iter().for_each(|m| motors[*m] = 1.0),
                        _ => motors.iter_mut().for_each(|m| *m = 1.0)
                    }
                }
                levels.insert(*position, motors);
            }
        }
        "rain" => {
            if t >= RAIN_MS {
                return None;
            }
            let step = (t / RAIN_STEP_MS) as u64;
            for (index, position) in positions.iter().enumerate() {
                let count = position.motors();
                let mut motors = vec![0.0; count];
                for drop in 0..count.div_ceil(5) as u64 {
                    let motor = mix(seed ^ (step << 16) ^ ((index as u64) << 8) ^ drop) as usize % count;
                    motors[motor] = RAIN_LEVEL;
                }
                levels.insert(*position, motors);
            }
        }
        "sweep" => {
            if t >= SWEEP_MS {
                return None;
            }
            for position in positions {
                let columns = position.columns();
                let rows = position.motors().div_ceil(columns);
                // From the bottom row up.
                let row = rows - 1 - (t * rows as u128 / SWEEP_MS) as usize;
                let mut motors = vec![0.0; position.motors()];
                for (motor, level) in motors.iter_mut().enumerate() {
                    if motor / columns == row {
                        *level = 1.0;
                    } else if motor / columns == row + 1 {
                        *level = SWEEP_TRAIL;
                    }
                }
                levels.insert(*position, motors);
            }
        }
        _ => return None
    }
    Some(levels)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VEST: [Position; 2] = [Position::VestFront, Position::VestBack];

    fn at(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    fn lit(levels: &Levels, position: Position) -> Vec<usize> {
        levels[&position].iter().enumerate().filter(|(_, l)| **l > 0.0).map(|(m, _)| m).collect()
    }

    #[test]
    fn heartbeat_thumps_twice_a_beat_on_the_chest() {
        assert_eq!(lit(&levels("heartbeat", &VEST, at(50), 0).unwrap(), Position::VestFront), HEARTBEAT_VEST);
        assert!(lit(&levels("heartbeat", &VEST, at(150), 0).unwrap(), Position::VestFront).is_empty());
        assert_eq!(lit(&levels("heartbeat", &VEST, at(250), 0).unwrap(), Position::VestBack), HEARTBEAT_VEST);
        assert_eq!(lit(&levels("heartbeat", &[Position::Head], at(950), 0).unwrap(), Position::Head).len(), 6);
        assert!(levels("heartbeat", &VEST, at(2700), 0).is_none());
    }

    #[test]
    fn sweep_climbs_from_the_bottom_row() {
        let start = levels("sweep", &[Position::VestBack], at(0), 0).unwrap();
        assert_eq!(lit(&start, Position::VestBack), [16, 17, 18, 19]);
        let end = levels("sweep", &[Position::VestBack], at(1499), 0).unwrap();
        assert_eq!(lit(&end, Position::VestBack), [0, 1, 2, 3, 4, 5, 6, 7], "the top row, and the trail below");
        assert_eq!(end[&Position::VestBack][4], SWEEP_TRAIL);
        assert!(levels("sweep", &[Position::VestBack], at(1500), 0).is_none());
    }

    #[test]
    fn rain_falls_somewhere_new_each_step() {
        let first = levels("rain", &VEST, at(0), 1).unwrap();
        assert!((1..=4).contains(&lit(&first, Position::VestFront).len()));
        let changes = (1..30).filter(|step| levels("rain", &VEST, at(step * 100), 1).unwrap() != first).count();
        assert!(changes > 20);
        assert!(levels("rain", &VEST, at(3000), 1).is_none());
    }

    #[test]
    fn unknown_effects_play_nothing() {
        assert!(levels("explode", &VEST, at(0), 0).is_none());
        assert!(!exists("explode"));
    }
}
