//! What `haptics.json` holds (see `types/haptics.ts` for the webview's copy). Sources are a tagged
//! union, like automation steps, so new kinds slot in as new variants.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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

/// A toy motor, as controls store it (`IntifaceActuatorRef`): the names are for display.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Actuator {
    pub device_index: u32,
    pub actuator_index: u32,
    pub actuator_type: String,
    pub device_name: String,
    pub actuator_description: String
}

impl Actuator {
    pub fn key(&self) -> (u32, u32) {
        (self.device_index, self.actuator_index)
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

impl HapticsSettings {
    /// The outputs that apply while wearing `avatar_id`: the avatar's own for each actuator it
    /// overrides, the global ones for the rest. None at all while haptics are off, globally or for
    /// this avatar.
    pub fn active_outputs(&self, avatar_id: Option<&str>) -> Vec<HapticsOutput> {
        if !self.enabled {
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
        assert_eq!(output.actuator.key(), (2, 1));
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
}
