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

/// Which corner of your view the HUD sits in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Corner {
    #[default]
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight
}

/// How the HUD keeps up with your head.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Follow {
    /// Stays put in the world while you look around a little, then glides back to its corner.
    #[default]
    Lazy,
    /// Moves exactly with your view.
    Glued
}

/// The HUD in a corner of your view. Display only: nothing on it is pressed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HudSettings {
    pub enabled: bool,
    pub corner: Corner,
    /// 0.5–2, 1 is the usual size. It keeps that size in your view at any distance.
    pub size: f32,
    /// 0.3–1.
    pub opacity: f32,
    /// In meters, 0.5–3.
    pub distance: f32,
    pub follow: Follow,
    /// How fast Lazy glides back, 0.1–1.
    pub catch_up: f32,
    /// Hidden while the wrist panel shows: it shows the same things.
    pub hide_with_panel: bool,
    /// What's on it.
    pub status: bool,
    pub viewers: bool,
    pub last_action: bool,
    pub toys: bool,
    pub leash: bool,
    pub warnings: bool
}

impl Default for HudSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            corner: Corner::BottomLeft,
            size: 1.0,
            opacity: 0.85,
            distance: 1.5,
            follow: Follow::Lazy,
            catch_up: 0.5,
            hide_with_panel: true,
            status: true,
            viewers: true,
            last_action: true,
            toys: false,
            leash: false,
            warnings: true
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OverlaySettings {
    pub notify: NotifySettings,
    pub panel: PanelSettings,
    pub hud: HudSettings,
    /// Avatar id -> the controls on its Pinned page, in order (see `super::panel::pinned`).
    pub pinned: std::collections::BTreeMap<String, Vec<String>>
}
