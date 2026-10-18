//! Reads the contact receivers vrc-owo-suit's Unity package (github.com/shadorki/vrc-owo-suit) puts
//! on an avatar: one bool per muscle group, true while any contact overlaps it, and an optional
//! `owo_intensity` radial. Only the names are used, no code: that repository has no licence.
//!
//! The receivers don't tell your own hands from other people's, so there's no choice of whose
//! touch plays, unlike bHaptics.
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::{Duration, Instant};

use serde::Serialize;

use super::model::Muscle;

/// A muscle held on this long without changing is taken as stuck (an avatar left mid-touch, a
/// contact resting on a receiver) and ignored until it goes off.
pub const STUCK_AFTER: Duration = Duration::from_secs(30);
pub const INTENSITY_PARAMETER: &str = "owo_intensity";

/// Each muscle's level, 0 to 1. Muscles at 0 may be left out.
pub type Levels = BTreeMap<Muscle, f64>;

/// A parameter name (without `/avatar/parameters/`) as a muscle, if it is one.
pub fn parse(name: &str) -> Option<Muscle> {
    Muscle::ALL.into_iter().find(|muscle| muscle.parameter() == name)
}

/// What the worn avatar has for the OWO Skin, from its parameter names.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detected {
    /// The muscles it has a receiver for.
    pub muscles: Vec<Muscle>,
    /// Whether it has `owo_intensity`.
    pub intensity: bool,
    /// SPS, OGB or VRCFury Haptics zones.
    pub sps_zones: usize,
    /// Whether it has `TKOSC/OWO`.
    pub switch: bool
}

pub fn detect<'a>(names: impl IntoIterator<Item = &'a str>, switch_parameter: &str) -> Detected {
    let mut muscles = BTreeSet::new();
    let mut zones = BTreeSet::new();
    let mut detected = Detected::default();
    for name in names {
        detected.switch |= name == switch_parameter;
        detected.intensity |= name == INTENSITY_PARAMETER;
        if let Some((zone, _)) = crate::haptics::zones::parse(name) {
            zones.insert(zone);
        }
        muscles.extend(parse(name));
    }
    detected.muscles = muscles.into_iter().collect();
    detected.sps_zones = zones.len();
    detected
}

/// Every muscle the avatar says is touched now, and since when.
#[derive(Debug, Default)]
pub struct Touch {
    held: HashMap<Muscle, Instant>
}

impl Touch {
    /// A live parameter. False when it isn't a muscle.
    pub fn on_parameter(&mut self, name: &str, on: bool, now: Instant) -> bool {
        let Some(muscle) = parse(name) else {
            return false;
        };
        if on {
            self.held.entry(muscle).or_insert(now);
        } else {
            self.held.remove(&muscle);
        }
        true
    }

    /// Each touched muscle at `level`. Stuck ones count as off.
    pub fn levels(&self, level: f64, now: Instant) -> Levels {
        self.held.iter().filter(|(_, since)| now.duration_since(**since) < STUCK_AFTER).map(|(muscle, _)| (*muscle, level)).collect()
    }

    pub fn clear(&mut self) {
        self.held.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_vrc_owo_suits_names() {
        assert_eq!(parse("owo_suit_Pectoral_L"), Some(Muscle::PectoralL));
        assert_eq!(parse("owo_suit_Lumbar_R"), Some(Muscle::LumbarR));
        assert_eq!(parse("owo_suit_Tail"), None);
        assert_eq!(parse("owo_intensity"), None);
    }

    #[test]
    fn a_hand_on_the_lower_back() {
        let now = Instant::now();
        let mut touch = Touch::default();
        touch.on_parameter("owo_suit_Lumbar_L", true, now);
        touch.on_parameter("owo_suit_Lumbar_R", true, now);
        touch.on_parameter("owo_suit_Lumbar_L", false, now);
        assert_eq!(touch.levels(0.5, now), Levels::from([(Muscle::LumbarR, 0.5)]));
    }

    #[test]
    fn a_stuck_muscle_is_ignored_until_it_lets_go() {
        let start = Instant::now();
        let at = |s: u64| start + Duration::from_secs(s);
        let mut touch = Touch::default();
        touch.on_parameter("owo_suit_Arm_R", true, start);
        touch.on_parameter("owo_suit_Arm_R", true, at(20));
        assert!(touch.levels(1.0, at(29)).contains_key(&Muscle::ArmR), "still true is no change");
        assert!(touch.levels(1.0, at(30)).is_empty());
        touch.on_parameter("owo_suit_Arm_R", false, at(40));
        touch.on_parameter("owo_suit_Arm_R", true, at(41));
        assert!(!touch.levels(1.0, at(42)).is_empty());
    }

    #[test]
    fn checks_what_the_avatar_has() {
        let names = ["owo_suit_Pectoral_L", "owo_suit_Pectoral_R", "owo_intensity", "OGB/Touch/Chest/Others", "TKOSC/OWO", "Go/Float"];
        let detected = detect(names, "TKOSC/OWO");
        assert_eq!(detected.muscles, [Muscle::PectoralR, Muscle::PectoralL]);
        assert!(detected.intensity && detected.switch);
        assert_eq!(detected.sps_zones, 1);
        assert_eq!(detect(["Go/Float"], "TKOSC/OWO"), Detected::default());
    }
}
