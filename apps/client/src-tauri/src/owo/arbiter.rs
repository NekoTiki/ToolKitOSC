//! Decides what each OWO muscle plays, every tick, the way `bhaptics::arbiter` does for motors.
//!
//! A new sensation replaces the one playing, so nothing sends on its own: touch, viewers,
//! automations and Test each muscle all end up in the one packet of each tick.
//!
//! Avatar touch comes first, per side: while anything touches a muscle on your front (and for a
//! grace period after), the front plays touch only, and what viewers and automations ask for there
//! is held back. Your back stays theirs.
use std::collections::{BTreeSet, HashMap};
use std::time::{Duration, Instant};

use super::model::{Muscle, Side};
use super::touch::Levels;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Priority {
    pub threshold: f64,
    pub grace: Duration
}

/// The same defaults as avatar haptics' and bHaptics' touch priority.
pub const PRIORITY: Priority = Priority { threshold: 0.02, grace: Duration::from_millis(1500) };

#[derive(Debug, Clone)]
struct Lease {
    levels: Levels,
    until: Instant
}

/// What viewers, automations and Test each muscle ask for, each under its own id until it runs out.
#[derive(Debug, Default)]
pub struct Leases {
    leases: HashMap<String, Lease>
}

impl Leases {
    /// Asks for these levels until `until`, replacing what `id` asked for.
    pub fn set(&mut self, id: &str, levels: Levels, until: Instant) {
        self.leases.insert(id.to_owned(), Lease { levels, until });
    }

    pub fn remove(&mut self, id: &str) {
        self.leases.remove(id);
    }

    pub fn clear(&mut self) {
        self.leases.clear();
    }

    /// The strongest ask per muscle, dropping the ones that ran out.
    pub fn levels(&mut self, now: Instant) -> Levels {
        self.leases.retain(|_, lease| lease.until > now);
        let mut levels = Levels::new();
        for (muscle, level) in self.leases.values().flat_map(|lease| &lease.levels) {
            let out = levels.entry(*muscle).or_default();
            *out = out.max(level.clamp(0.0, 1.0));
        }
        levels
    }
}

#[derive(Debug, Default)]
pub struct Arbiter {
    /// Touch keeps a side until then, even below the threshold.
    touch_until: HashMap<Side, Instant>,
    /// What plays now, only muscles above 0.
    playing: Levels
}

impl Arbiter {
    /// One tick: touch and everyone else's asks become what each muscle plays, scaled by its
    /// strength. Empty when nothing plays.
    pub fn tick(&mut self, touch: &Levels, asks: &Levels, strength: impl Fn(Muscle) -> f64, priority: Priority, now: Instant) -> &Levels {
        for (muscle, level) in touch {
            if *level > priority.threshold {
                self.touch_until.insert(muscle.side(), now + priority.grace);
            }
        }
        self.touch_until.retain(|_, until| now < *until);

        self.playing = Muscle::ALL
            .into_iter()
            .filter_map(|muscle| {
                let from = if self.touch_until.contains_key(&muscle.side()) { touch } else { asks };
                let level = from.get(&muscle).copied().unwrap_or(0.0).clamp(0.0, 1.0) * strength(muscle).clamp(0.0, 1.0);
                (level > 0.0).then_some((muscle, level))
            })
            .collect();
        &self.playing
    }

    /// Sides touch drives now: viewers' taps there wait.
    pub fn touched(&self) -> Vec<Side> {
        self.touch_until.keys().copied().collect::<BTreeSet<_>>().into_iter().collect()
    }

    pub fn playing(&self) -> &Levels {
        &self.playing
    }

    /// Stop everything, or the link dropped: nothing plays, touch lets go.
    pub fn clear(&mut self) {
        self.touch_until.clear();
        self.playing.clear();
    }
}

/// What the tick sends this time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Send {
    /// What plays now, as one sensation.
    Play,
    /// Everything just went quiet: stop at once rather than let the last sensation run out.
    Stop
}

/// When to send: as soon as what plays changes (a sensation's next step, a new touch), and again
/// before the last sensation runs out.
#[derive(Debug, Default)]
pub struct Pace {
    last: Option<(Instant, Levels)>
}

impl Pace {
    pub fn due(&mut self, playing: &Levels, resend: Duration, now: Instant) -> Option<Send> {
        let send = match &self.last {
            _ if playing.is_empty() => self.last.as_ref().is_some_and(|(_, last)| !last.is_empty()).then_some(Send::Stop),
            Some((at, last)) if last == playing && now.duration_since(*at) < resend => None,
            _ => Some(Send::Play)
        };
        if send.is_some() {
            self.last = Some((now, playing.clone()));
        }
        send
    }

    pub fn clear(&mut self) {
        self.last = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full(_: Muscle) -> f64 {
        1.0
    }

    #[test]
    fn touch_holds_its_side_back_and_leaves_the_other_open() {
        let mut arbiter = Arbiter::default();
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let touch = Levels::from([(Muscle::PectoralL, 1.0)]);
        let asks = Levels::from([(Muscle::AbdominalR, 0.5), (Muscle::LumbarL, 0.5)]);

        let playing = arbiter.tick(&touch, &asks, full, PRIORITY, at(0)).clone();
        assert_eq!(playing, Levels::from([(Muscle::PectoralL, 1.0), (Muscle::LumbarL, 0.5)]), "one packet for both sides");
        assert_eq!(arbiter.touched(), [Side::Front]);

        // Touch lets go: the front stays quiet for the grace, then the viewer's ask comes back.
        assert_eq!(arbiter.tick(&Levels::new(), &asks, full, PRIORITY, at(1400)).len(), 1);
        let playing = arbiter.tick(&Levels::new(), &asks, full, PRIORITY, at(1600));
        assert_eq!(playing.len(), 2);
        assert!(arbiter.touched().is_empty());
    }

    #[test]
    fn strength_scales_each_muscle() {
        let mut arbiter = Arbiter::default();
        let strength = |muscle: Muscle| if muscle == Muscle::ArmL { 0.5 } else { 1.0 };
        let touch = Levels::from([(Muscle::ArmL, 0.8), (Muscle::ArmR, 0.8)]);
        let playing = arbiter.tick(&touch, &Levels::new(), strength, PRIORITY, Instant::now());
        assert_eq!(playing, &Levels::from([(Muscle::ArmL, 0.4), (Muscle::ArmR, 0.8)]));
    }

    #[test]
    fn leases_run_out_and_the_strongest_wins() {
        let start = Instant::now();
        let mut leases = Leases::default();
        leases.set("tap:a", Levels::from([(Muscle::DorsalL, 0.2), (Muscle::DorsalR, 0.6)]), start + Duration::from_millis(300));
        leases.set("automation:1", Levels::from([(Muscle::DorsalL, 0.5)]), start + Duration::from_secs(2));
        assert_eq!(leases.levels(start), Levels::from([(Muscle::DorsalL, 0.5), (Muscle::DorsalR, 0.6)]));
        assert_eq!(leases.levels(start + Duration::from_millis(300)), Levels::from([(Muscle::DorsalL, 0.5)]));
        leases.remove("automation:1");
        assert!(leases.levels(start).is_empty());
    }

    #[test]
    fn sends_on_change_and_before_it_runs_out() {
        let mut pace = Pace::default();
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let resend = Duration::from_millis(250);
        let chest = Levels::from([(Muscle::PectoralL, 1.0)]);
        let both = Levels::from([(Muscle::PectoralL, 1.0), (Muscle::LumbarL, 0.5)]);
        assert_eq!(pace.due(&Levels::new(), resend, at(0)), None, "nothing to stop yet");
        assert_eq!(pace.due(&chest, resend, at(0)), Some(Send::Play));
        assert_eq!(pace.due(&chest, resend, at(200)), None);
        assert_eq!(pace.due(&both, resend, at(220)), Some(Send::Play), "a change goes out at once");
        assert_eq!(pace.due(&both, resend, at(470)), Some(Send::Play), "and again before it runs out");
        assert_eq!(pace.due(&Levels::new(), resend, at(480)), Some(Send::Stop));
        assert_eq!(pace.due(&Levels::new(), resend, at(800)), None, "stopped once");
    }

    #[test]
    fn clearing_stops_and_forgets() {
        let mut arbiter = Arbiter::default();
        let now = Instant::now();
        arbiter.tick(&Levels::from([(Muscle::DorsalL, 1.0)]), &Levels::new(), full, PRIORITY, now);
        arbiter.clear();
        assert!(arbiter.touched().is_empty());
        assert!(arbiter.playing().is_empty());
    }
}
