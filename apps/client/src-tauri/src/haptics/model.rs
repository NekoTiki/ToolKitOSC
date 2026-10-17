//! What `haptics.json` holds (see `types/haptics.ts` for the webview's copy). Sources are a tagged
//! union, like automation steps, so new kinds slot in as new variants.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub use crate::bhaptics::model::Position;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HapticsSettings {
    /// The master switch, also `TKOSC/Haptics`.
    pub enabled: bool,
    pub priority: TouchPriority,
    /// Used on every avatar, unless the avatar overrides that actuator.
    pub outputs: Vec<HapticsOutput>,
    /// Keyed by avatar id (`avtr_…`).
    pub avatars: BTreeMap<String, AvatarHaptics>
}

impl Default for HapticsSettings {
    fn default() -> Self {
        Self { enabled: true, priority: TouchPriority::default(), outputs: Vec::new(), avatars: BTreeMap::new() }
    }
}

/// While touch drives a toy, viewers wait: above `threshold`, and for `grace_ms` after.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TouchPriority {
    pub threshold: f64,
    pub grace_ms: u64
}

impl Default for TouchPriority {
    fn default() -> Self {
        Self { threshold: 0.02, grace_ms: 1500 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AvatarHaptics {
    /// The last name seen, for the page.
    pub name: String,
    /// Off: no haptics while wearing this avatar.
    pub enabled: bool,
    /// Each replaces the global output for the same actuator.
    pub outputs: Vec<HapticsOutput>
}

impl Default for AvatarHaptics {
    fn default() -> Self {
        Self { name: String::new(), enabled: true, outputs: Vec::new() }
    }
}

/// A toy motor, as controls store it (`IntifaceActuatorRef`): the names are for display. Or, with
/// `bhaptics`, a part of your bHaptics gear: then the Intiface fields are unused.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Actuator {
    pub device_index: u32,
    pub actuator_index: u32,
    pub actuator_type: String,
    pub device_name: String,
    pub actuator_description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bhaptics: Option<BhapticsTarget>
}

/// Which actuator an output is for, to match a global output with an avatar's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActuatorKey {
    Intiface(u32, u32),
    Bhaptics(Position, Region)
}

impl Actuator {
    pub fn key(&self) -> ActuatorKey {
        match &self.bhaptics {
            Some(target) => ActuatorKey::Bhaptics(target.position, target.region),
            None => ActuatorKey::Intiface(self.device_index, self.actuator_index)
        }
    }
}

/// Part of one bHaptics position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Region {
    #[default]
    Whole,
    /// The top two rows of a vest side: the chest, or the upper back.
    Upper,
    /// The three rows under them: the belly, or the lower back.
    Lower
}

/// How a level lights the region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Fill {
    /// Every motor at the level.
    #[default]
    All,
    /// Rows light from the bottom up as the level grows, like a socket's depth.
    Depth
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BhapticsTarget {
    pub position: Position,
    #[serde(default)]
    pub region: Region,
    #[serde(default)]
    pub fill: Fill
}

impl BhapticsTarget {
    /// The region's motors, top row first.
    pub fn motors(&self) -> Vec<usize> {
        let columns = self.position.columns();
        let vest = matches!(self.position, Position::VestFront | Position::VestBack);
        (0..self.position.motors())
            .filter(|motor| match (self.region, vest) {
                (Region::Upper, true) => motor / columns < 2,
                (Region::Lower, true) => motor / columns >= 2,
                _ => true
            })
            .collect()
    }

    /// What each of the position's motors plays for `level` (0–1, already capped at `max`).
    pub fn levels(&self, level: f64, max: f64) -> Vec<f64> {
        let columns = self.position.columns();
        let motors = self.motors();
        let mut levels = vec![0.0; self.position.motors()];
        match self.fill {
            Fill::All => motors.iter().for_each(|motor| levels[*motor] = level),
            Fill::Depth => {
                let top = motors.first().map_or(0, |motor| motor / columns);
                let rows = motors.last().map_or(0, |motor| motor / columns + 1) - top;
                // How far up the region the level reaches, in rows.
                let reach = if max > 0.0 { (level / max).clamp(0.0, 1.0) } else { 0.0 } * rows as f64;
                for motor in motors {
                    let from_bottom = (rows - 1 - (motor / columns - top)) as f64;
                    levels[motor] = (reach - from_bottom).clamp(0.0, 1.0) * max;
                }
            }
        }
        levels
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HapticsOutput {
    pub actuator: Actuator,
    pub sources: Vec<HapticsSource>,
    /// 0–1, the cap for this actuator whatever its sources say.
    pub max_strength: f64
}

impl Default for HapticsOutput {
    fn default() -> Self {
        Self { actuator: Actuator::default(), sources: Vec::new(), max_strength: 1.0 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
// The kinds are named after what they read, and more than SPS will come.
#[allow(clippy::enum_variant_names)]
pub enum HapticsSource {
    SpsSocket(SocketSource),
    SpsPlug(PlugSource),
    SpsTouch(TouchSource)
}

impl HapticsSource {
    pub fn zones(&self) -> &ZoneFilter {
        match self {
            Self::SpsSocket(source) => &source.zones,
            Self::SpsPlug(source) => &source.zones,
            Self::SpsTouch(source) => &source.zones
        }
    }

    pub fn shape(&self) -> &Shape {
        match self {
            Self::SpsSocket(source) => &source.shape,
            Self::SpsPlug(source) => &source.shape,
            Self::SpsTouch(source) => &source.shape
        }
    }
}

/// A socket (orifice) on your avatar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SocketSource {
    pub zones: ZoneFilter,
    pub own_hands: bool,
    pub other_hands: bool,
    pub my_plugs: bool,
    pub other_plugs: bool,
    /// Rubbing against other people's sockets.
    pub other_sockets: bool,
    pub shape: Shape
}

impl Default for SocketSource {
    fn default() -> Self {
        Self {
            zones: ZoneFilter::default(),
            own_hands: false,
            other_hands: true,
            my_plugs: false,
            other_plugs: true,
            other_sockets: true,
            shape: Shape::default()
        }
    }
}

/// A plug (penetrator) on your avatar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PlugSource {
    pub zones: ZoneFilter,
    pub own_hands: bool,
    pub other_hands: bool,
    pub my_sockets: bool,
    pub other_sockets: bool,
    /// Rubbing against other people's plugs.
    pub other_plugs: bool,
    pub shape: Shape
}

impl Default for PlugSource {
    fn default() -> Self {
        Self {
            zones: ZoneFilter::default(),
            own_hands: false,
            other_hands: true,
            my_sockets: false,
            other_sockets: true,
            other_plugs: true,
            shape: Shape::default()
        }
    }
}

/// A touch zone (head pats and the like).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TouchSource {
    pub zones: ZoneFilter,
    pub own_hands: bool,
    pub other_hands: bool,
    pub shape: Shape
}

impl Default for TouchSource {
    fn default() -> Self {
        Self { zones: ZoneFilter::default(), own_hands: false, other_hands: true, shape: Shape::default() }
    }
}

/// Which zones of the right kind a source reads, by name. An empty `include` means every zone.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ZoneFilter {
    pub include: Vec<String>,
    pub exclude: Vec<String>
}

impl ZoneFilter {
    pub fn matches(&self, zone: &str) -> bool {
        let listed = |list: &[String]| list.iter().map(|name| name.trim()).any(|name| !name.is_empty() && name == zone);
        if listed(&self.exclude) {
            return false;
        }
        let include_any = self.include.iter().any(|name| !name.trim().is_empty());
        !include_any || listed(&self.include)
    }
}

/// Turns a raw contact value into a level, in this order: follow movement, dead zone, scale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Shape {
    /// Speed instead of depth: the toy responds to thrusting, not to holding still.
    pub follow_movement: bool,
    /// 0–1. Values under it read 0, the rest is stretched back to 0–1.
    pub dead_zone: f64,
    /// 0–1, the source's Max strength.
    pub scale: f64
}

impl Default for Shape {
    fn default() -> Self {
        Self { follow_movement: false, dead_zone: 0.0, scale: 1.0 }
    }
}

impl HapticsOutput {
    /// A toy seen for the first time: a socket, a plug and a touch zone, each on every zone, like
    /// OscGoesBrrr gives a new toy.
    pub fn with_defaults(actuator: Actuator) -> Self {
        Self {
            actuator,
            sources: vec![
                HapticsSource::SpsSocket(SocketSource::default()),
                HapticsSource::SpsPlug(PlugSource::default()),
                HapticsSource::SpsTouch(TouchSource::default())
            ],
            max_strength: 1.0
        }
    }
}

impl HapticsSettings {
    /// Gives every actuator that has no global output yet the default sources. One whose sources
    /// you deleted keeps none. Returns whether any was added.
    pub fn add_missing(&mut self, actuators: impl IntoIterator<Item = Actuator>) -> bool {
        let mut added = false;
        for actuator in actuators {
            if !self.outputs.iter().any(|output| output.actuator.key() == actuator.key()) {
                self.outputs.push(HapticsOutput::with_defaults(actuator));
                added = true;
            }
        }
        added
    }

    /// Whether haptics are on while wearing `avatar_id`: the main switch, and the avatar's own.
    pub fn enabled_for(&self, avatar_id: Option<&str>) -> bool {
        self.enabled && !avatar_id.and_then(|id| self.avatars.get(id)).is_some_and(|avatar| !avatar.enabled)
    }

    /// The outputs that apply while wearing `avatar_id`: the avatar's own for each actuator it
    /// overrides, the global ones for the rest. None at all while haptics are off, globally or for
    /// this avatar.
    pub fn active_outputs(&self, avatar_id: Option<&str>) -> Vec<HapticsOutput> {
        if !self.enabled_for(avatar_id) {
            return Vec::new();
        }
        let Some(avatar) = avatar_id.and_then(|id| self.avatars.get(id)) else {
            return self.outputs.clone();
        };
        if !avatar.enabled {
            return Vec::new();
        }
        let mut outputs: Vec<HapticsOutput> = self
            .outputs
            .iter()
            .filter(|global| !avatar.outputs.iter().any(|own| own.actuator.key() == global.actuator.key()))
            .cloned()
            .collect();
        outputs.extend(avatar.outputs.iter().cloned());
        outputs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(device: u32, max: f64) -> HapticsOutput {
        HapticsOutput {
            actuator: Actuator { device_index: device, actuator_type: "Vibrate".into(), ..Default::default() },
            sources: vec![HapticsSource::SpsSocket(SocketSource::default())],
            max_strength: max
        }
    }

    #[test]
    fn reads_what_the_webview_writes() {
        let json = r#"{
            "enabled": true,
            "outputs": [{
                "actuator": { "deviceIndex": 2, "actuatorIndex": 1, "actuatorType": "Vibrate", "deviceName": "Lush 3" },
                "sources": [{ "kind": "sps-socket", "zones": { "include": ["Vagina"] }, "myPlugs": true, "shape": { "deadZone": 0.1 } }],
                "maxStrength": 0.8
            }]
        }"#;
        let settings: HapticsSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.priority, TouchPriority { threshold: 0.02, grace_ms: 1500 });
        let output = &settings.outputs[0];
        assert_eq!(output.actuator.key(), ActuatorKey::Intiface(2, 1));
        let HapticsSource::SpsSocket(socket) = &output.sources[0] else { panic!("not a socket") };
        assert!(socket.my_plugs && socket.other_plugs && socket.other_hands && !socket.own_hands);
        assert_eq!(socket.shape, Shape { follow_movement: false, dead_zone: 0.1, scale: 1.0 });

        let back = serde_json::to_value(&settings).unwrap();
        assert_eq!(back["outputs"][0]["sources"][0]["kind"], "sps-socket");
        assert_eq!(back["priority"]["graceMs"], 1500);
    }

    #[test]
    fn zone_filter() {
        let all = ZoneFilter::default();
        assert!(all.matches("Head"));
        let some = ZoneFilter { include: vec!["Vagina".into(), " ".into()], exclude: vec![] };
        assert!(some.matches("Vagina") && !some.matches("Mouth"));
        let but = ZoneFilter { include: vec![" ".into()], exclude: vec!["Mouth".into()] };
        assert!(but.matches("Vagina") && !but.matches("Mouth"));
    }

    #[test]
    fn new_toys_get_the_three_default_sources() {
        let mut settings = HapticsSettings { outputs: vec![HapticsOutput { sources: vec![], ..output(0, 1.0) }], ..Default::default() };
        let lush = Actuator { device_index: 0, ..Default::default() };
        let edge = Actuator { device_index: 1, ..Default::default() };
        assert!(settings.add_missing([lush.clone(), edge.clone()]));
        assert!(settings.outputs[0].sources.is_empty(), "deleted sources stay deleted");
        let kinds: Vec<_> = settings.outputs[1].sources.iter().map(|s| serde_json::to_value(s).unwrap()["kind"].clone()).collect();
        assert_eq!(kinds, ["sps-socket", "sps-plug", "sps-touch"]);
        assert!(settings.outputs[1].sources.iter().all(|s| s.zones().include.is_empty()));
        assert!(!settings.add_missing([lush, edge]));
    }

    #[test]
    fn avatar_overrides_per_actuator() {
        let mut settings = HapticsSettings { outputs: vec![output(0, 1.0), output(1, 1.0)], ..Default::default() };
        assert_eq!(settings.active_outputs(Some("avtr_a")).len(), 2);

        settings.avatars.insert("avtr_a".into(), AvatarHaptics { outputs: vec![output(1, 0.5)], ..Default::default() });
        let active = settings.active_outputs(Some("avtr_a"));
        assert_eq!(active.len(), 2);
        assert_eq!(active.iter().find(|o| o.actuator.device_index == 1).unwrap().max_strength, 0.5);
        assert_eq!(active.iter().find(|o| o.actuator.device_index == 0).unwrap().max_strength, 1.0);

        settings.avatars.get_mut("avtr_a").unwrap().enabled = false;
        assert!(settings.active_outputs(Some("avtr_a")).is_empty());
        assert_eq!(settings.active_outputs(Some("avtr_b")).len(), 2);

        settings.enabled = false;
        assert!(settings.active_outputs(None).is_empty());
    }

    #[test]
    fn a_vest_region_reads_and_keys_apart_from_toys() {
        let actuator: Actuator =
            serde_json::from_str(r#"{ "bhaptics": { "position": "VestFront", "region": "upper", "fill": "depth" } }"#).unwrap();
        assert_eq!(actuator.key(), ActuatorKey::Bhaptics(Position::VestFront, Region::Upper));
        assert_ne!(actuator.key(), Actuator::default().key());
        let toy = serde_json::to_value(Actuator::default()).unwrap();
        assert!(toy.get("bhaptics").is_none(), "toys are saved as before");
    }

    #[test]
    fn regions_and_fills() {
        let chest = BhapticsTarget { position: Position::VestFront, region: Region::Upper, fill: Fill::All };
        assert_eq!(chest.motors(), (0..8).collect::<Vec<_>>());
        let belly = BhapticsTarget { region: Region::Lower, ..chest };
        assert_eq!(belly.motors(), (8..20).collect::<Vec<_>>());
        let head = BhapticsTarget { position: Position::Head, region: Region::Upper, fill: Fill::All };
        assert_eq!(head.motors().len(), 6, "only vests have parts");

        assert_eq!(chest.levels(0.5, 1.0)[..9], [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.0]);
        let depth = BhapticsTarget { fill: Fill::Depth, ..belly };
        let half = depth.levels(0.5, 1.0);
        assert_eq!(half[16..20], [1.0; 4], "the bottom row first");
        assert_eq!(half[12..16], [0.5; 4]);
        assert_eq!(half[8], 0.0);
        let deep = depth.levels(1.0, 1.0);
        assert!(deep[8..20].iter().all(|level| *level == 1.0));
        assert!(depth.levels(0.5, 0.5)[8..20].iter().all(|level| *level == 0.5), "max strength fills it at max");
    }
}
