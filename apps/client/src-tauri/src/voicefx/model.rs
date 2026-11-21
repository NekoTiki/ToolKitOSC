//! What `voicefx.json` holds (see `lib/tauri/voicefx.ts` for the webview's copy).
use serde::{Deserialize, Serialize};

/// The effects, in the order of `TKOSC/VoiceFXEffect` (1 to 4; 0 is the default effect).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Effect {
    #[default]
    Muffled,
    Underwater,
    Radio,
    SoftMute
}

impl Effect {
    pub const ALL: [Effect; 4] = [Effect::Muffled, Effect::Underwater, Effect::Radio, Effect::SoftMute];

    /// `TKOSC/VoiceFXEffect`: 0 (or anything unknown) is the default effect.
    pub fn from_parameter(value: f64) -> Option<Effect> {
        let index = value.round() as i64;
        (1..=4).contains(&index).then(|| Effect::ALL[index as usize - 1])
    }

    pub fn label(self) -> &'static str {
        match self {
            Effect::Muffled => "Muffled",
            Effect::Underwater => "Underwater",
            Effect::Radio => "Radio",
            Effect::SoftMute => "Soft mute"
        }
    }

    /// As the webview and saved controls write it.
    pub fn parse(id: &str) -> Option<Effect> {
        serde_json::from_value(serde_json::Value::String(id.to_owned())).ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MicMode {
    #[default]
    One,
    /// One mic on desktop, another in VR, picked from VRChat's `VRMode`.
    Split
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub mic_mode: MicMode,
    /// Device names. `None`: the system's default microphone.
    pub mic: Option<String>,
    pub mic_vr: Option<String>,
    /// The cable VRChat listens to. Nothing plays until one is picked: the default output would
    /// be your own speakers.
    pub output: Option<String>,
    /// Also plays your voice on the default output, for testing.
    pub monitor: bool,
    /// What VRChat's `VRMode` last said, so the right mic is used before it says so again.
    pub last_vr: bool,
    /// Used when a source names no effect: `TKOSC/VoiceFX`, Turn on myself, an automation.
    pub default_effect: Effect,
    /// The strength of those, 0 to 1.
    pub level: f64,
    pub mouth: Mouth
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mic_mode: MicMode::One,
            mic: None,
            mic_vr: None,
            output: None,
            monitor: false,
            last_vr: false,
            default_effect: Effect::Muffled,
            level: 0.7,
            mouth: Mouth::default()
        }
    }
}

/// The mouth socket link: something in the socket holds an effect, stronger the deeper it goes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Mouth {
    pub enabled: bool,
    /// The socket's name. `None`: the one whose name looks like a mouth (see `looks_like_mouth`).
    pub socket: Option<String>,
    pub others: bool,
    pub own: bool,
    pub hands: bool,
    pub effect: Effect,
    /// Strength at the tip and all the way in.
    pub tip: f64,
    pub deep: f64,
    /// How long after the socket is empty the effect ends.
    pub release_ms: u32
}

impl Default for Mouth {
    fn default() -> Self {
        Self { enabled: false, socket: None, others: true, own: false, hands: false, effect: Effect::Muffled, tip: 0.3, deep: 1.0, release_ms: 1000 }
    }
}

pub fn looks_like_mouth(socket: &str) -> bool {
    let name = socket.to_lowercase();
    ["blowjob", "mouth", "oral", "bj"].iter().any(|word| name.contains(word))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effects_read_as_the_webview_writes_them() {
        assert_eq!(Effect::parse("soft-mute"), Some(Effect::SoftMute));
        assert_eq!(Effect::parse("muffled"), Some(Effect::Muffled));
        assert_eq!(Effect::parse("loud"), None);
    }

    #[test]
    fn the_effect_parameter_counts_from_one() {
        assert_eq!(Effect::from_parameter(0.0), None);
        assert_eq!(Effect::from_parameter(1.0), Some(Effect::Muffled));
        assert_eq!(Effect::from_parameter(4.0), Some(Effect::SoftMute));
        assert_eq!(Effect::from_parameter(9.0), None);
    }

    #[test]
    fn mouth_sockets_are_found_by_name() {
        assert!(looks_like_mouth("Blowjob"));
        assert!(looks_like_mouth("SPS_Mouth"));
        assert!(!looks_like_mouth("Vagina"));
    }

    #[test]
    fn missing_fields_use_the_defaults() {
        let settings: Settings = serde_json::from_str(r#"{"monitor":true}"#).unwrap();
        assert!(settings.monitor);
        assert_eq!(settings.level, 0.7);
        assert!(!settings.mouth.enabled);
    }
}
