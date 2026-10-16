//! Turns the zones' contacts into one level per actuator, every tick. Pure: no app handle, so it's
//! tested on its own.
use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

use serde::Serialize;

use super::model::{HapticsOutput, HapticsSource, Shape};
use super::zones::{self, Contact, Family, Value, Zone, ZoneId, ZoneKind};

/// The longest gap between two ticks that Follow movement measures speed over.
const MAX_STEP: Duration = Duration::from_millis(250);
/// Follow movement: this many units per second reads as full strength.
const FULL_SPEED: f64 = 5.0;

/// A zone as the page shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoneView {
    pub family: Family,
    pub kind: ZoneKind,
    pub name: String,
    pub level: f64
}

/// One actuator's level, and each of its sources' levels in order (for the page).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputLevel {
    pub device_index: u32,
    pub actuator_index: u32,
    pub level: f64,
    pub sources: Vec<f64>
}

#[derive(Debug, Default)]
pub struct Engine {
    zones: BTreeMap<ZoneId, Zone>,
    last_tick: Option<Instant>,
    /// Follow movement: each reading's value at the last tick, keyed by output, source, zone and
    /// contact. Only the ones read last tick are kept.
    previous: HashMap<(usize, usize, ZoneId, Contact), f64>
}

impl Engine {
    /// A live avatar parameter (name without `/avatar/parameters/`). Returns whether it was a
    /// contact parameter.
    pub fn on_parameter(&mut self, name: &str, value: Value) -> bool {
        let Some((id, key)) = zones::parse(name) else {
            return false;
        };
        self.zones.entry(id).or_default().set(&key, value);
        true
    }

    /// Forgets every zone: another avatar, or VRChat went away.
    pub fn clear(&mut self) {
        self.zones.clear();
        self.previous.clear();
    }

    /// Legacy TPS zones only count on avatars with nothing newer.
    fn counted(&self) -> impl Iterator<Item = (&ZoneId, &Zone)> {
        let has_newer = self.zones.keys().any(|id| id.family != Family::Tps);
        self.zones.iter().filter(move |(id, _)| !has_newer || id.family != Family::Tps)
    }

    pub fn zones(&self) -> Vec<ZoneView> {
        self.counted()
            .map(|(id, zone)| ZoneView {
                family: id.family,
                kind: id.kind,
                name: id.name.clone(),
                level: zone.strongest(id.family, id.kind)
            })
            .collect()
    }

    /// Each output's level now: the strongest of its sources, capped by its Max strength.
    pub fn tick(&mut self, now: Instant, outputs: &[HapticsOutput]) -> Vec<OutputLevel> {
        let step = self.last_tick.map_or(MAX_STEP, |last| now.saturating_duration_since(last).min(MAX_STEP));
        self.last_tick = Some(now);

        let mut seen = HashMap::new();
        let levels = outputs
            .iter()
            .enumerate()
            .map(|(output_at, output)| {
                let sources: Vec<f64> = output
                    .sources
                    .iter()
                    .enumerate()
                    .map(|(source_at, source)| {
                        let mut level: f64 = 0.0;
                        for (id, zone) in self.counted() {
                            let Some(contacts) = contacts(source, id.kind) else { continue };
                            if !source.zones().matches(&id.name) {
                                continue;
                            }
                            for contact in contacts {
                                let raw = zone.read(id.family, id.kind, contact);
                                let key = (output_at, source_at, id.clone(), contact);
                                let previous = self.previous.get(&key).copied();
                                seen.insert(key, raw);
                                level = level.max(shape(source.shape(), raw, previous, step));
                            }
                        }
                        level.clamp(0.0, 1.0)
                    })
                    .collect();
                let strongest = sources.iter().copied().fold(0.0, f64::max);
                OutputLevel {
                    device_index: output.actuator.device_index,
                    actuator_index: output.actuator.actuator_index,
                    level: strongest.min(output.max_strength.clamp(0.0, 1.0)),
                    sources
                }
            })
            .collect();
        self.previous = seen;
        levels
    }
}

/// The contacts a source listens to on a zone of this kind, or `None` when it doesn't read that
/// kind of zone at all.
fn contacts(source: &HapticsSource, kind: ZoneKind) -> Option<Vec<Contact>> {
    let pick = |list: &[(bool, Contact)]| list.iter().filter(|(on, _)| *on).map(|(_, contact)| *contact).collect();
    match (source, kind) {
        (HapticsSource::SpsSocket(s), ZoneKind::Socket) => Some(pick(&[
            (s.own_hands, Contact::OwnHands),
            (s.other_hands, Contact::OtherHands),
            (s.my_plugs, Contact::Mine),
            (s.other_plugs, Contact::Others),
            (s.other_sockets, Contact::Rubbing)
        ])),
        (HapticsSource::SpsPlug(s), ZoneKind::Plug) => Some(pick(&[
            (s.own_hands, Contact::OwnHands),
            (s.other_hands, Contact::OtherHands),
            (s.my_sockets, Contact::Mine),
            (s.other_sockets, Contact::Others),
            (s.other_plugs, Contact::Rubbing)
        ])),
        (HapticsSource::SpsTouch(s), ZoneKind::Touch) => {
            Some(pick(&[(s.own_hands, Contact::OwnHands), (s.other_hands, Contact::OtherHands)]))
        }
        _ => None
    }
}

/// Follow movement, then the dead zone, then the scale. `previous` is the raw value at the last
/// tick (`None` for a reading that just appeared: it starts still).
fn shape(shape: &Shape, raw: f64, previous: Option<f64>, step: Duration) -> f64 {
    let mut value = raw;
    if shape.follow_movement {
        let seconds = step.as_secs_f64();
        let speed = if seconds > 0.0 { (raw - previous.unwrap_or(raw)) / seconds } else { 0.0 };
        value = speed.abs() / FULL_SPEED;
    }
    if shape.dead_zone >= 1.0 {
        value = 0.0;
    } else if shape.dead_zone > 0.0 {
        value = (value - shape.dead_zone) / (1.0 - shape.dead_zone);
    }
    let value = value * shape.scale;
    if value.is_nan() { 0.0 } else { value }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::haptics::model::{Actuator, PlugSource, SocketSource, TouchSource, ZoneFilter};
    use Value::{Bool, Number};

    fn output(sources: Vec<HapticsSource>, max: f64) -> HapticsOutput {
        HapticsOutput { actuator: Actuator { device_index: 3, actuator_index: 1, ..Default::default() }, sources, max_strength: max }
    }

    fn socket(include: &[&str]) -> HapticsSource {
        HapticsSource::SpsSocket(SocketSource {
            zones: ZoneFilter { include: include.iter().map(|s| s.to_string()).collect(), exclude: vec![] },
            ..Default::default()
        })
    }

    fn touch(shape: Shape) -> HapticsSource {
        HapticsSource::SpsTouch(TouchSource { shape, ..Default::default() })
    }

    #[test]
    fn strongest_source_capped_by_the_output() {
        let mut engine = Engine::default();
        engine.on_parameter("OGB/Orf/Vagina/PenOthers", Number(0.6));
        engine.on_parameter("OGB/Orf/Mouth/PenOthers", Number(0.9));
        engine.on_parameter("VFH/Zone/Touch/Head/Others", Number(0.3));
        let now = Instant::now();

        let levels = engine.tick(now, &[output(vec![socket(&["Vagina"]), touch(Shape::default())], 1.0)]);
        assert_eq!(levels[0].sources, vec![0.6, 0.3]);
        assert_eq!(levels[0].level, 0.6);
        assert_eq!((levels[0].device_index, levels[0].actuator_index), (3, 1));

        let levels = engine.tick(now, &[output(vec![socket(&[])], 0.8)]);
        assert_eq!(levels[0].level, 0.8);
    }

    #[test]
    fn sources_only_read_their_own_kind_of_zone() {
        let mut engine = Engine::default();
        engine.on_parameter("OGB/Pen/Penis/PenOthers", Number(0.7));
        let plug = HapticsSource::SpsPlug(PlugSource::default());
        let levels = engine.tick(Instant::now(), &[output(vec![socket(&[]), touch(Shape::default()), plug], 1.0)]);
        assert_eq!(levels[0].sources, vec![0.0, 0.0, 0.7]);
    }

    #[test]
    fn switches_pick_the_contacts() {
        let mut engine = Engine::default();
        engine.on_parameter("OGB/Orf/Vagina/TouchSelf", Number(0.5));
        engine.on_parameter("OGB/Orf/Vagina/TouchSelfClose", Bool(true));
        let default = socket(&[]);
        let own = HapticsSource::SpsSocket(SocketSource { own_hands: true, ..Default::default() });
        let levels = engine.tick(Instant::now(), &[output(vec![default, own], 1.0)]);
        assert_eq!(levels[0].sources, vec![0.0, 0.5]);
    }

    #[test]
    fn tps_only_without_newer_zones() {
        let mut engine = Engine::default();
        engine.on_parameter("TPS_Internal/Orf/Vagina/Depth_In", Number(0.4));
        let outputs = [output(vec![socket(&[])], 1.0)];
        assert_eq!(engine.tick(Instant::now(), &outputs)[0].level, 0.4);
        assert_eq!(engine.zones().len(), 1);

        engine.on_parameter("OGB/Touch/Head/Others", Number(0.0));
        assert_eq!(engine.tick(Instant::now(), &outputs)[0].level, 0.0);
        assert_eq!(engine.zones().iter().map(|z| z.name.as_str()).collect::<Vec<_>>(), ["Head"]);
    }

    #[test]
    fn dead_zone_then_scale() {
        let mut engine = Engine::default();
        engine.on_parameter("OGB/Touch/Head/Others", Number(0.55));
        let shaped = touch(Shape { follow_movement: false, dead_zone: 0.1, scale: 0.8 });
        let level = engine.tick(Instant::now(), &[output(vec![shaped], 1.0)])[0].level;
        assert!((level - 0.4).abs() < 1e-9, "{level}");

        // Under the dead zone goes negative, so it never wins the max.
        assert!(shape(&Shape { dead_zone: 0.6, ..Default::default() }, 0.3, None, MAX_STEP) < 0.0);
        assert_eq!(shape(&Shape { dead_zone: 1.0, ..Default::default() }, 0.9, None, MAX_STEP), 0.0);
    }

    #[test]
    fn follow_movement_reads_speed() {
        let mut engine = Engine::default();
        let outputs = [output(vec![touch(Shape { follow_movement: true, dead_zone: 0.0, scale: 1.0 })], 1.0)];
        let start = Instant::now();
        engine.on_parameter("OGB/Touch/Head/Others", Number(0.2));
        assert_eq!(engine.tick(start, &outputs)[0].level, 0.0, "a new reading starts still");

        engine.on_parameter("OGB/Touch/Head/Others", Number(0.3));
        let level = engine.tick(start + Duration::from_millis(50), &outputs)[0].level;
        // 0.1 in 50 ms = 2/s, so 2 / 5.
        assert!((level - 0.4).abs() < 1e-9, "{level}");

        engine.on_parameter("OGB/Touch/Head/Others", Number(0.2));
        let level = engine.tick(start + Duration::from_secs(10), &outputs)[0].level;
        // A long gap counts as 250 ms: 0.1 in 0.25 s = 0.4/s.
        assert!((level - 0.08).abs() < 1e-9, "{level}");

        assert_eq!(engine.tick(start + Duration::from_secs(11), &outputs)[0].level, 0.0, "held still");
    }

    #[test]
    fn clear_forgets_the_zones() {
        let mut engine = Engine::default();
        assert!(engine.on_parameter("OGB/Touch/Head/Others", Number(0.5)));
        assert!(!engine.on_parameter("Grounded", Bool(true)));
        engine.clear();
        assert!(engine.zones().is_empty());
        assert_eq!(engine.tick(Instant::now(), &[output(vec![touch(Shape::default())], 1.0)])[0].level, 0.0);
    }
}
