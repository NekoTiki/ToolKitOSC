//! What `owo.json` holds (see `lib/tauri/owo.ts` for the webview's copy), and the OWO Skin's ten
//! muscle groups.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A muscle group the OWO Skin drives, in the order of its index in OWO's protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema)]
pub enum Muscle {
    PectoralR,
    PectoralL,
    AbdominalR,
    AbdominalL,
    ArmR,
    ArmL,
    DorsalR,
    DorsalL,
    LumbarR,
    LumbarL
}

/// Touch owns one side at a time: the front (chest, belly, arms) or the back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Front,
    Back
}

impl Muscle {
    pub const ALL: [Muscle; 10] = [
        Muscle::PectoralR,
        Muscle::PectoralL,
        Muscle::AbdominalR,
        Muscle::AbdominalL,
        Muscle::ArmR,
        Muscle::ArmL,
        Muscle::DorsalR,
        Muscle::DorsalL,
        Muscle::LumbarR,
        Muscle::LumbarL
    ];

    /// Its number in OWO's packets.
    pub fn index(self) -> usize {
        self as usize
    }

    #[cfg(debug_assertions)]
    pub fn from_index(index: usize) -> Option<Muscle> {
        Muscle::ALL.get(index).copied()
    }

    pub fn side(self) -> Side {
        if self.index() < 6 { Side::Front } else { Side::Back }
    }

    /// The bool vrc-owo-suit's Unity package puts on the avatar for it.
    pub fn parameter(self) -> &'static str {
        match self {
            Muscle::PectoralR => "owo_suit_Pectoral_R",
            Muscle::PectoralL => "owo_suit_Pectoral_L",
            Muscle::AbdominalR => "owo_suit_Abdominal_R",
            Muscle::AbdominalL => "owo_suit_Abdominal_L",
            Muscle::ArmR => "owo_suit_Arm_R",
            Muscle::ArmL => "owo_suit_Arm_L",
            Muscle::DorsalR => "owo_suit_Dorsal_R",
            Muscle::DorsalL => "owo_suit_Dorsal_L",
            Muscle::LumbarR => "owo_suit_Lumbar_R",
            Muscle::LumbarL => "owo_suit_Lumbar_L"
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MuscleSettings {
    /// 0 to 1, under the limits set in the OWO app's calibration.
    pub strength: f64
}

impl Default for MuscleSettings {
    fn default() -> Self {
        Self { strength: 0.6 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OwoSettings {
    /// "Use the OWO Skin", also `TKOSC/OWO`. Off mutes avatar touch only.
    pub enabled: bool,
    /// The OWO app's IP, or `ip:port`. None finds it by broadcast on your network.
    pub address: Option<String>,
    /// Hz, 1 to 100.
    pub frequency: u8,
    /// One missing here plays at the default strength.
    pub muscles: BTreeMap<Muscle, MuscleSettings>,
    /// Scales touch by the avatar's `owo_intensity` radial, when it has one.
    pub use_avatar_intensity: bool
}

impl Default for OwoSettings {
    fn default() -> Self {
        Self { enabled: true, address: None, frequency: 100, muscles: BTreeMap::new(), use_avatar_intensity: true }
    }
}

impl OwoSettings {
    pub fn strength(&self, muscle: Muscle) -> f64 {
        self.muscles.get(&muscle).copied().unwrap_or_default().strength.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_the_webview_writes() {
        let settings: OwoSettings =
            serde_json::from_str(r#"{ "enabled": false, "address": "192.168.1.42", "muscles": { "LumbarL": { "strength": 0.45 } } }"#).unwrap();
        assert!(!settings.enabled);
        assert_eq!(settings.address.as_deref(), Some("192.168.1.42"));
        assert_eq!(settings.strength(Muscle::LumbarL), 0.45);
        assert_eq!(settings.strength(Muscle::PectoralL), 0.6);
        assert_eq!(settings.frequency, 100);
        assert_eq!(serde_json::to_value(&settings).unwrap()["useAvatarIntensity"], true);
    }

    #[test]
    fn muscles_are_numbered_and_sided_as_owo_does() {
        assert_eq!(Muscle::PectoralR.index(), 0);
        assert_eq!(Muscle::LumbarL.index(), 9);
        assert_eq!(Muscle::from_index(8), Some(Muscle::LumbarR));
        assert_eq!(Muscle::from_index(10), None);
        assert_eq!(Muscle::ArmL.side(), Side::Front);
        assert_eq!(Muscle::DorsalR.side(), Side::Back);
    }
}
