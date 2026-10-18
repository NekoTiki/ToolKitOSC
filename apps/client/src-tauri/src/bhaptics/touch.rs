//! Reads the touch parameters bHaptics' VRChat package puts on an avatar: one bool per motor,
//! true while a contact overlaps that motor's receiver.
//!
//! - Current package (github.com/bhaptics/VRChatOSC): `bOSC/v2/<Device>/<n>/self` and `…/others`,
//!   `bOSC/v2m/…` on the Quest-compatible prefabs. `n` starts at 0. The receivers name the
//!   devices like Player does: `Head`, `VestFront`, `VestBack`, `ForearmL`, `HandL`, `GloveL`,
//!   `FootL` and the `R` ones (`ArmLeft`, `HandLeft`… are read too).
//! - The original bHapticsOSC app's avatars: `bHaptics_<Device>_<n>_bool` and
//!   `bHapticsOSC_<Device>_<n>_int` (0–100), with `Vest_Front`, `Arm_Left`… and `n` from 1.
//!   That app numbered the vest's front rows, the head and the right foot the other way round,
//!   so those are flipped here to land on the same motors.
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::{Duration, Instant};

use serde::Serialize;

use super::model::{Position, TouchFrom};

/// A motor held on this long without changing is taken as stuck (an avatar left mid-touch, a
/// contact resting on a receiver) and ignored until it goes off.
pub const STUCK_AFTER: Duration = Duration::from_secs(30);

/// Each position's motors, 0 to 1.
pub type Levels = BTreeMap<Position, Vec<f64>>;

/// Who a contact belongs to. The old avatars don't say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Who {
    Own,
    Others,
    Either
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Contact {
    pub position: Position,
    pub motor: usize,
    pub who: Who
}

/// Which package a parameter comes from, for the avatar check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Scheme {
    #[serde(rename = "v2")]
    V2,
    #[serde(rename = "v2m")]
    V2Mobile,
    #[serde(rename = "legacy")]
    Legacy
}

fn v2_device(name: &str) -> Option<Position> {
    Some(match name {
        "Head" => Position::Head,
        "VestFront" => Position::VestFront,
        "VestBack" => Position::VestBack,
        "ForearmL" | "ArmLeft" => Position::ForearmL,
        "ForearmR" | "ArmRight" => Position::ForearmR,
        "HandL" | "HandLeft" => Position::HandL,
        "HandR" | "HandRight" => Position::HandR,
        "GloveL" | "GloveLeft" => Position::GloveL,
        "GloveR" | "GloveRight" => Position::GloveR,
        "FootL" | "FootLeft" => Position::FootL,
        "FootR" | "FootRight" => Position::FootR,
        _ => return None
    })
}

const LEGACY_DEVICES: [(&str, Position); 9] = [
    ("Head", Position::Head),
    ("Vest_Front", Position::VestFront),
    ("Vest_Back", Position::VestBack),
    ("Arm_Left", Position::ForearmL),
    ("Arm_Right", Position::ForearmR),
    ("Hand_Left", Position::HandL),
    ("Hand_Right", Position::HandR),
    ("Foot_Left", Position::FootL),
    ("Foot_Right", Position::FootR)
];

/// The old app's node `n` (from 1) as a motor index.
fn legacy_motor(position: Position, n: usize) -> Option<usize> {
    let count = position.motors();
    if n == 0 || n > count {
        return None;
    }
    let i = n - 1;
    Some(match position {
        Position::VestFront => i / 4 * 4 + (3 - i % 4),
        Position::Head | Position::FootR => count - 1 - i,
        _ => i
    })
}

/// Whether the value is a 0–100 intensity rather than an on/off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    OnOff,
    Intensity
}

/// A parameter name (without `/avatar/parameters/`) as a motor, if it is one.
pub fn parse(name: &str) -> Option<(Contact, Kind, Scheme)> {
    if let Some(rest) = name.strip_prefix("bOSC/") {
        let mut parts = rest.split('/');
        let scheme = match parts.next()? {
            "v2" => Scheme::V2,
            "v2m" => Scheme::V2Mobile,
            _ => return None
        };
        let position = v2_device(parts.next()?)?;
        let motor: usize = parts.next()?.parse().ok()?;
        let who = match parts.next()? {
            "self" => Who::Own,
            "others" => Who::Others,
            _ => return None
        };
        if parts.next().is_some() || motor >= position.motors() {
            return None;
        }
        return Some((Contact { position, motor, who }, Kind::OnOff, scheme));
    }

    let (rest, kind) = if let Some(rest) = name.strip_prefix("bHapticsOSC_").and_then(|r| r.strip_suffix("_int")) {
        (rest, Kind::Intensity)
    } else if let Some(rest) = name.strip_prefix("bHaptics_").and_then(|r| r.strip_suffix("_bool")) {
        (rest, Kind::OnOff)
    } else {
        return None;
    };
    let (device, n) = rest.rsplit_once('_')?;
    let position = LEGACY_DEVICES.iter().find(|(name, _)| *name == device)?.1;
    let motor = legacy_motor(position, n.parse().ok()?)?;
    Some((Contact { position, motor, who: Who::Either }, kind, Scheme::Legacy))
}

/// One device's receivers on the worn avatar.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCheck {
    pub position: Position,
    /// Motors the avatar has a receiver for.
    pub receivers: usize,
    pub motors: usize,
    /// Whether it tells your own hands and other people's apart (the current package does).
    pub own: bool,
    pub others: bool
}

/// What the worn avatar has for bHaptics, from its parameter names.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detected {
    pub schemes: Vec<Scheme>,
    pub devices: Vec<DeviceCheck>,
    /// SPS, OGB or VRCFury Haptics zones, which could drive the vest instead.
    pub sps_zones: usize,
    /// Whether it has `TKOSC/bHaptics`.
    pub switch: bool
}

pub fn detect<'a>(names: impl IntoIterator<Item = &'a str>, switch_parameter: &str) -> Detected {
    let mut schemes = BTreeSet::new();
    let mut motors: BTreeMap<Position, (BTreeSet<usize>, bool, bool)> = BTreeMap::new();
    let mut zones = BTreeSet::new();
    let mut switch = false;
    for name in names {
        if name == switch_parameter {
            switch = true;
        }
        if let Some((zone, _)) = crate::haptics::zones::parse(name) {
            zones.insert(zone);
        }
        let Some((contact, _, scheme)) = parse(name) else {
            continue;
        };
        schemes.insert(scheme);
        let entry = motors.entry(contact.position).or_default();
        entry.0.insert(contact.motor);
        entry.1 |= contact.who == Who::Own;
        entry.2 |= contact.who == Who::Others;
    }
    Detected {
        schemes: schemes.into_iter().collect(),
        devices: motors
            .into_iter()
            .map(|(position, (found, own, others))| DeviceCheck { position, receivers: found.len(), motors: position.motors(), own, others })
            .collect(),
        sps_zones: zones.len(),
        switch
    }
}

#[derive(Debug, Clone, Copy)]
struct Held {
    level: f64,
    since: Instant
}

/// Every motor the avatar says is touched now.
#[derive(Debug, Default)]
pub struct Touch {
    held: HashMap<Contact, Held>
}

impl Touch {
    /// A live parameter. False when it isn't a bHaptics one.
    pub fn on_parameter(&mut self, name: &str, value: f64, now: Instant) -> bool {
        let Some((contact, kind, _)) = parse(name) else {
            return false;
        };
        let level = match kind {
            Kind::OnOff => if value != 0.0 { 1.0 } else { 0.0 },
            Kind::Intensity => (value / 100.0).clamp(0.0, 1.0)
        };
        if level <= 0.0 {
            self.held.remove(&contact);
        } else {
            let held = self.held.entry(contact).or_insert(Held { level, since: now });
            held.level = level;
        }
        true
    }

    /// Each position's motors now, from the hands `from` allows. Stuck ones count as off.
    pub fn levels(&self, from: TouchFrom, now: Instant) -> Levels {
        let mut levels = Levels::new();
        for (contact, held) in &self.held {
            let allowed = match contact.who {
                Who::Own => from.own,
                Who::Others => from.others,
                Who::Either => from.own || from.others
            };
            if !allowed || now.duration_since(held.since) >= STUCK_AFTER {
                continue;
            }
            let motors = levels.entry(contact.position).or_insert_with(|| vec![0.0; contact.position.motors()]);
            motors[contact.motor] = motors[contact.motor].max(held.level);
        }
        levels
    }

    pub fn clear(&mut self) {
        self.held.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OTHERS: TouchFrom = TouchFrom { own: false, others: true };
    const BOTH: TouchFrom = TouchFrom { own: true, others: true };

    fn contact(position: Position, motor: usize, who: Who) -> Contact {
        Contact { position, motor, who }
    }

    #[test]
    fn reads_the_current_package() {
        assert_eq!(parse("bOSC/v2/VestFront/6/others"), Some((contact(Position::VestFront, 6, Who::Others), Kind::OnOff, Scheme::V2)));
        assert_eq!(parse("bOSC/v2m/ArmLeft/0/self"), Some((contact(Position::ForearmL, 0, Who::Own), Kind::OnOff, Scheme::V2Mobile)));
        assert_eq!(parse("bOSC/v2/GloveRight/7/others").map(|p| p.0.position), Some(Position::GloveR));
        assert_eq!(parse("bOSC/v2/ForearmL/0/others"), Some((contact(Position::ForearmL, 0, Who::Others), Kind::OnOff, Scheme::V2)), "the prefabs' own names");
        assert_eq!(parse("bOSC/v2/GloveR/7/self").map(|p| p.0), Some(contact(Position::GloveR, 7, Who::Own)));
        assert_eq!(parse("bOSC/v2m/FootL/2/self").map(|p| p.0.position), Some(Position::FootL));
        assert_eq!(parse("bOSC/v2/VestFront/20/others"), None, "a vest side has 20 motors");
        assert_eq!(parse("bOSC/v2/Tail/0/others"), None);
        assert_eq!(parse("bOSC/v2/Head/0/everyone"), None);
        assert_eq!(parse("OGB/Touch/Chest/Others"), None);
    }

    #[test]
    fn reads_the_old_app_and_flips_what_it_numbered_backwards() {
        // Front rows were right to left: node 1 is the top row's rightmost motor.
        assert_eq!(parse("bHaptics_Vest_Front_1_bool").map(|p| p.0), Some(contact(Position::VestFront, 3, Who::Either)));
        assert_eq!(parse("bHaptics_Vest_Front_5_bool").map(|p| p.0.motor), Some(7));
        assert_eq!(parse("bHaptics_Vest_Front_20_bool").map(|p| p.0.motor), Some(16));
        assert_eq!(parse("bHaptics_Vest_Back_1_bool").map(|p| p.0.motor), Some(0));
        assert_eq!(parse("bHaptics_Head_1_bool").map(|p| p.0.motor), Some(5));
        assert_eq!(parse("bHaptics_Foot_Right_1_bool").map(|p| p.0.motor), Some(2));
        assert_eq!(parse("bHaptics_Foot_Left_1_bool").map(|p| p.0.motor), Some(0));
        assert_eq!(parse("bHapticsOSC_Arm_Right_6_int"), Some((contact(Position::ForearmR, 5, Who::Either), Kind::Intensity, Scheme::Legacy)));
        assert_eq!(parse("bHaptics_Head_0_bool"), None, "the old app counted from 1");
        assert_eq!(parse("bHaptics_Head_7_bool"), None);
    }

    #[test]
    fn a_hand_sliding_down_the_chest() {
        let start = Instant::now();
        let mut touch = Touch::default();
        touch.on_parameter("bOSC/v2/VestFront/1/others", 1.0, start);
        touch.on_parameter("bOSC/v2/VestFront/5/others", 1.0, start);
        touch.on_parameter("bOSC/v2/VestFront/1/others", 0.0, start);
        touch.on_parameter("bOSC/v2/VestFront/9/others", 1.0, start);
        let front = &touch.levels(OTHERS, start)[&Position::VestFront];
        assert_eq!(front.iter().enumerate().filter(|(_, l)| **l > 0.0).map(|(i, _)| i).collect::<Vec<_>>(), [5, 9]);
    }

    #[test]
    fn own_hands_only_when_asked() {
        let now = Instant::now();
        let mut touch = Touch::default();
        touch.on_parameter("bOSC/v2/Head/2/self", 1.0, now);
        assert!(touch.levels(OTHERS, now).is_empty());
        assert_eq!(touch.levels(BOTH, now)[&Position::Head][2], 1.0);
    }

    #[test]
    fn a_hug_lights_both_sides() {
        let now = Instant::now();
        let mut touch = Touch::default();
        for motor in [4, 5, 6, 7] {
            touch.on_parameter(&format!("bOSC/v2/VestFront/{motor}/others"), 1.0, now);
            touch.on_parameter(&format!("bOSC/v2/VestBack/{motor}/others"), 1.0, now);
        }
        let levels = touch.levels(OTHERS, now);
        assert_eq!(levels.len(), 2);
        assert_eq!(levels[&Position::VestBack].iter().sum::<f64>(), 4.0);
    }

    #[test]
    fn a_stuck_motor_is_ignored_until_it_lets_go() {
        let start = Instant::now();
        let at = |s: u64| start + Duration::from_secs(s);
        let mut touch = Touch::default();
        touch.on_parameter("bOSC/v2/VestBack/0/others", 1.0, start);
        assert!(!touch.levels(OTHERS, at(29)).is_empty());
        assert!(touch.levels(OTHERS, at(30)).is_empty());
        touch.on_parameter("bOSC/v2/VestBack/0/others", 0.0, at(40));
        touch.on_parameter("bOSC/v2/VestBack/0/others", 1.0, at(41));
        assert!(!touch.levels(OTHERS, at(42)).is_empty());
    }

    #[test]
    fn intensity_from_the_old_app() {
        let now = Instant::now();
        let mut touch = Touch::default();
        touch.on_parameter("bHapticsOSC_Vest_Back_2_int", 40.0, now);
        assert_eq!(touch.levels(OTHERS, now)[&Position::VestBack][1], 0.4);
    }

    #[test]
    fn checks_what_the_avatar_has() {
        let mut names: Vec<String> = (0..20).map(|n| format!("bOSC/v2/VestFront/{n}/others")).collect();
        names.extend((0..19).map(|n| format!("bOSC/v2/VestBack/{n}/self")));
        names.push("bHaptics_Head_1_bool".into());
        names.push("OGB/Touch/Chest/Others".into());
        names.push("OGB/Touch/Chest/Self".into());
        names.push("TKOSC/bHaptics".into());
        let detected = detect(names.iter().map(String::as_str), "TKOSC/bHaptics");
        assert_eq!(detected.schemes, [Scheme::V2, Scheme::Legacy]);
        assert_eq!(detected.devices.len(), 3);
        assert_eq!(detected.devices[0], DeviceCheck { position: Position::Head, receivers: 1, motors: 6, own: false, others: false });
        assert_eq!(detected.devices[1], DeviceCheck { position: Position::VestFront, receivers: 20, motors: 20, own: false, others: true });
        assert_eq!(detected.devices[2].receivers, 19);
        assert_eq!(detected.sps_zones, 1);
        assert!(detected.switch);
        assert_eq!(detect(["Go/Float"], "TKOSC/bHaptics"), Detected::default());
    }

    #[test]
    fn an_avatar_change_mid_touch_clears() {
        let now = Instant::now();
        let mut touch = Touch::default();
        touch.on_parameter("bOSC/v2/VestFront/0/others", 1.0, now);
        touch.clear();
        assert!(touch.levels(OTHERS, now).is_empty());
    }
}
