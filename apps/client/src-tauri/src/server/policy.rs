//! What Rust needs to decide on a viewer's command besides the controls (those are in `setup`), as
//! the webview last pushed it (`server_set_policy`). The webview stays the source of truth for
//! these: groups and bans in its database. This is a read-only copy.
use std::collections::HashMap;

use serde::Deserialize;
use serde_json::{json, Value};
use tauri::AppHandle;

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
    pub fn banned(&self, ip: &str, discord_id: Option<&str>) -> Option<&Ban> {
        self.bans.iter().find(|ban| (ban.scope == "ip" && ban.value == ip) || (ban.scope == "discord" && Some(ban.value.as_str()) == discord_id))
    }
}

/// The widest limits any viewer gets on a control: the profile's, opened up by every group rule.
fn widest_limits(control: &Value) -> Option<Value> {
    let groups: Vec<String> = control.get("groupRules").and_then(Value::as_object).map(|rules| rules.keys().cloned().collect()).unwrap_or_default();
    super::limits::resolve_control_for_viewer(control, &groups).get("limits").cloned().filter(|limits| !limits.is_null())
}

fn controls(groups: &[Value]) -> impl Iterator<Item = &Value> {
    groups.iter().filter_map(|group| group.get("controls")?.as_array()).flatten()
}

/// A profile turned on, or its limits changed while on: running toys are brought within the new
/// limits. A toy above its new max drops to it, and a pattern that's now blocked stops (a viewer's
/// own pattern too: it was clamped to the old limits). Only ever lowers, so it can't start
/// anything; avatar parameters are left alone. Uses the most any group gets, so a friend's toy
/// isn't cut to what everyone else may set.
pub fn clamp_running(app: &AppHandle, previous: Option<&[Value]>, groups: &[Value]) {
    for control in controls(groups) {
        let Some(id) = control.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(limits) = widest_limits(control) else {
            continue;
        };
        let before = previous
            .and_then(|previous| controls(previous).find(|c| c.get("id").and_then(Value::as_str) == Some(id)))
            .and_then(widest_limits);
        if before.as_ref() == Some(&limits) {
            continue;
        }

        match control.get("type").and_then(Value::as_str) {
            Some("intiface-toy") => {
                let (_, max) = super::limits::limit_range(Some(&limits));
                if crate::controls::toy_level(id) > max {
                    crate::controls::run(app, control, &json!({ "type": "intiface-toy", "value": max }), None);
                }
            }
            Some("intiface-pattern") => {
                let current = crate::toys::patterns::value(app, id);
                let blocked = current == super::limits::VIEWER_PATTERN_ID || super::limits::is_option_disabled(Some(&limits), &json!(current));
                if current != super::limits::OFF_ID && blocked {
                    crate::controls::run(app, control, &json!({ "type": "intiface-pattern", "value": super::limits::OFF_ID }), None);
                }
            }
            _ => {}
        }
    }
}
