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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OverlaySettings {
    pub notify: NotifySettings
}
