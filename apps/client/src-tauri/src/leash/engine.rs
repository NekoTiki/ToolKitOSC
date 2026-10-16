//! Turns a pull on the leash into movement: which leashes the worn avatar has, which are held,
//! and what to send to VRChat's `/input/*` each tick. No `AppHandle`, so it can be tested alone.
//!
//! The maths and defaults are OSCLeash's (`Controllers/ThreadController.py`, v2.2.0), so a leash
//! feels the same as it does there:
//!
//! > MIT License
//! >
//! > Copyright (c) 2022 ZenithVal
//! >
//! > Permission is hereby granted, free of charge, to any person obtaining a copy of this software
//! > and associated documentation files (the "Software"), to deal in the Software without
//! > restriction, including without limitation the rights to use, copy, modify, merge, publish,
//! > distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the
//! > Software is furnished to do so, subject to the following conditions:
//! >
//! > The above copyright notice and this permission notice shall be included in all copies or
//! > substantial portions of the Software.
//! >
//! > THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING
//! > BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
//! > NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
//! > DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! > OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
//!
//! Two small changes: the final walk values are clamped to -1..1 again after the up/down
//! compensation, and a compensation that would flip your direction is skipped.
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use serde::Serialize;

use super::model::AvatarLeash;
use crate::types::AvatarParameter;

/// The compass contacts, shared by every leash on the avatar: `<prefix>_Z+` and so on.
pub const DIRECTIONS: [&str; 6] = ["Z+", "Z-", "X+", "X-", "Y+", "Y-"];
/// The compass prefix OSCLeash's prefab uses, picked first when several are found.
const COMPASS_PREFIX: &str = "Leash";

/// Stretch above which a pull counts for the setup checks.
const STRETCHED: f64 = 0.05;
/// Held this long without ever stretching: the leash can't stretch.
const STRETCH_WAIT: Duration = Duration::from_secs(3);
/// Stretched this long with every direction at 0: the compass doesn't see the needle.
const COMPASS_WAIT: Duration = Duration::from_secs(2);
/// How far forward/back or left/right has to swing during pulls before the needle counts as
/// following the leash.
const NEEDLE_SWING: f64 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Bool(bool),
    Number(f64)
}

/// What the worn avatar has.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detected {
    /// Every grabbable PhysBone with `_IsGrabbed` and `_Stretch`, whether or not it moves you.
    pub leashes: Vec<String>,
    /// The compass prefix, once all six directions are there.
    pub compass: Option<String>,
    pub problems: Vec<Problem>
}

/// A setup mistake the parameters show.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Problem {
    NoLeash,
    MissingStretch { leash: String },
    NoCompass,
    MissingDirections { missing: Vec<String> },
    WrongType { parameter: String, expected: &'static str, found: String }
}

fn kind_of<'a>(parameters: &'a [AvatarParameter], name: &str) -> Option<&'a str> {
    parameters
        .iter()
        .find(|p| p.name == name)
        .map(|p| p.output.as_ref().or(p.input.as_ref()).map_or("", |address| address.kind.as_str()))
}

/// The leashes and compass on an avatar, from its parameters.
pub fn detect(parameters: &[AvatarParameter]) -> Detected {
    let mut problems = Vec::new();
    let mut wrong_type = |parameter: &str, expected: &'static str| -> bool {
        match kind_of(parameters, parameter) {
            Some(found) if !found.is_empty() && found != expected => {
                problems.push(Problem::WrongType { parameter: parameter.to_owned(), expected, found: found.to_owned() });
                true
            }
            _ => false
        }
    };

    let mut names: Vec<&str> = parameters.iter().filter_map(|p| p.name.strip_suffix("_IsGrabbed")).collect();
    names.sort_unstable();
    names.dedup();
    let mut leashes = Vec::new();
    let mut missing_stretch = Vec::new();
    for name in names {
        let stretch = format!("{name}_Stretch");
        if kind_of(parameters, &stretch).is_none() {
            missing_stretch.push(name.to_owned());
            continue;
        }
        let bad_grab = wrong_type(&format!("{name}_IsGrabbed"), "Bool");
        let bad_stretch = wrong_type(&stretch, "Float");
        if !bad_grab && !bad_stretch {
            leashes.push(name.to_owned());
        }
    }

    // Prefixes with at least one direction, OSCLeash's first, then the most complete.
    let mut prefixes: BTreeMap<&str, usize> = BTreeMap::new();
    for parameter in parameters {
        for direction in DIRECTIONS {
            if let Some(prefix) = parameter.name.strip_suffix(direction).and_then(|rest| rest.strip_suffix('_')) {
                *prefixes.entry(prefix).or_default() += 1;
            }
        }
    }
    let prefix = if prefixes.contains_key(COMPASS_PREFIX) {
        Some(COMPASS_PREFIX)
    } else {
        prefixes.iter().max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0))).map(|(prefix, _)| *prefix)
    };

    let mut compass = None;
    if let Some(prefix) = prefix {
        let missing: Vec<String> = DIRECTIONS
            .iter()
            .map(|direction| format!("{prefix}_{direction}"))
            .filter(|name| kind_of(parameters, name).is_none())
            .collect();
        let mut bad = false;
        for direction in DIRECTIONS {
            bad |= wrong_type(&format!("{prefix}_{direction}"), "Float");
        }
        if !missing.is_empty() {
            problems.push(Problem::MissingDirections { missing });
        } else if !bad {
            compass = Some(prefix.to_owned());
        }
    }

    if leashes.is_empty() && missing_stretch.is_empty() && !problems.iter().any(|p| matches!(p, Problem::WrongType { .. })) {
        problems.insert(0, Problem::NoLeash);
    }
    for leash in missing_stretch {
        problems.push(Problem::MissingStretch { leash });
    }
    if prefix.is_none() && !leashes.is_empty() {
        problems.push(Problem::NoCompass);
    }

    Detected { leashes, compass, problems }
}

/// The compass contacts: how much of the pull goes each way, 0 to 1.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Directions {
    pub z_pos: f64,
    pub z_neg: f64,
    pub x_pos: f64,
    pub x_neg: f64,
    pub y_pos: f64,
    pub y_neg: f64
}

impl Directions {
    fn any(&self) -> bool {
        [self.z_pos, self.z_neg, self.x_pos, self.x_neg, self.y_pos, self.y_neg].iter().any(|v| *v > 0.0)
    }

    pub fn forward(&self) -> f64 {
        self.z_pos - self.z_neg
    }

    pub fn right(&self) -> f64 {
        self.x_pos - self.x_neg
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Motion {
    #[default]
    Idle,
    /// Held, but not pulled past the walk point.
    Grabbed,
    Walking,
    Running
}

/// What goes to `/input/*`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    pub vertical: f64,
    pub horizontal: f64,
    pub look: f64,
    pub run: bool,
    pub motion: Motion
}

impl Output {
    pub fn still(motion: Motion) -> Self {
        Self { motion, ..Self::default() }
    }

    /// Something to send: held, or moving.
    pub fn holds(&self) -> bool {
        self.motion != Motion::Idle
    }
}

/// Which way the leash hangs from you, from the end of its name (`Leash_North`), for turning.
/// A leash without one is in front.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hang {
    North,
    South,
    East,
    West
}

impl Hang {
    pub fn of(leash: &str) -> Self {
        match leash.rsplit('_').next() {
            Some("South") => Self::South,
            Some("East") => Self::East,
            Some("West") => Self::West,
            _ => Self::North
        }
    }
}

fn clamp(value: f64) -> f64 {
    value.clamp(-1.0, 1.0)
}

/// OSCLeash's maths, for one held leash.
pub fn movement(stretch: f64, d: Directions, s: &AvatarLeash, hang: Hang) -> Output {
    if stretch <= s.walk {
        return Output::still(Motion::Grabbed);
    }

    let mult = stretch * s.strength;
    let mut vertical = clamp(d.forward() * mult);
    let mut horizontal = clamp(d.right() * mult);

    let y = d.y_pos + d.y_neg;
    if y >= s.up_down_deadzone {
        vertical = 0.0;
        horizontal = 0.0;
    }
    if s.up_down_compensation != 0.0 {
        let modifier = clamp(1.0 - y * s.up_down_compensation);
        if modifier > 0.0 {
            vertical = clamp(vertical / modifier);
            horizontal = clamp(horizontal / modifier);
        }
    }

    let run = stretch > s.run;
    Output {
        vertical,
        horizontal,
        look: turning(vertical, horizontal, stretch, d, s, hang),
        run,
        motion: if run { Motion::Running } else { Motion::Walking }
    }
}

fn turning(vertical: f64, horizontal: f64, stretch: f64, d: Directions, s: &AvatarLeash, hang: Hang) -> f64 {
    if !s.turning.enabled || stretch <= s.turning.deadzone {
        return 0.0;
    }
    let goal = s.turning.goal_degrees / 180.0;
    let multiplier = s.turning.multiplier;
    let speed = match hang {
        Hang::North if d.z_pos < goal => multiplier * horizontal + if d.x_pos > d.x_neg { d.z_neg } else { -d.z_neg },
        Hang::South if d.z_neg < goal => multiplier * -horizontal + if d.x_pos > d.x_neg { -d.z_pos } else { d.z_pos },
        Hang::East if d.x_pos < goal => multiplier * vertical + if d.z_pos > d.z_neg { d.x_neg } else { -d.x_neg },
        Hang::West if d.x_neg < goal => multiplier * -vertical + if d.z_pos > d.z_neg { -d.x_pos } else { d.x_pos },
        _ => 0.0
    };
    clamp(speed)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Check {
    /// Needs someone to pull the leash.
    #[default]
    Waiting,
    Passed,
    Failed
}

/// The setup checks only a pull can answer. Kept for the avatar once passed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Checks {
    /// Pulling stretches the leash (Max stretch above 0).
    pub stretches: Check,
    /// A stretched leash moves at least one compass contact.
    pub compass: Check,
    /// The compass changes as the leash is pulled different ways.
    pub needle: Check
}

/// The current grab, for the checks.
#[derive(Debug, Default)]
struct Trial {
    since: Option<Instant>,
    stretched_since: Option<Instant>,
    max_stretch: f64,
    forward: Option<(f64, f64)>,
    right: Option<(f64, f64)>
}

fn widen(range: &mut Option<(f64, f64)>, value: f64) -> f64 {
    let (low, high) = range.get_or_insert((value, value));
    *low = low.min(value);
    *high = high.max(value);
    *high - *low
}

/// One tick: what's held and what to send.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Frame {
    pub held: Vec<String>,
    /// The held leash that moves you: the most stretched one that's on.
    pub active: Option<String>,
    pub stretch: f64,
    pub directions: Directions,
    pub output: Output
}

#[derive(Debug, Default)]
pub struct Engine {
    detected: Detected,
    /// `_IsGrabbed` as VRChat sent it live. Never read from saved values, so an avatar never loads
    /// walking.
    grabbed: BTreeMap<String, bool>,
    checks: Checks,
    trial: Trial
}

impl Engine {
    pub fn detected(&self) -> &Detected {
        &self.detected
    }

    pub fn checks(&self) -> Checks {
        self.checks
    }

    /// Another avatar: start over.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The worn avatar's leashes, once its parameters are known. Grabs already heard are kept.
    pub fn set_detected(&mut self, detected: Detected) {
        self.detected = detected;
        self.checks = Checks::default();
        self.trial = Trial::default();
    }

    /// Forgets every grab, so a leash moves you again only once grabbed again.
    pub fn let_go(&mut self) {
        self.grabbed.clear();
    }

    /// A live parameter. True when a grab started or ended.
    pub fn on_parameter(&mut self, name: &str, value: Value) -> bool {
        let Some(leash) = name.strip_suffix("_IsGrabbed") else {
            return false;
        };
        let on = match value {
            Value::Bool(on) => on,
            Value::Number(value) => value != 0.0
        };
        self.grabbed.insert(leash.to_owned(), on).unwrap_or(false) != on
    }

    /// `read` gives a parameter's current value by name.
    pub fn tick(&mut self, now: Instant, settings: &AvatarLeash, read: impl Fn(&str) -> Option<f64>) -> Frame {
        let directions = match &self.detected.compass {
            Some(prefix) => {
                let get = |direction: &str| read(&format!("{prefix}_{direction}")).unwrap_or(0.0).clamp(0.0, 1.0);
                Directions { z_pos: get("Z+"), z_neg: get("Z-"), x_pos: get("X+"), x_neg: get("X-"), y_pos: get("Y+"), y_neg: get("Y-") }
            }
            None => Directions::default()
        };

        let held: Vec<(String, f64)> = self
            .detected
            .leashes
            .iter()
            .filter(|leash| self.grabbed.get(*leash).copied().unwrap_or(false))
            .map(|leash| (leash.clone(), read(&format!("{leash}_Stretch")).unwrap_or(0.0).clamp(0.0, 1.0)))
            .collect();

        let most = held.iter().map(|(_, stretch)| *stretch).reduce(f64::max);
        self.observe(now, most, directions);

        let active = held.iter().filter(|(leash, _)| settings.moves_you(leash)).max_by(|a, b| a.1.total_cmp(&b.1)).cloned();
        let (stretch, output) = match &active {
            None => (most.unwrap_or(0.0), Output::default()),
            Some((leash, stretch)) if self.detected.compass.is_some() => (*stretch, movement(*stretch, directions, settings, Hang::of(leash))),
            Some((_, stretch)) => (*stretch, Output::still(Motion::Grabbed))
        };

        Frame { held: held.into_iter().map(|(leash, _)| leash).collect(), active: active.map(|(leash, _)| leash), stretch, directions, output }
    }

    fn observe(&mut self, now: Instant, stretch: Option<f64>, directions: Directions) {
        let Some(stretch) = stretch else {
            self.trial = Trial::default();
            return;
        };
        let trial = &mut self.trial;
        let since = *trial.since.get_or_insert(now);
        trial.max_stretch = trial.max_stretch.max(stretch);
        let stretched = stretch > STRETCHED;
        let stretched_since = if stretched { Some(*trial.stretched_since.get_or_insert(now)) } else { trial.stretched_since };

        if trial.max_stretch > STRETCHED {
            self.checks.stretches = Check::Passed;
        } else if self.checks.stretches != Check::Passed && now.duration_since(since) >= STRETCH_WAIT {
            self.checks.stretches = Check::Failed;
        }

        if self.detected.compass.is_none() {
            return;
        }
        if stretched && directions.any() {
            self.checks.compass = Check::Passed;
        } else if self.checks.compass != Check::Passed
            && !directions.any()
            && stretched_since.is_some_and(|at| now.duration_since(at) >= COMPASS_WAIT)
        {
            self.checks.compass = Check::Failed;
        }

        if stretched && directions.any() {
            let forward = widen(&mut trial.forward, directions.forward());
            let right = widen(&mut trial.right, directions.right());
            if forward >= NEEDLE_SWING || right >= NEEDLE_SWING {
                self.checks.needle = Check::Passed;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AvatarParameterAddress;
    use std::collections::HashMap;

    fn param(name: &str, kind: &str) -> AvatarParameter {
        let address = AvatarParameterAddress { address: format!("/avatar/parameters/{name}"), kind: kind.into() };
        AvatarParameter { name: name.into(), input: Some(address.clone()), output: Some(address) }
    }

    fn compass(prefix: &str) -> Vec<AvatarParameter> {
        DIRECTIONS.iter().map(|d| param(&format!("{prefix}_{d}"), "Float")).collect()
    }

    fn leash(name: &str) -> Vec<AvatarParameter> {
        vec![param(&format!("{name}_IsGrabbed"), "Bool"), param(&format!("{name}_Stretch"), "Float"), param(&format!("{name}_Angle"), "Float")]
    }

    fn avatar(parts: &[Vec<AvatarParameter>]) -> Vec<AvatarParameter> {
        parts.concat()
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn finds_leashes_and_the_compass() {
        let detected = detect(&avatar(&[leash("Leash"), leash("Tail"), compass("Leash")]));
        assert_eq!(detected.leashes, ["Leash", "Tail"]);
        assert_eq!(detected.compass.as_deref(), Some("Leash"));
        assert!(detected.problems.is_empty());
    }

    #[test]
    fn no_leash() {
        let detected = detect(&[param("VelocityZ", "Float")]);
        assert_eq!(detected.problems, [Problem::NoLeash]);
        assert!(detected.compass.is_none());
    }

    #[test]
    fn leash_without_compass_or_stretch() {
        let detected = detect(&avatar(&[leash("Leash"), vec![param("Collar_IsGrabbed", "Bool")]]));
        assert_eq!(detected.problems, [Problem::MissingStretch { leash: "Collar".into() }, Problem::NoCompass]);
    }

    #[test]
    fn names_missing_directions_and_wrong_types() {
        let mut parameters = avatar(&[leash("Leash"), compass("Leash")]);
        parameters.retain(|p| p.name != "Leash_X-");
        parameters.iter_mut().find(|p| p.name == "Leash_Stretch").unwrap().output.as_mut().unwrap().kind = "Bool".into();
        let detected = detect(&parameters);
        assert!(detected.leashes.is_empty());
        assert!(detected.compass.is_none());
        assert!(detected.problems.contains(&Problem::MissingDirections { missing: vec!["Leash_X-".into()] }));
        assert!(detected.problems.contains(&Problem::WrongType { parameter: "Leash_Stretch".into(), expected: "Float", found: "Bool".into() }));
        assert!(!detected.problems.contains(&Problem::NoLeash));
    }

    #[test]
    fn walks_like_the_concept_example() {
        let d = Directions { z_pos: 0.9, ..Directions::default() };
        let output = movement(0.40, d, &AvatarLeash::default(), Hang::North);
        assert!(near(output.vertical, 0.432));
        assert_eq!(output.horizontal, 0.0);
        assert!(!output.run);
        assert_eq!(output.motion, Motion::Walking);
    }

    #[test]
    fn runs_past_the_run_point() {
        let d = Directions { z_pos: 0.9, ..Directions::default() };
        let output = movement(0.82, d, &AvatarLeash::default(), Hang::North);
        assert!(output.run);
        assert_eq!(output.motion, Motion::Running);
        assert!(near(output.vertical, 0.8856));
    }

    #[test]
    fn stays_still_below_the_walk_point() {
        let d = Directions { z_pos: 1.0, ..Directions::default() };
        assert_eq!(movement(0.15, d, &AvatarLeash::default(), Hang::North), Output::still(Motion::Grabbed));
    }

    #[test]
    fn up_or_down_pulls_stop_you() {
        let d = Directions { z_pos: 0.5, y_pos: 0.5, ..Directions::default() };
        let output = movement(0.5, d, &AvatarLeash::default(), Hang::North);
        assert_eq!((output.vertical, output.horizontal), (0.0, 0.0));
        assert_eq!(output.motion, Motion::Walking);
    }

    #[test]
    fn up_down_compensation_strengthens_a_slanted_pull() {
        // 0.5 * 0.5 * 1.2 = 0.3, divided by 1 - 0.25.
        let d = Directions { z_pos: 0.5, y_neg: 0.25, ..Directions::default() };
        assert!(near(movement(0.5, d, &AvatarLeash::default(), Hang::North).vertical, 0.4));
    }

    #[test]
    fn sideways_and_backwards() {
        let d = Directions { x_neg: 0.6, z_neg: 0.2, ..Directions::default() };
        let output = movement(0.5, d, &AvatarLeash::default(), Hang::North);
        assert!(near(output.horizontal, -0.36));
        assert!(near(output.vertical, -0.12));
    }

    #[test]
    fn turning_only_when_on() {
        let d = Directions { x_pos: 0.7, z_neg: 0.3, ..Directions::default() };
        let mut settings = AvatarLeash::default();
        assert_eq!(movement(0.5, d, &settings, Hang::North).look, 0.0);
        settings.turning.enabled = true;
        // 0.8 * (0.7 * 0.6) + 0.3
        assert!(near(movement(0.5, d, &settings, Hang::North).look, 0.636));
        assert_eq!(Hang::of("Leash_West"), Hang::West);
        assert_eq!(Hang::of("Leash"), Hang::North);
    }

    fn values(pairs: &[(&str, f64)]) -> HashMap<String, f64> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    fn engine() -> Engine {
        let mut engine = Engine::default();
        engine.set_detected(detect(&avatar(&[leash("Leash"), leash("Tail"), compass("Leash")])));
        engine
    }

    #[test]
    fn only_a_live_grab_moves_you() {
        let mut engine = engine();
        let read = values(&[("Leash_Stretch", 0.5), ("Leash_Z+", 1.0)]);
        let now = Instant::now();
        let frame = engine.tick(now, &AvatarLeash::default(), |n| read.get(n).copied());
        assert_eq!(frame.output.motion, Motion::Idle);

        assert!(engine.on_parameter("Leash_IsGrabbed", Value::Bool(true)));
        assert!(!engine.on_parameter("Leash_IsGrabbed", Value::Bool(true)));
        let frame = engine.tick(now, &AvatarLeash::default(), |n| read.get(n).copied());
        assert_eq!(frame.active.as_deref(), Some("Leash"));
        assert_eq!(frame.output.motion, Motion::Walking);

        engine.let_go();
        assert_eq!(engine.tick(now, &AvatarLeash::default(), |n| read.get(n).copied()).output.motion, Motion::Idle);
    }

    #[test]
    fn a_tail_doesnt_move_you_unless_picked() {
        let mut engine = engine();
        let read = values(&[("Tail_Stretch", 0.9), ("Leash_Z+", 1.0)]);
        engine.on_parameter("Tail_IsGrabbed", Value::Bool(true));
        let mut settings = AvatarLeash::default();
        let frame = engine.tick(Instant::now(), &settings, |n| read.get(n).copied());
        assert_eq!(frame.held, ["Tail"]);
        assert_eq!(frame.active, None);
        assert_eq!(frame.output.motion, Motion::Idle);

        settings.leashes.insert("Tail".into(), true);
        assert_eq!(engine.tick(Instant::now(), &settings, |n| read.get(n).copied()).output.motion, Motion::Running);
    }

    #[test]
    fn the_most_stretched_leash_wins() {
        let mut engine = Engine::default();
        engine.set_detected(detect(&avatar(&[leash("Leash_North"), leash("Leash_South"), compass("Leash")])));
        engine.on_parameter("Leash_North_IsGrabbed", Value::Bool(true));
        engine.on_parameter("Leash_South_IsGrabbed", Value::Number(1.0));
        let read = values(&[("Leash_North_Stretch", 0.3), ("Leash_South_Stretch", 0.6), ("Leash_Z-", 1.0)]);
        let frame = engine.tick(Instant::now(), &AvatarLeash::default(), |n| read.get(n).copied());
        assert_eq!(frame.active.as_deref(), Some("Leash_South"));
        assert_eq!(frame.stretch, 0.6);
    }

    #[test]
    fn checks_wait_for_a_pull_then_pass_or_fail() {
        let mut engine = engine();
        let start = Instant::now();
        engine.on_parameter("Leash_IsGrabbed", Value::Bool(true));

        // Held without stretching, then stretched with no compass contact.
        let none = values(&[]);
        engine.tick(start, &AvatarLeash::default(), |n| none.get(n).copied());
        assert_eq!(engine.checks(), Checks::default());
        engine.tick(start + Duration::from_secs(3), &AvatarLeash::default(), |n| none.get(n).copied());
        assert_eq!(engine.checks().stretches, Check::Failed);

        let stretched = values(&[("Leash_Stretch", 0.5)]);
        engine.tick(start + Duration::from_secs(4), &AvatarLeash::default(), |n| stretched.get(n).copied());
        engine.tick(start + Duration::from_secs(6), &AvatarLeash::default(), |n| stretched.get(n).copied());
        assert_eq!(engine.checks().stretches, Check::Passed);
        assert_eq!(engine.checks().compass, Check::Failed);

        // Pulled forward, then to the side.
        let forward = values(&[("Leash_Stretch", 0.5), ("Leash_Z+", 0.8)]);
        let side = values(&[("Leash_Stretch", 0.5), ("Leash_X+", 0.8)]);
        engine.tick(start + Duration::from_secs(7), &AvatarLeash::default(), |n| forward.get(n).copied());
        assert_eq!(engine.checks().compass, Check::Passed);
        assert_eq!(engine.checks().needle, Check::Waiting);
        engine.tick(start + Duration::from_secs(8), &AvatarLeash::default(), |n| side.get(n).copied());
        assert_eq!(engine.checks().needle, Check::Passed);
    }
}
