//! What `bhaptics.json` holds (see `lib/tauri/bhaptics.ts` for the webview's copy), and the
//! positions bHaptics Player knows.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A place on the body bHaptics Player drives, named as Player names it in a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Position {
    Head,
    VestFront,
    VestBack,
    ForearmL,
    ForearmR,
    HandL,
    HandR,
    GloveL,
    GloveR,
    FootL,
    FootR
}

impl Position {
    pub const ALL: [Position; 11] = [
        Position::Head,
        Position::VestFront,
        Position::VestBack,
        Position::ForearmL,
        Position::ForearmR,
        Position::HandL,
        Position::HandR,
        Position::GloveL,
        Position::GloveR,
        Position::FootL,
        Position::FootR
    ];

    pub fn motors(self) -> usize {
        match self {
            Position::Head | Position::ForearmL | Position::ForearmR => 6,
            Position::VestFront | Position::VestBack => 20,
            Position::HandL | Position::HandR | Position::FootL | Position::FootR => 3,
            Position::GloveL | Position::GloveR => 8
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Position::Head => "Head",
            Position::VestFront => "VestFront",
            Position::VestBack => "VestBack",
            Position::ForearmL => "ForearmL",
            Position::ForearmR => "ForearmR",
            Position::HandL => "HandL",
            Position::HandR => "HandR",
            Position::GloveL => "GloveL",
            Position::GloveR => "GloveR",
            Position::FootL => "FootL",
            Position::FootR => "FootR"
        }
    }

    /// How many motors in a row, numbered left to right, top to bottom.
    pub fn columns(self) -> usize {
        match self {
            Position::Head => 6,
            Position::VestFront | Position::VestBack | Position::GloveL | Position::GloveR => 4,
            _ => 3
        }
    }

    /// The name Player lists in `ConnectedPositions`: both sides of a vest are one `Vest`.
    pub fn device(self) -> &'static str {
        match self {
            Position::VestFront | Position::VestBack => "Vest",
            other => other.name()
        }
    }

    /// The key our frames for this position play under.
    pub fn key(self) -> String {
        format!("TKOSC_{}", self.name())
    }
}

/// Who reads the avatar's `bOSC` touch: ToolKitOSC, or bHaptics Player's own VRChat handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TouchSource {
    #[default]
    Toolkitosc,
    Player
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TouchFrom {
    /// Your own hands.
    #[serde(rename = "self")]
    pub own: bool,
    /// Everyone else.
    pub others: bool
}

impl Default for TouchFrom {
    fn default() -> Self {
        Self { own: false, others: true }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PositionSettings {
    /// 0 to 1, on top of Player's own intensity for the device.
    pub strength: f64
}

impl Default for PositionSettings {
    fn default() -> Self {
        Self { strength: 1.0 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BhapticsSettings {
    /// "Use bHaptics", also `TKOSC/bHaptics`. Off mutes avatar touch only.
    pub enabled: bool,
    pub touch_source: TouchSource,
    pub touch_from: TouchFrom,
    /// One missing here plays at full strength.
    pub positions: BTreeMap<Position, PositionSettings>
}

impl Default for BhapticsSettings {
    fn default() -> Self {
        Self { enabled: true, touch_source: TouchSource::default(), touch_from: TouchFrom::default(), positions: BTreeMap::new() }
    }
}

impl BhapticsSettings {
    pub fn strength(&self, position: Position) -> f64 {
        self.positions.get(&position).copied().unwrap_or_default().strength.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_the_webview_writes() {
        let settings: BhapticsSettings = serde_json::from_str(
            r#"{ "enabled": false, "touchSource": "player", "touchFrom": { "self": true }, "positions": { "VestBack": { "strength": 0.5 } } }"#
        )
        .unwrap();
        assert!(!settings.enabled);
        assert_eq!(settings.touch_source, TouchSource::Player);
        assert_eq!(settings.touch_from, TouchFrom { own: true, others: true });
        assert_eq!(settings.strength(Position::VestBack), 0.5);
        assert_eq!(settings.strength(Position::VestFront), 1.0);
        assert_eq!(serde_json::to_value(&settings).unwrap()["touchSource"], "player");
    }

    #[test]
    fn an_empty_file_is_the_defaults() {
        let settings: BhapticsSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings, BhapticsSettings::default());
        assert!(settings.enabled);
        assert!(!settings.touch_from.own);
    }
}
