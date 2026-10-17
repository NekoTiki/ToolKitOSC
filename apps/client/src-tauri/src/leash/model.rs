//! What `leash.json` holds (see `lib/tauri/leash.ts` for the webview's copy). Defaults are
//! OSCLeash's, so a leash feels the same as it did there.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LeashSettings {
    /// "Leash moves me", also `TKOSC/Leash`.
    pub enabled: bool,
    /// Keyed by avatar id (`avtr_…`). Leash length and avatar size change what feels right.
    pub avatars: BTreeMap<String, AvatarLeash>
}

impl Default for LeashSettings {
    fn default() -> Self {
        Self { enabled: true, avatars: BTreeMap::new() }
    }
}

impl LeashSettings {
    /// This avatar's settings, or the defaults for one never set up.
    pub fn for_avatar(&self, avatar_id: Option<&str>) -> AvatarLeash {
        avatar_id.and_then(|id| self.avatars.get(id)).cloned().unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AvatarLeash {
    /// The last name seen, for the page.
    pub name: String,
    /// Stretch (0 to 1) past which you walk.
    pub walk: f64,
    /// Stretch past which you run.
    pub run: f64,
    /// How much a pull moves you.
    pub strength: f64,
    /// A pull this much up or down stops you rather than walking you into the floor or sky.
    pub up_down_deadzone: f64,
    pub up_down_compensation: f64,
    pub turning: Turning,
    /// Which grabbable PhysBones move you. One missing here uses `default_on`.
    pub leashes: BTreeMap<String, bool>
}

impl Default for AvatarLeash {
    fn default() -> Self {
        Self {
            name: String::new(),
            walk: 0.15,
            run: 0.70,
            strength: 1.2,
            up_down_deadzone: 0.5,
            up_down_compensation: 1.0,
            turning: Turning::default(),
            leashes: BTreeMap::new()
        }
    }
}

impl AvatarLeash {
    pub fn moves_you(&self, leash: &str) -> bool {
        self.leashes.get(leash).copied().unwrap_or_else(|| default_on(leash))
    }
}

/// Avatars have other grabbable bones (ears, tail, hair): only the ones called a leash move you
/// until you pick others.
pub fn default_on(leash: &str) -> bool {
    leash.to_lowercase().contains("leash")
}

/// Turning toward the pull. Off by default: it can cause motion sickness.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Turning {
    pub enabled: bool,
    pub multiplier: f64,
    /// Stretch past which you turn.
    pub deadzone: f64,
    /// Stop turning once the pull is within this many degrees of straight ahead.
    pub goal_degrees: f64
}

impl Default for Turning {
    fn default() -> Self {
        Self { enabled: false, multiplier: 0.8, deadzone: 0.15, goal_degrees: 90.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_leashes_move_you_by_default() {
        let avatar = AvatarLeash::default();
        assert!(avatar.moves_you("Leash"));
        assert!(avatar.moves_you("Leash_North"));
        assert!(avatar.moves_you("CollarLEASH"));
        assert!(!avatar.moves_you("Tail"));
        assert!(!avatar.moves_you("Ear_L"));
    }

    #[test]
    fn picked_bones_override_the_default() {
        let mut avatar = AvatarLeash::default();
        avatar.leashes.insert("Tail".into(), true);
        avatar.leashes.insert("Leash".into(), false);
        assert!(avatar.moves_you("Tail"));
        assert!(!avatar.moves_you("Leash"));
    }

    #[test]
    fn missing_fields_use_the_defaults() {
        let avatar: AvatarLeash = serde_json::from_str(r#"{"walk":0.2}"#).unwrap();
        assert_eq!(avatar.walk, 0.2);
        assert_eq!(avatar.run, 0.70);
        assert_eq!(avatar.strength, 1.2);
        let settings: LeashSettings = serde_json::from_str("{}").unwrap();
        assert!(settings.enabled);
    }
}
