//! The checks a viewer's command goes through, ported from
//! `packages/shared-ui/src/utils/controlLimits.ts`. The relay runs the TypeScript ones and the host
//! runs these, so they must agree: both are run against the same cases in
//! `controlLimits.vectors.json` (vitest there, `cargo test` here). Change one, change the other.
//!
//! Controls and commands stay JSON (`serde_json::Value`), as they arrive: every control type
//! passes through without a Rust copy of its shape.
use serde_json::{json, Map, Value};

pub const OFF_ID: &str = "off";
pub const VIEWER_PATTERN_ID: &str = "viewer";
pub const VIEWER_LIVE_ID: &str = "viewer-live";
pub const PATTERN_SAMPLE_MS: f64 = 50.0;
pub const PATTERN_MIN_MS: f64 = 1000.0;
pub const PATTERN_MAX_MS: f64 = 30000.0;

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

/// JS `===` for the JSON values options are keyed by: numbers compare by value (1 and 1.0 are the
/// same), everything else exactly.
pub fn same(a: &Value, b: &Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(a), Some(b)) => a == b,
        _ => a == b
    }
}

fn number(value: Option<&Value>) -> Option<f64> {
    value.and_then(Value::as_f64)
}

/// JS `Number.isInteger`, as a usable index.
fn index(value: Option<&Value>) -> Option<usize> {
    let value = number(value)?;
    (value.fract() == 0.0 && value >= 0.0).then_some(value as usize)
}

pub fn is_option_disabled(limits: Option<&Value>, key: &Value) -> bool {
    limits
        .and_then(|limits| limits.get("disabledOptions"))
        .and_then(Value::as_array)
        .is_some_and(|options| options.iter().any(|option| same(option, key)))
}

/// The range a viewer may set on a Slider or Toy, 0–1.
pub fn limit_range(limits: Option<&Value>) -> (f64, f64) {
    let min = clamp(number(limits.and_then(|l| l.get("min"))).unwrap_or(0.0), 0.0, 1.0);
    let max = clamp(number(limits.and_then(|l| l.get("max"))).unwrap_or(1.0), min, 1.0);
    (min, max)
}

/// Drops the parts that don't limit anything, so "no limits" is always `None`.
pub fn normalize_limits(limits: Option<&Value>) -> Option<Value> {
    let limits = limits.filter(|limits| limits.is_object())?;
    let mut normalized = Map::new();

    let min = number(limits.get("min")).filter(|min| *min > 0.0).map(|min| clamp(min, 0.0, 1.0));
    if let Some(min) = min {
        normalized.insert("min".into(), json!(min));
    }
    if let Some(max) = number(limits.get("max")).filter(|max| *max < 1.0) {
        normalized.insert("max".into(), json!(clamp(max, min.unwrap_or(0.0), 1.0)));
    }
    if let Some(options) = limits.get("disabledOptions").and_then(Value::as_array).filter(|options| !options.is_empty()) {
        normalized.insert("disabledOptions".into(), Value::Array(options.clone()));
    }

    (!normalized.is_empty()).then_some(Value::Object(normalized))
}

/// The most generous of several limits: the lowest min, the highest max, and only the options
/// every one blocks. Any "no limits" wins outright.
pub fn most_generous_limits(list: &[Option<Value>]) -> Option<Value> {
    if list.is_empty() || list.iter().any(|limits| normalize_limits(limits.as_ref()).is_none()) {
        return None;
    }

    let ranges: Vec<_> = list.iter().map(|limits| limit_range(limits.as_ref())).collect();
    let options: Vec<Vec<Value>> = list
        .iter()
        .map(|limits| limits.as_ref().and_then(|l| l.get("disabledOptions")).and_then(Value::as_array).cloned().unwrap_or_default())
        .collect();
    let (first, rest) = options.split_first()?;
    let disabled: Vec<Value> = first.iter().filter(|key| rest.iter().all(|keys| keys.iter().any(|k| same(k, key)))).cloned().collect();

    normalize_limits(Some(&json!({
        "min": ranges.iter().map(|r| r.0).fold(f64::INFINITY, f64::min),
        "max": ranges.iter().map(|r| r.1).fold(f64::NEG_INFINITY, f64::max),
        "disabledOptions": disabled
    })))
}

/// One viewer's copy of a control: the profile's rules for everyone, opened up by the rules of the
/// groups they're in, the most generous winning. Leaves out `groupRules` and `viaGroup` (the
/// share page's badge, which the host doesn't need). `unavailable` is kept: no group can use a toy
/// that's offline.
pub fn resolve_control_for_viewer(control: &Value, group_ids: &[String]) -> Value {
    let mut base = control.clone();
    let group_rules = base.as_object_mut().and_then(|map| {
        map.remove("viaGroup");
        map.remove("groupRules")
    });

    let everyone_locked = control.get("locked").and_then(Value::as_bool).unwrap_or(false);
    let everyone_limits = normalize_limits(control.get("limits"));
    let candidates: Vec<Option<Value>> = group_ids
        .iter()
        .filter_map(|id| group_rules.as_ref()?.get(id).filter(|rule| rule.is_object()))
        .filter_map(|rule| {
            let locked = rule.get("locked").and_then(Value::as_bool).unwrap_or(everyone_locked);
            let limits = match rule.get("limits") {
                Some(limits) if !limits.is_null() => normalize_limits(Some(limits)),
                _ => everyone_limits.clone()
            };
            (!locked).then_some(limits)
        })
        .collect();

    if candidates.is_empty() {
        return base;
    }

    let mut open = Vec::with_capacity(candidates.len() + 1);
    if !everyone_locked {
        open.push(everyone_limits);
    }
    open.extend(candidates);

    if let Some(map) = base.as_object_mut() {
        map.insert("locked".into(), Value::Bool(false));
        match most_generous_limits(&open) {
            Some(limits) => map.insert("limits".into(), limits),
            None => map.remove("limits")
        };
    }
    base
}

/// Checks a viewer's command against the control it targets (locks, availability and limits
/// included). Returns the command to run, with slider and toy values clamped into range, or `None`
/// to drop it.
pub fn sanitize_command(control: &Value, command: &Value) -> Option<Value> {
    let kind = command.get("type")?.as_str()?;
    if control.get("type").and_then(Value::as_str) != Some(kind)
        || control.get("locked").and_then(Value::as_bool).unwrap_or(false)
        || control.get("unavailable").and_then(Value::as_bool).unwrap_or(false)
    {
        return None;
    }

    let limits = control.get("limits").filter(|limits| !limits.is_null());
    let value = command.get("value");

    match kind {
        "boolean" | "boolean-group" => {
            let value = value?.as_bool()?;
            Some(json!({ "type": kind, "value": value }))
        }
        "boolean-enum" => {
            let at = index(value)?;
            let input = control.get("inputs")?.as_array()?.get(at)?;
            let id = input.get("id").cloned().unwrap_or(Value::Null);
            (!is_option_disabled(limits, &id)).then(|| json!({ "type": kind, "value": value }))
        }
        "enum" | "step-enum" => {
            let at = index(value)?;
            let option = control.get("options")?.as_array()?.get(at)?;
            let key = option.get("value").cloned().unwrap_or(Value::Null);
            (!is_option_disabled(limits, &key)).then(|| json!({ "type": kind, "value": value }))
        }
        "slider" | "intiface-toy" => {
            let value = number(value)?;
            let (min, max) = limit_range(limits);
            Some(json!({ "type": kind, "value": clamp(value, min, max) }))
        }
        "intiface-pattern" => {
            let id = value?.as_str()?;
            if id == OFF_ID {
                return Some(json!({ "type": kind, "value": id }));
            }

            if id == VIEWER_PATTERN_ID || id == VIEWER_LIVE_ID {
                let options = control.get("viewerPatterns").filter(|options| !options.is_null())?;
                if !options.get("enabled").and_then(Value::as_bool).unwrap_or(false) {
                    return None;
                }
                // Nothing above the control's own cap or the profile's max is ever played.
                let cap = number(options.get("maxIntensity")).map(|cap| clamp(cap, 0.0, 1.0)).unwrap_or(f64::NAN);
                let top = cap.min(limit_range(limits).1);

                if id == VIEWER_LIVE_ID {
                    let level = number(command.get("level"))?;
                    return Some(json!({ "type": kind, "value": id, "level": clamp(level, 0.0, top) }));
                }

                let samples = command.get("samples")?.as_array()?;
                let max_ms = number(options.get("maxMs")).unwrap_or(f64::NAN).min(PATTERN_MAX_MS);
                let max_samples = (max_ms / PATTERN_SAMPLE_MS).floor();
                let count = samples.len() as f64;
                if count * PATTERN_SAMPLE_MS < PATTERN_MIN_MS || count > max_samples {
                    return None;
                }
                let samples: Option<Vec<f64>> = samples.iter().map(Value::as_f64).collect();
                // JS Math.round: halves go up. Every sample is 0 or more here, so `round` agrees.
                let samples: Vec<f64> = samples?.into_iter().map(|sample| clamp(sample, 0.0, top * 100.0).round()).collect();
                return Some(json!({ "type": kind, "value": id, "samples": samples }));
            }

            let allowed = control
                .get("allowedPatterns")
                .and_then(Value::as_array)
                .is_some_and(|patterns| patterns.iter().any(|pattern| pattern.get("id").and_then(Value::as_str) == Some(id)));
            (allowed && !is_option_disabled(limits, &json!(id))).then(|| json!({ "type": kind, "value": id }))
        }
        "open-shock-shocker" | "preset" => Some(json!({ "type": kind })),
        _ => None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Numbers compare by value at any depth (the TypeScript side writes 50, Rust 50.0).
    fn equal(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Array(a), Value::Array(b)) => a.len() == b.len() && a.iter().zip(b).all(|(a, b)| equal(a, b)),
            (Value::Object(a), Value::Object(b)) => {
                a.len() == b.len() && a.iter().all(|(key, value)| b.get(key).is_some_and(|other| equal(value, other)))
            }
            _ => match (a.as_f64(), b.as_f64()) {
                (Some(a), Some(b)) => (a - b).abs() < 1e-12,
                _ => a == b
            }
        }
    }

    #[test]
    fn shared_vectors() {
        let cases: Vec<Value> =
            serde_json::from_str(include_str!("../../../../../packages/shared-ui/src/utils/controlLimits.vectors.json")).unwrap();
        assert!(!cases.is_empty());

        for case in cases {
            let name = case["name"].as_str().unwrap();
            let groups: Vec<String> = serde_json::from_value(case.get("groupIds").cloned().unwrap_or(json!([]))).unwrap();
            let resolved = resolve_control_for_viewer(&case["control"], &groups);

            if let Some(expected) = case.get("resolved") {
                let got = json!({
                    "locked": resolved.get("locked").and_then(Value::as_bool).unwrap_or(false),
                    "limits": resolved.get("limits").cloned().unwrap_or(Value::Null)
                });
                assert!(equal(&got, expected), "{name}: resolved {got} vs {expected}");
            }

            let got = sanitize_command(&resolved, &case["command"]).unwrap_or(Value::Null);
            assert!(equal(&got, &case["expected"]), "{name}: {got} vs {}", case["expected"]);
        }
    }
}
