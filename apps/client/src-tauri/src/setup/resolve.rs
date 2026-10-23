//! The controls as everything else sees them: each stored control with the active profile's
//! `locked`, `limits` and `groupRules`, whether it's `unavailable` (and why), and `paused` while
//! avatar touch drives its toy. Was `useControls`' `controls` computed in the webview.
use serde_json::{json, Map, Value};

use super::Profile;

/// Fields `resolve` adds. Never saved.
pub const DERIVED: [&str; 6] = ["locked", "limits", "groupRules", "unavailable", "unavailableReason", "paused"];

/// What `resolve` needs to know about the rest of the app right now.
pub trait Live {
    /// The add-on a control type belongs to, by name, when it's off.
    fn addon_off(&self, control_type: &str) -> Option<String>;
    fn openshock_available(&self) -> bool;
    /// Whether every hub these shockers are paired to is offline (false while that isn't known).
    fn hubs_offline(&self, shocker_ids: &[String]) -> bool;
    fn intiface_connected(&self) -> bool;
    fn has_device(&self, index: u64) -> bool;
    fn touched(&self, device_index: u64, actuator_index: u64) -> bool;
    fn preset_exists(&self, id: &str) -> bool;
    fn bhaptics_connected(&self) -> bool;
    fn owo_connected(&self) -> bool;
    /// A recorded pattern's current name and icon.
    fn saved_pattern(&self, id: &str) -> Option<(String, Option<String>)>;
    fn builtin_pattern(&self, id: &str) -> bool;
    /// Profiles keep their rules for a deleted viewer group; those don't count.
    fn viewer_group_exists(&self, id: &str) -> bool;
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn actuators(control: &Value) -> impl Iterator<Item = (u64, u64)> + '_ {
    control
        .get("actuators")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|actuator| Some((actuator.get("deviceIndex")?.as_u64()?, actuator.get("actuatorIndex")?.as_u64()?)))
}

fn strings(control: &Value, key: &str) -> Vec<String> {
    control.get(key).and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect()
}

/// The stored control without the fields `resolve` adds, as it's saved.
pub fn strip(control: &mut Value) {
    if let Some(control) = control.as_object_mut() {
        for key in DERIVED {
            control.remove(key);
        }
    }
}

/// A control saved before `intiface-vibrator` (with `vibrators`) became `intiface-toy` (with
/// `actuators`).
pub fn upgrade(control: &mut Value) {
    if text(control, "type") != Some("intiface-vibrator") {
        return;
    }
    if let Some(control) = control.as_object_mut() {
        control.insert("type".into(), json!("intiface-toy"));
        if let Some(vibrators) = control.remove("vibrators") {
            control.insert("actuators".into(), vibrators);
        }
    }
}

fn unavailable(control: &Value, live: &impl Live) -> (bool, Option<String>) {
    let kind = text(control, "type").unwrap_or_default();
    if let Some(addon) = live.addon_off(kind) {
        return (true, Some(format!("{addon} is off")));
    }
    match kind {
        "open-shock-shocker" => {
            let offline = live.openshock_available() && live.hubs_offline(&strings(control, "shockers"));
            (!live.openshock_available() || offline, offline.then(|| "Hub offline".to_owned()))
        }
        // A control on several toys stays available while one of them is there.
        "intiface-toy" | "intiface-pattern" => (!live.intiface_connected() || actuators(control).all(|(device, _)| !live.has_device(device)), None),
        "preset" => (!live.preset_exists(text(control, "presetId").unwrap_or_default()), None),
        "bhaptics" => (!live.bhaptics_connected(), None),
        "owo" => (!live.owo_connected(), None),
        _ => (false, None)
    }
}

/// Recorded patterns can be renamed or deleted after a control picked them: refreshes the copies
/// and drops the missing ones.
fn resolve_patterns(refs: &Value, live: &impl Live) -> Value {
    let refs = refs.as_array().into_iter().flatten().filter_map(|pattern| {
        let id = text(pattern, "id")?;
        match live.saved_pattern(id) {
            Some((name, Some(icon))) => Some(json!({ "id": id, "name": name, "icon": icon })),
            Some((name, None)) => Some(json!({ "id": id, "name": name })),
            None => live.builtin_pattern(id).then(|| pattern.clone())
        }
    });
    Value::Array(refs.collect())
}

fn resolve_control(control: &Value, profile: Option<&Profile>, live: &impl Live) -> Value {
    let mut control = control.clone();
    let id = text(&control, "id").unwrap_or_default().to_owned();
    let kind = text(&control, "type").unwrap_or_default().to_owned();
    let (is_unavailable, reason) = unavailable(&control, live);
    let paused = matches!(kind.as_str(), "intiface-toy" | "intiface-pattern") && actuators(&control).any(|(device, actuator)| live.touched(device, actuator));
    let group_rules: Map<String, Value> = profile
        .map(|profile| {
            profile
                .group_overrides
                .iter()
                .filter(|(group, _)| live.viewer_group_exists(group))
                .filter_map(|(group, rules)| Some((group.clone(), rules.get(&id)?.clone())))
                .collect()
        })
        .unwrap_or_default();

    let Some(fields) = control.as_object_mut() else {
        return control;
    };
    if kind == "intiface-pattern" {
        let patterns = resolve_patterns(fields.get("allowedPatterns").unwrap_or(&Value::Null), live);
        fields.insert("allowedPatterns".into(), patterns);
    }
    fields.insert("locked".into(), json!(profile.and_then(|profile| profile.locked_controls.get(&id).copied()).unwrap_or(false)));
    if let Some(limits) = profile.and_then(|profile| profile.limits.get(&id)) {
        fields.insert("limits".into(), limits.clone());
    }
    if !group_rules.is_empty() {
        fields.insert("groupRules".into(), Value::Object(group_rules));
    }
    if let Some(reason) = reason {
        fields.insert("unavailableReason".into(), json!(reason));
    }
    if paused {
        fields.insert("paused".into(), json!(true));
    }
    fields.insert("unavailable".into(), json!(is_unavailable));
    control
}

pub fn resolve(groups: &[Value], profile: Option<&Profile>, live: &impl Live) -> Vec<Value> {
    groups
        .iter()
        .map(|group| {
            let mut group = group.clone();
            if let Some(controls) = group.get_mut("controls").and_then(Value::as_array_mut) {
                for control in controls.iter_mut() {
                    *control = resolve_control(control, profile, live);
                }
            }
            group
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Fake {
        off: Vec<(&'static str, &'static str)>,
        openshock: bool,
        offline: bool,
        intiface: bool,
        devices: Vec<u64>,
        touched: Vec<(u64, u64)>,
        presets: Vec<&'static str>,
        bhaptics: bool,
        owo: bool
    }

    impl Live for Fake {
        fn addon_off(&self, control_type: &str) -> Option<String> {
            self.off.iter().find(|(kind, _)| *kind == control_type).map(|(_, name)| name.to_string())
        }
        fn openshock_available(&self) -> bool {
            self.openshock
        }
        fn hubs_offline(&self, _: &[String]) -> bool {
            self.offline
        }
        fn intiface_connected(&self) -> bool {
            self.intiface
        }
        fn has_device(&self, index: u64) -> bool {
            self.devices.contains(&index)
        }
        fn touched(&self, device: u64, actuator: u64) -> bool {
            self.touched.contains(&(device, actuator))
        }
        fn preset_exists(&self, id: &str) -> bool {
            self.presets.contains(&id)
        }
        fn bhaptics_connected(&self) -> bool {
            self.bhaptics
        }
        fn owo_connected(&self) -> bool {
            self.owo
        }
        fn saved_pattern(&self, id: &str) -> Option<(String, Option<String>)> {
            (id == "mine").then(|| ("Renamed".to_owned(), None))
        }
        fn builtin_pattern(&self, id: &str) -> bool {
            id == "pulse"
        }
        fn viewer_group_exists(&self, id: &str) -> bool {
            id != "deleted"
        }
    }

    fn one(control: Value, profile: Option<&Profile>, live: &Fake) -> Value {
        resolve(&[json!({ "id": "g", "controls": [control] })], profile, live)[0]["controls"][0].clone()
    }

    fn profile() -> Profile {
        serde_json::from_value(json!({
            "id": "p",
            "name": "Strict",
            "lockedControls": { "a": true },
            "limits": { "toy": { "max": 0.5 } },
            "groupOverrides": { "friends": { "toy": { "limits": { "max": 1 } } }, "deleted": { "toy": { "locked": true } } }
        }))
        .unwrap()
    }

    #[test]
    fn applies_the_profile() {
        let profile = profile();
        let live = Fake { intiface: true, devices: vec![0], ..Default::default() };
        let locked = one(json!({ "id": "a", "type": "boolean" }), Some(&profile), &live);
        assert_eq!(locked["locked"], json!(true));
        assert_eq!(locked["unavailable"], json!(false));

        let toy = one(json!({ "id": "toy", "type": "intiface-toy", "actuators": [{ "deviceIndex": 0, "actuatorIndex": 0 }] }), Some(&profile), &live);
        assert_eq!(toy["locked"], json!(false));
        assert_eq!(toy["limits"], json!({ "max": 0.5 }));
        assert_eq!(toy["groupRules"], json!({ "friends": { "limits": { "max": 1 } } }));

        let none = one(json!({ "id": "toy", "type": "boolean" }), None, &live);
        assert!(none.get("limits").is_none() && none.get("groupRules").is_none());
    }

    #[test]
    fn unavailable_per_type() {
        let toy = json!({ "id": "t", "type": "intiface-toy", "actuators": [{ "deviceIndex": 1, "actuatorIndex": 0 }, { "deviceIndex": 2, "actuatorIndex": 0 }] });
        assert_eq!(one(toy.clone(), None, &Fake::default())["unavailable"], json!(true));
        assert_eq!(one(toy.clone(), None, &Fake { intiface: true, devices: vec![2], ..Default::default() })["unavailable"], json!(false));
        let touched = one(toy, None, &Fake { intiface: true, devices: vec![1], touched: vec![(1, 0)], ..Default::default() });
        assert_eq!(touched["paused"], json!(true));

        let shocker = json!({ "id": "s", "type": "open-shock-shocker", "shockers": ["x"] });
        assert_eq!(one(shocker.clone(), None, &Fake::default())["unavailable"], json!(true));
        let offline = one(shocker.clone(), None, &Fake { openshock: true, offline: true, ..Default::default() });
        assert_eq!((offline["unavailable"].clone(), offline["unavailableReason"].clone()), (json!(true), json!("Hub offline")));
        assert_eq!(one(shocker, None, &Fake { openshock: true, ..Default::default() })["unavailable"], json!(false));

        let preset = json!({ "id": "p", "type": "preset", "presetId": "x" });
        assert_eq!(one(preset.clone(), None, &Fake::default())["unavailable"], json!(true));
        assert_eq!(one(preset, None, &Fake { presets: vec!["x"], ..Default::default() })["unavailable"], json!(false));

        assert_eq!(one(json!({ "id": "b", "type": "bhaptics" }), None, &Fake::default())["unavailable"], json!(true));
        assert_eq!(one(json!({ "id": "o", "type": "owo" }), None, &Fake { owo: true, ..Default::default() })["unavailable"], json!(false));

        let off = one(json!({ "id": "o", "type": "owo" }), None, &Fake { owo: true, off: vec![("owo", "OWO Skin")], ..Default::default() });
        assert_eq!((off["unavailable"].clone(), off["unavailableReason"].clone()), (json!(true), json!("OWO Skin is off")));
    }

    #[test]
    fn refreshes_pattern_refs() {
        let control = json!({ "id": "p", "type": "intiface-pattern", "actuators": [], "allowedPatterns": [
            { "id": "mine", "name": "Old" }, { "id": "pulse", "name": "Pulse" }, { "id": "gone", "name": "Gone" }
        ] });
        let resolved = one(control, None, &Fake::default());
        assert_eq!(resolved["allowedPatterns"], json!([{ "id": "mine", "name": "Renamed" },{ "id": "pulse", "name": "Pulse" }]));
    }

    #[test]
    fn strip_and_upgrade() {
        let mut control = one(json!({ "id": "a", "type": "intiface-vibrator", "vibrators": [] }), Some(&profile()), &Fake::default());
        strip(&mut control);
        upgrade(&mut control);
        assert_eq!(control, json!({ "id": "a", "type": "intiface-toy", "actuators": [] }));
    }
}
