//! What `overlay.json` holds.
use serde::{Deserialize, Serialize};

/// Where toasts go.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    /// XSOverlay if it runs, otherwise OVR Toolkit.
    #[default]
    Auto,
    Xsoverlay,
    Ovrtoolkit,
    Off
}

/// Which events send a toast.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NotifySettings {
    pub to: Target,
    /// Shocks and OWO.
    pub shocks: bool,
    pub toys: bool,
    /// Every other control.
    pub controls: bool,
    /// Someone joined.
    pub viewers: bool,
    /// Stop everything and resume.
    pub stop: bool,
    pub automations: bool
}

impl Default for NotifySettings {
    fn default() -> Self {
        Self { to: Target::Auto, shocks: true, toys: false, controls: false, viewers: true, stop: true, automations: false }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Hand {
    #[default]
    Left,
    Right
}

/// The panel on your wrist.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PanelSettings {
    pub enabled: bool,
    pub hand: Hand,
    /// 10–30.
    pub width_cm: f32,
    /// 0.3–1.
    pub opacity: f32,
    /// Fully shown within this angle between the panel's face and your eyes; it fades out over the
    /// next 15°.
    pub show_within_deg: f32
}

impl Default for PanelSettings {
    fn default() -> Self {
        Self { enabled: true, hand: Hand::Left, width_cm: 16.0, opacity: 0.9, show_within_deg: 35.0 }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OverlaySettings {
    pub notify: NotifySettings,
    pub panel: PanelSettings
}
