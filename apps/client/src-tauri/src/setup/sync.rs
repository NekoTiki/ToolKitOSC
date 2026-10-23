//! What the share server needs from the setup: the controls viewers may see (`controls-update`),
//! the avatar parameters those controls use (see `server::set_forwarded`), and who on the share page
//! is in which viewer group (`viewer-groups`), so it sends each viewer their own copy. Sent when
//! the server signs us in and whenever one of them changes.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use serde_json::{json, Value};
use tauri::AppHandle;

/// What was last sent, so nothing goes out twice.
#[derive(Default)]
pub struct Sent {
    controls: Option<Value>,
    viewer_groups: Option<Value>
}

fn sent(app: &AppHandle) -> &Mutex<Sent> {
    &super::setup(app).sent
}

/// The groups viewers may see: hidden groups left out, and controls whose add-on is off. Rust still
/// has those, as unavailable, so a command already on its way is dropped rather than unknown.
fn visible(app: &AppHandle, groups: &[Value]) -> Vec<Value> {
    groups
        .iter()
        .filter(|group| !group.get("hidden").and_then(Value::as_bool).unwrap_or(false))
        .map(|group| {
            let mut group = group.clone();
            if let Some(controls) = group.get_mut("controls").and_then(Value::as_array_mut) {
                controls.retain(|control| control.get("type").and_then(Value::as_str).is_none_or(|kind| crate::addons::control_type_live(app, kind)));
            }
            group
        })
        .collect()
}

/// Every avatar parameter the visible controls set. Their values are shown on the share page.
fn addresses(groups: &[Value]) -> BTreeSet<String> {
    let mut addresses = BTreeSet::new();
    let mut add = |value: Option<&Value>| {
        if let Some(address) = value.and_then(Value::as_str) {
            addresses.insert(address.to_owned());
        }
    };
    for control in groups.iter().filter_map(|group| group.get("controls")?.as_array()).flatten() {
        match control.get("type").and_then(Value::as_str) {
            Some("boolean" | "enum" | "slider" | "step-enum") => add(control.get("inputAddress")),
            Some("boolean-group") => {
                for input in control.get("inputs").and_then(Value::as_array).into_iter().flatten() {
                    add(input.get("inputAddress"));
                }
            }
            Some("boolean-enum") => {
                for input in control.get("inputs").and_then(Value::as_array).into_iter().flatten() {
                    for side in ["true", "false"] {
                        for address in input.pointer(&format!("/inputAddress/{side}")).and_then(Value::as_array).into_iter().flatten() {
                            add(Some(address));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    addresses
}

/// Sends whatever changed since last time. `again` sends everything: the server just signed us
/// in and knows nothing yet.
pub fn sync(app: &AppHandle, again: bool) {
    if !crate::server::authed(app) {
        return;
    }
    let snapshot = super::snapshot(app);
    let groups = visible(app, &snapshot.groups);
    let controls = json!({ "avatarId": snapshot.avatar_id, "groups": groups });

    let members: BTreeMap<String, Vec<String>> =
        crate::server::viewers(app).into_iter().filter(|(_, groups)| !groups.is_empty()).map(|(viewer, groups)| (viewer.peer_id, groups)).collect();
    let viewer_groups = json!({ "groups": super::viewers::groups(app), "members": members });

    let (send_controls, send_groups) = {
        let mut sent = sent(app).lock().unwrap();
        let send_controls = again || sent.controls.as_ref() != Some(&controls);
        let send_groups = again || sent.viewer_groups.as_ref() != Some(&viewer_groups);
        sent.controls = Some(controls.clone());
        sent.viewer_groups = Some(viewer_groups.clone());
        (send_controls, send_groups)
    };

    if send_controls {
        crate::server::send_message(app, "controls-update", controls);
        // After the controls that asked for them: the server sends their values on.
        crate::server::set_forwarded(app, addresses(&groups).into_iter().collect());
    }
    if send_groups {
        crate::server::send_message(app, "viewer-groups", viewer_groups);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_of_every_type() {
        let groups = [json!({ "controls": [
            { "type": "boolean", "inputAddress": "/a" },
            { "type": "boolean-group", "inputs": [{ "inputAddress": "/b" }, { "inputAddress": "/c" }] },
            { "type": "boolean-enum", "inputs": [{ "inputAddress": { "true": ["/d"], "false": ["/e"] } }] },
            { "type": "slider", "inputAddress": "/f" },
            { "type": "preset", "presetId": "x" }
        ] })];
        assert_eq!(addresses(&groups).into_iter().collect::<Vec<_>>(), ["/a", "/b", "/c", "/d", "/e", "/f"]);
    }
}
