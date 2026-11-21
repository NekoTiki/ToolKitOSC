//! Who wants an effect on right now. Every source holds its own effect and strength; the newest
//! hold plays, and when it ends the next newest still held takes over. Never two at once.
use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::model::Effect;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "kebab-case")]
pub enum Source {
    /// Turn on myself, on the page.
    You,
    /// `TKOSC/VoiceFX`.
    Avatar,
    /// The mouth socket link.
    Mouth,
    /// An automation step. They share one hold.
    Automation,
    /// A control, by its id: you or a viewer.
    Control(String)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hold {
    pub source: Source,
    /// The viewer or automation behind it, for the page.
    pub by: Option<String>,
    /// `None`: the default effect.
    pub effect: Option<Effect>,
    pub level: f64,
    pub until: Option<Instant>,
    seq: u64
}

#[derive(Debug, Default)]
pub struct Holds {
    list: Vec<Hold>,
    seq: u64
}

impl Holds {
    /// Starts a hold, or starts this source's hold again: either way it's now the newest.
    pub fn hold(&mut self, source: Source, by: Option<String>, effect: Option<Effect>, level: f64, until: Option<Instant>) {
        self.list.retain(|hold| hold.source != source);
        self.seq += 1;
        self.list.push(Hold { source, by, effect, level: level.clamp(0.0, 1.0), until, seq: self.seq });
    }

    /// Changes what a hold plays without making it the newest. False when it isn't held.
    pub fn adjust(&mut self, source: &Source, effect: Option<Effect>, level: f64) -> bool {
        match self.list.iter_mut().find(|hold| hold.source == *source) {
            Some(hold) => {
                hold.effect = effect;
                hold.level = level.clamp(0.0, 1.0);
                true
            }
            None => false
        }
    }

    pub fn release(&mut self, source: &Source) -> bool {
        let before = self.list.len();
        self.list.retain(|hold| hold.source != *source);
        self.list.len() != before
    }

    pub fn is_held(&self, source: &Source) -> bool {
        self.list.iter().any(|hold| hold.source == *source)
    }

    /// Ends the timed holds that are over.
    pub fn expire(&mut self, now: Instant) {
        self.list.retain(|hold| hold.until.map_or(true, |until| now < until));
    }

    pub fn clear(&mut self) {
        self.list.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn playing(&self) -> Option<&Hold> {
        self.list.iter().max_by_key(|hold| hold.seq)
    }

    /// The one playing first, then the ones waiting, in the order they'd take over.
    pub fn newest_first(&self) -> Vec<&Hold> {
        let mut holds: Vec<&Hold> = self.list.iter().collect();
        holds.sort_by_key(|hold| std::cmp::Reverse(hold.seq));
        holds
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn control(id: &str) -> Source {
        Source::Control(id.into())
    }

    #[test]
    fn the_newest_hold_plays_and_the_next_takes_over() {
        let mut holds = Holds::default();
        holds.hold(Source::Avatar, None, None, 0.7, None);
        holds.hold(control("a"), Some("Mika".into()), Some(Effect::Radio), 0.5, None);
        assert_eq!(holds.playing().unwrap().source, control("a"));
        assert!(holds.release(&control("a")));
        assert_eq!(holds.playing().unwrap().source, Source::Avatar);
        assert!(holds.release(&Source::Avatar));
        assert!(holds.playing().is_none());
        assert!(!holds.release(&Source::Avatar));
    }

    #[test]
    fn holding_again_makes_it_the_newest_but_adjusting_doesnt() {
        let mut holds = Holds::default();
        holds.hold(Source::Mouth, None, Some(Effect::Muffled), 0.3, None);
        holds.hold(Source::You, None, None, 0.7, None);
        assert!(holds.adjust(&Source::Mouth, Some(Effect::Muffled), 0.9));
        assert_eq!(holds.playing().unwrap().source, Source::You);
        assert_eq!(holds.newest_first()[1].level, 0.9);
        holds.hold(Source::Mouth, None, Some(Effect::Muffled), 0.4, None);
        assert_eq!(holds.playing().unwrap().source, Source::Mouth);
        assert_eq!(holds.newest_first().len(), 2);
        assert!(!holds.adjust(&Source::Avatar, None, 1.0));
    }

    #[test]
    fn timed_holds_end_on_their_own() {
        let mut holds = Holds::default();
        let now = Instant::now();
        holds.hold(Source::Avatar, None, None, 0.7, None);
        holds.hold(control("dunk"), None, Some(Effect::Underwater), 0.8, Some(now + Duration::from_secs(20)));
        holds.expire(now + Duration::from_secs(19));
        assert_eq!(holds.playing().unwrap().effect, Some(Effect::Underwater));
        holds.expire(now + Duration::from_secs(20));
        assert_eq!(holds.playing().unwrap().source, Source::Avatar);
    }

    #[test]
    fn levels_stay_within_range_and_clear_ends_everything() {
        let mut holds = Holds::default();
        holds.hold(Source::You, None, None, 3.0, None);
        assert_eq!(holds.playing().unwrap().level, 1.0);
        holds.clear();
        assert!(holds.is_empty());
    }
}
