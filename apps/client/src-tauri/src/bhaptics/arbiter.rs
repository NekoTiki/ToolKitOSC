//! Decides what each bHaptics motor plays, every tick, the way `toys::arbiter` does for toys.
//!
//! Avatar touch comes first, per position: while anything touches your vest front (above the
//! threshold, and for a grace period after), the vest front plays touch only, and what viewers,
//! effects and automations ask for there is held back. Your back stays theirs. Everyone else asks
//! through a lease that runs out by itself, so a closed share page or a lost connection stops its
//! motors without anyone saying so.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::{Duration, Instant};

use super::model::Position;
use super::protocol::Submit;
use super::touch::Levels;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Priority {
    pub threshold: f64,
    pub grace: Duration
}

/// The same defaults as avatar haptics' touch priority.
pub const PRIORITY: Priority = Priority { threshold: 0.02, grace: Duration::from_millis(1500) };

#[derive(Debug, Clone)]
struct Lease {
    position: Position,
    levels: Vec<f64>,
    until: Instant
}

/// What viewers, effects, automations and Test motors ask for, each under its own id until it
/// runs out.
#[derive(Debug, Default)]
pub struct Leases {
    leases: HashMap<String, Lease>
}

impl Leases {
    /// Asks for these levels on `position` until `until`, replacing what `id` asked for there.
    pub fn set(&mut self, id: &str, position: Position, levels: Vec<f64>, until: Instant) {
        let mut levels = levels;
        levels.resize(position.motors(), 0.0);
        self.leases.insert(format!("{id}@{}", position.name()), Lease { position, levels, until });
    }

    /// Lets go of everything `id` asked for.
    pub fn remove(&mut self, id: &str) {
        let prefix = format!("{id}@");
        self.leases.retain(|key, _| !key.starts_with(&prefix));
    }

    pub fn clear(&mut self) {
        self.leases.clear();
    }

    /// The strongest ask per motor, dropping the ones that ran out.
    pub fn levels(&mut self, now: Instant) -> Levels {
        self.leases.retain(|_, lease| lease.until > now);
        let mut levels = Levels::new();
        for lease in self.leases.values() {
            let motors = levels.entry(lease.position).or_insert_with(|| vec![0.0; lease.position.motors()]);
            for (motor, level) in motors.iter_mut().zip(&lease.levels) {
                *motor = motor.max(level.clamp(0.0, 1.0));
            }
        }
        levels
    }
}

/// Each position's state, without the sending, so it can be tested.
#[derive(Debug, Default)]
pub struct Arbiter {
    /// Touch keeps a position until then, even below the threshold.
    touch_until: HashMap<Position, Instant>,
    touched: HashSet<Position>,
    /// What's playing now, 0–100 per motor, only for positions that play.
    playing: BTreeMap<Position, Vec<u8>>
}

impl Arbiter {
    /// One tick: touch and everyone else's asks become what each position plays, scaled by its
    /// strength. Returns what to send: a frame for every position that plays (they only last
    /// `protocol::FRAME_MS`), and a turn-off for each that just stopped.
    pub fn tick(&mut self, touch: &Levels, asks: &Levels, strength: impl Fn(Position) -> f64, priority: Priority, now: Instant) -> Vec<Submit> {
        let mut submits = Vec::new();
        for position in Position::ALL {
            let touch_levels = touch.get(&position);
            if touch_levels.is_some_and(|levels| levels.iter().any(|level| *level > priority.threshold)) {
                self.touch_until.insert(position, now + priority.grace);
            }
            let owned = self.touch_until.get(&position).is_some_and(|until| now < *until);
            if owned {
                self.touched.insert(position);
            } else {
                self.touch_until.remove(&position);
                self.touched.remove(&position);
            }

            let levels = if owned { touch_levels } else { asks.get(&position) };
            let strength = strength(position).clamp(0.0, 1.0);
            let intensities: Vec<u8> = (0..position.motors())
                .map(|motor| {
                    let level = levels.and_then(|levels| levels.get(motor)).copied().unwrap_or(0.0).clamp(0.0, 1.0);
                    (level * strength * 100.0).round() as u8
                })
                .collect();

            if intensities.iter().any(|intensity| *intensity > 0) {
                submits.push(Submit::Frame { position, intensities: intensities.clone() });
                self.playing.insert(position, intensities);
            } else if self.playing.remove(&position).is_some() {
                submits.push(Submit::TurnOff(position));
            }
        }
        submits
    }

    /// Positions touch drives now: viewers' taps there wait.
    pub fn touched(&self) -> Vec<Position> {
        let mut touched: Vec<Position> = self.touched.iter().copied().collect();
        touched.sort();
        touched
    }

    pub fn playing(&self) -> &BTreeMap<Position, Vec<u8>> {
        &self.playing
    }

    /// Stop everything, or the link dropped: nothing plays, touch lets go.
    pub fn clear(&mut self) {
        self.touch_until.clear();
        self.touched.clear();
        self.playing.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full(_: Position) -> f64 {
        1.0
    }

    fn one(position: Position, motor: usize, level: f64) -> Levels {
        let mut levels = vec![0.0; position.motors()];
        levels[motor] = level;
        Levels::from([(position, levels)])
    }

    fn frame(position: Position, motor: usize, intensity: u8) -> Submit {
        let mut intensities = vec![0; position.motors()];
        intensities[motor] = intensity;
        Submit::Frame { position, intensities }
    }

    #[test]
    fn plays_every_tick_and_turns_off_once() {
        let mut arbiter = Arbiter::default();
        let now = Instant::now();
        let touch = one(Position::VestFront, 6, 1.0);
        let strength = |position: Position| if position == Position::VestFront { 0.7 } else { 1.0 };
        assert_eq!(arbiter.tick(&touch, &Levels::new(), strength, PRIORITY, now), vec![frame(Position::VestFront, 6, 70)]);
        assert_eq!(arbiter.tick(&touch, &Levels::new(), strength, PRIORITY, now), vec![frame(Position::VestFront, 6, 70)], "frames run out");
        let later = now + Duration::from_secs(2);
        assert_eq!(arbiter.tick(&Levels::new(), &Levels::new(), strength, PRIORITY, later), vec![Submit::TurnOff(Position::VestFront)]);
        assert_eq!(arbiter.tick(&Levels::new(), &Levels::new(), strength, PRIORITY, later), vec![]);
    }

    #[test]
    fn touch_holds_its_side_back_and_leaves_the_other_open() {
        let mut arbiter = Arbiter::default();
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let touch = one(Position::VestFront, 6, 1.0);
        let mut asks = one(Position::VestFront, 0, 0.5);
        asks.extend(one(Position::VestBack, 0, 0.5));

        let sent = arbiter.tick(&touch, &asks, full, PRIORITY, at(0));
        assert_eq!(sent, vec![frame(Position::VestFront, 6, 100), frame(Position::VestBack, 0, 50)]);
        assert_eq!(arbiter.touched(), [Position::VestFront]);

        // Touch lets go: the front stays quiet for the grace, then the viewer's ask comes back.
        let sent = arbiter.tick(&Levels::new(), &asks, full, PRIORITY, at(1000));
        assert_eq!(sent, vec![Submit::TurnOff(Position::VestFront), frame(Position::VestBack, 0, 50)]);
        assert_eq!(arbiter.tick(&Levels::new(), &asks, full, PRIORITY, at(1400))[0], frame(Position::VestBack, 0, 50));
        let sent = arbiter.tick(&Levels::new(), &asks, full, PRIORITY, at(1600));
        assert_eq!(sent, vec![frame(Position::VestFront, 0, 50), frame(Position::VestBack, 0, 50)]);
        assert!(arbiter.touched().is_empty());
    }

    #[test]
    fn leases_run_out_and_the_strongest_wins() {
        let start = Instant::now();
        let mut leases = Leases::default();
        leases.set("viewer:sam", Position::VestBack, vec![0.2, 0.6], start + Duration::from_millis(300));
        leases.set("effect", Position::VestBack, vec![0.5], start + Duration::from_secs(2));
        let levels = leases.levels(start);
        assert_eq!(&levels[&Position::VestBack][..3], [0.5, 0.6, 0.0]);
        let levels = leases.levels(start + Duration::from_millis(300));
        assert_eq!(&levels[&Position::VestBack][..2], [0.5, 0.0], "the viewer's ran out");
        leases.remove("effect");
        assert!(leases.levels(start).is_empty());
    }

    #[test]
    fn a_new_lease_from_the_same_asker_replaces_the_old_one() {
        let start = Instant::now();
        let mut leases = Leases::default();
        let until = start + Duration::from_secs(1);
        leases.set("viewer:sam", Position::VestBack, vec![1.0], until);
        leases.set("viewer:sam", Position::VestBack, vec![0.0, 1.0], until);
        assert_eq!(&leases.levels(start)[&Position::VestBack][..2], [0.0, 1.0]);
    }

    #[test]
    fn clearing_stops_and_forgets() {
        let mut arbiter = Arbiter::default();
        let now = Instant::now();
        arbiter.tick(&one(Position::Head, 0, 1.0), &Levels::new(), full, PRIORITY, now);
        arbiter.clear();
        assert!(arbiter.touched().is_empty());
        assert!(arbiter.playing().is_empty());
        assert_eq!(arbiter.tick(&Levels::new(), &Levels::new(), full, PRIORITY, now), vec![], "nothing to turn off twice");
    }
}
