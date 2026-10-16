//! Decides what each actuator gets and sends it to Intiface: one ScalarCmd per device, only for
//! devices it has, and only when a level actually changed since it was last sent.
//!
//! Controls, patterns and automations ask for a level, and among them the last one wins. Avatar
//! haptics come first: while touch drives an actuator (above the threshold, and for a grace period
//! after), what everyone else asks for is kept but held back. When touch lets go, the actuator gets
//! whatever they ask for now, so a pattern picks up where its clock has got to.
use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::intiface::{self, protocol};
use crate::state::AppState;

/// An actuator a control drives, as the webview stores it (see `IntifaceActuatorRef`). Its type
/// has to match what the device reports for that index, or Intiface rejects the whole command.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActuatorRef {
    pub device_index: u32,
    pub actuator_index: u32,
    pub actuator_type: String
}

impl ActuatorRef {
    fn key(&self) -> Key {
        (self.device_index, self.actuator_index)
    }
}

type Key = (u32, u32);

/// One actuator touch is driving or may drive, as `haptics` reports it each tick.
#[derive(Debug, Clone)]
pub struct Touch {
    pub actuator: ActuatorRef,
    pub level: f64
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Priority {
    pub threshold: f64,
    pub grace: Duration
}

#[derive(Debug, Default)]
struct TouchSlot {
    level: f64,
    /// Touch keeps the actuator until then, even below the threshold.
    until: Option<Instant>,
    owns: bool
}

/// Each actuator's state, without the sending, so it can be tested.
#[derive(Debug, Default)]
pub(super) struct Arbiter {
    /// What controls, patterns and automations last asked for.
    requested: HashMap<Key, f64>,
    /// The actuators haptics has sources for.
    touch: HashMap<Key, TouchSlot>,
    sent: HashMap<Key, f64>,
    types: HashMap<Key, String>
}

/// A level to send now.
type Send = (Key, f64);

impl Arbiter {
    fn out(&mut self, key: Key, level: f64) -> Option<Send> {
        if self.sent.get(&key) == Some(&level) {
            return None;
        }
        self.sent.insert(key, level);
        Some((key, level))
    }

    fn owned(&self, key: &Key) -> bool {
        self.touch.get(key).is_some_and(|slot| slot.owns)
    }

    /// A control, pattern or automation asks for `level`. Held back while touch owns it.
    pub(super) fn request(&mut self, actuator: &ActuatorRef, level: f64) -> Option<Send> {
        let key = actuator.key();
        self.types.insert(key, actuator.actuator_type.clone());
        self.requested.insert(key, level);
        if self.owned(&key) { None } else { self.out(key, level) }
    }

    /// Haptics' levels this tick, for every actuator it has sources for. One left out isn't
    /// touch's any more.
    pub(super) fn touch(&mut self, touches: &[Touch], priority: Priority, now: Instant) -> Vec<Send> {
        let mut sends = Vec::new();

        let gone: Vec<Key> = self.touch.keys().filter(|key| !touches.iter().any(|t| t.actuator.key() == **key)).copied().collect();
        for key in gone {
            if self.touch.remove(&key).is_some_and(|slot| slot.owns) {
                let level = self.requested.get(&key).copied().unwrap_or(0.0);
                sends.extend(self.out(key, level));
            }
        }

        for touch in touches {
            let key = touch.actuator.key();
            let level = touch.level.clamp(0.0, 1.0);
            self.types.insert(key, touch.actuator.actuator_type.clone());
            let slot = self.touch.entry(key).or_default();
            let was_owned = slot.owns;
            if level > priority.threshold {
                slot.until = Some(now + priority.grace);
            }
            slot.level = level;
            slot.owns = level > priority.threshold || slot.until.is_some_and(|until| now < until);
            if !slot.owns {
                slot.until = None;
            }
            let owns = slot.owns;
            if owns {
                sends.extend(self.out(key, level));
            } else if was_owned {
                let level = self.requested.get(&key).copied().unwrap_or(0.0);
                sends.extend(self.out(key, level));
            }
        }
        sends
    }

    /// Stop everything: nothing is asked for any more, and touch lets go.
    pub(super) fn stop_all(&mut self) {
        self.requested.clear();
        self.touch.clear();
        self.sent.clear();
    }

    fn forget_sent(&mut self) {
        self.sent.clear();
    }

    /// The actuators touch drives right now.
    pub(super) fn touched(&self) -> Vec<Key> {
        let mut keys: Vec<Key> = self.touch.iter().filter(|(_, slot)| slot.owns).map(|(key, _)| *key).collect();
        keys.sort_unstable();
        keys
    }

    fn actuator_type(&self, key: &Key) -> String {
        self.types.get(key).cloned().unwrap_or_default()
    }
}

#[derive(Default)]
pub struct Toys {
    arbiter: Mutex<Arbiter>
}

fn toys(app: &AppHandle) -> &Toys {
    &app.state::<AppState>().inner().toys
}

pub(super) fn forget_sent(app: &AppHandle) {
    toys(app).arbiter.lock().unwrap().forget_sent();
}

/// Sends these levels, grouped by device.
fn send(app: &AppHandle, arbiter: &Arbiter, sends: Vec<Send>) {
    let mut by_device: BTreeMap<u32, Vec<(u32, f64, String)>> = BTreeMap::new();
    for ((device, actuator), level) in sends {
        by_device.entry(device).or_default().push((actuator, level, arbiter.actuator_type(&(device, actuator))));
    }
    let messages: Vec<_> = by_device.iter().map(|(device, scalars)| protocol::scalar_cmd(*device, scalars)).collect();
    if !messages.is_empty() {
        intiface::send(app, messages);
    }
}

/// Asks for these actuators at `level` (0–1). Nothing happens unless Intiface is connected. A toy
/// that isn't there is skipped: Intiface would refuse the whole command for it.
pub fn set(app: &AppHandle, actuators: &[ActuatorRef], level: f64) {
    if !intiface::connected(app) {
        return;
    }
    let level = level.clamp(0.0, 1.0);
    let mut arbiter = toys(app).arbiter.lock().unwrap();
    let sends = actuators
        .iter()
        .filter(|actuator| intiface::has_device(app, actuator.device_index))
        .filter_map(|actuator| arbiter.request(actuator, level))
        .collect();
    send(app, &arbiter, sends);
}

/// Haptics' levels this tick (see `Arbiter::touch`).
pub fn set_touch(app: &AppHandle, touches: Vec<Touch>, priority: Priority) {
    if !intiface::connected(app) {
        return;
    }
    let touches: Vec<Touch> = touches.into_iter().filter(|touch| intiface::has_device(app, touch.actuator.device_index)).collect();
    let mut arbiter = toys(app).arbiter.lock().unwrap();
    let sends = arbiter.touch(&touches, priority, Instant::now());
    send(app, &arbiter, sends);
}

/// An actuator touch drives right now, for the tiles.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Touched {
    pub device_index: u32,
    pub actuator_index: u32
}

pub fn touched(app: &AppHandle) -> Vec<Touched> {
    toys(app)
        .arbiter
        .lock()
        .unwrap()
        .touched()
        .into_iter()
        .map(|(device_index, actuator_index)| Touched { device_index, actuator_index })
        .collect()
}

/// Stops every connected toy, not only the ones a control drives.
pub fn stop_all(app: &AppHandle) {
    toys(app).arbiter.lock().unwrap().stop_all();
    intiface::send(app, vec![protocol::message("StopAllDevices", serde_json::json!({}))]);
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRIORITY: Priority = Priority { threshold: 0.02, grace: Duration::from_millis(1500) };

    fn lush() -> ActuatorRef {
        ActuatorRef { device_index: 0, actuator_index: 0, actuator_type: "Vibrate".into() }
    }

    fn edge() -> ActuatorRef {
        ActuatorRef { device_index: 1, actuator_index: 0, actuator_type: "Vibrate".into() }
    }

    fn touch(level: f64) -> Vec<Touch> {
        vec![Touch { actuator: lush(), level }]
    }

    #[test]
    fn last_writer_wins_without_touch() {
        let mut arbiter = Arbiter::default();
        assert_eq!(arbiter.request(&lush(), 0.4), Some(((0, 0), 0.4)));
        assert_eq!(arbiter.request(&lush(), 0.4), None, "unchanged");
        assert_eq!(arbiter.request(&lush(), 0.6), Some(((0, 0), 0.6)));
    }

    #[test]
    fn touch_owns_then_hands_back_after_the_grace() {
        let mut arbiter = Arbiter::default();
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);

        // A pattern runs; touch is set up but nobody touches.
        arbiter.request(&lush(), 0.5);
        assert_eq!(arbiter.touch(&touch(0.0), PRIORITY, at(0)), vec![]);
        assert_eq!(arbiter.request(&lush(), 0.6), Some(((0, 0), 0.6)));

        // Touch starts: the toy follows it, the pattern is held back.
        assert_eq!(arbiter.touch(&touch(0.3), PRIORITY, at(2000)), vec![((0, 0), 0.3)]);
        assert_eq!(arbiter.touched(), vec![(0, 0)]);
        assert_eq!(arbiter.request(&lush(), 0.7), None);

        // Touch stops: it keeps the toy at 0 for the grace.
        assert_eq!(arbiter.touch(&touch(0.3), PRIORITY, at(4950)), vec![]);
        assert_eq!(arbiter.touch(&touch(0.0), PRIORITY, at(5000)), vec![((0, 0), 0.0)]);
        assert_eq!(arbiter.request(&lush(), 0.8), None);
        assert_eq!(arbiter.touch(&touch(0.0), PRIORITY, at(6400)), vec![]);

        // Grace over: the toy rejoins the pattern where it is now.
        assert_eq!(arbiter.touch(&touch(0.0), PRIORITY, at(6550)), vec![((0, 0), 0.8)]);
        assert!(arbiter.touched().is_empty());
        assert_eq!(arbiter.request(&lush(), 0.9), Some(((0, 0), 0.9)));
    }

    #[test]
    fn a_new_touch_during_the_grace_extends_it() {
        let mut arbiter = Arbiter::default();
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        arbiter.touch(&touch(0.4), PRIORITY, at(0));
        arbiter.touch(&touch(0.0), PRIORITY, at(100));
        arbiter.touch(&touch(0.2), PRIORITY, at(1000));
        arbiter.touch(&touch(0.0), PRIORITY, at(1100));
        assert_eq!(arbiter.touch(&touch(0.0), PRIORITY, at(2000)), vec![], "grace runs from 1000 ms");
        assert_eq!(arbiter.touched(), vec![(0, 0)]);
        assert_eq!(arbiter.touch(&touch(0.0), PRIORITY, at(2600)), vec![], "back to the request: 0, already sent");
        assert!(arbiter.touched().is_empty());
    }

    #[test]
    fn toys_without_sources_are_never_touched() {
        let mut arbiter = Arbiter::default();
        arbiter.touch(&touch(0.5), PRIORITY, Instant::now());
        assert_eq!(arbiter.request(&edge(), 0.25), Some(((1, 0), 0.25)));
    }

    #[test]
    fn removing_sources_hands_the_toy_back() {
        let mut arbiter = Arbiter::default();
        arbiter.request(&lush(), 0.3);
        arbiter.touch(&touch(0.6), PRIORITY, Instant::now());
        assert_eq!(arbiter.touch(&[], PRIORITY, Instant::now()), vec![((0, 0), 0.3)]);
    }

    #[test]
    fn stop_everything_clears_touch_and_requests() {
        let mut arbiter = Arbiter::default();
        arbiter.request(&lush(), 0.3);
        arbiter.touch(&touch(0.6), PRIORITY, Instant::now());
        arbiter.stop_all();
        assert!(arbiter.touched().is_empty());
        assert_eq!(arbiter.touch(&[], PRIORITY, Instant::now()), vec![]);
        assert_eq!(arbiter.request(&lush(), 0.0), Some(((0, 0), 0.0)));
    }
}
