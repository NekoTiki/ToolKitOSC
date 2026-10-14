//! What Rust needs to decide on a viewer's command, as the webview last pushed it
//! (`server_set_policy`). The webview stays the source of truth for all of it: controls and
//! profiles in localStorage, groups and bans in its database. This is a read-only copy.
use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ban {
    /// `ip` (a stable IP, see `viewer::stable_ip`) or `discord`.
    pub scope: String,
    pub value: String,
    #[serde(default)]
    pub reason: Option<String>
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Policy {
    /// The control groups of the avatar you're wearing, hidden ones included, each control with
    /// the active profile's `locked`, `limits` and `groupRules` and whether it's `unavailable`, as
    /// `useControls` computes them.
    #[serde(default)]
    pub groups: Vec<Value>,
    /// Viewer key -> the groups they're in right now.
    #[serde(default)]
    pub memberships: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub bans: Vec<Ban>,
    /// Who may use controls: `everyone`, `username` or `discord`.
    #[serde(default)]
    pub client_type: String,
    /// Whether viewers see each other's names (a viewer's pattern says whose it is).
    #[serde(default)]
    pub show_viewers: bool
}

impl Policy {
    /// The control, and whether its group is hidden from viewers.
    pub fn control(&self, group_id: &str, control_id: &str) -> Option<(&Value, bool)> {
        let group = self.groups.iter().find(|group| group.get("id").and_then(Value::as_str) == Some(group_id))?;
        let hidden = group.get("hidden").and_then(Value::as_bool).unwrap_or(false);
        let control = group
            .get("controls")?
            .as_array()?
            .iter()
            .find(|control| control.get("id").and_then(Value::as_str) == Some(control_id))?;
        Some((control, hidden))
    }

    pub fn is_group_hidden(&self, group_id: &str) -> bool {
        self.groups
            .iter()
            .find(|group| group.get("id").and_then(Value::as_str) == Some(group_id))
            .and_then(|group| group.get("hidden"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    pub fn banned(&self, ip: &str, discord_id: Option<&str>) -> Option<&Ban> {
        self.bans.iter().find(|ban| (ban.scope == "ip" && ban.value == ip) || (ban.scope == "discord" && Some(ban.value.as_str()) == discord_id))
    }
}
