//! Edit tools: the assistant makes drafts. New automations are saved turned off, new controls go
//! into a hidden "From <assistant>" group, and everything it makes carries `createdBy` until you
//! press Keep or edit it yourself. It may change or delete only its own drafts, except
//! `update_control`, which shows you the change first while Edit is on Ask.
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde_json::{json, Value};

use super::json;
use crate::automations::model::{Automation, Guard, Retrigger, Step, Trigger};
use crate::mcp::server::Session;
use crate::types::CreatedBy;

/// The longest pattern: 30 s of 50 ms samples.
const MAX_PATTERN_SAMPLES: usize = 600;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRequest {
    /// Leave out for a new one. Only automations you made, and the host hasn't kept, can be changed.
    id: Option<String>,
    name: String,
    when: Trigger,
    /// Up to 8 steps, run together, each after its own delay.
    then: Vec<Step>,
    #[serde(default)]
    cooldown_ms: u32,
    #[serde(default)]
    retrigger: Retrigger,
    /// Up to 4 conditions that all have to be true for it to start.
    #[serde(default)]
    only_if: Vec<Guard>,
    /// One of the onlyIf conditions is enough, instead of all of them.
    #[serde(default)]
    only_if_any: bool
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct IdRequest {
    id: String
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PatternRequest {
    /// Leave out for a new one. Only patterns you made, and the host hasn't kept, can be changed.
    id: Option<String>,
    name: String,
    /// Intensities from 0 to 1, one every 50 ms, from 1 to 30 seconds' worth. It loops.
    samples: Vec<f64>
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct ControlsRequest {
    /// New controls, shaped as tkosc://docs/control-types describes, without ids: they get new
    /// ones. They go into a hidden group named after you until the host keeps them.
    controls: Vec<Value>
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateControlRequest {
    group_id: String,
    control_id: String,
    /// Fields to change, merged into the control. id and type can't change.
    changes: serde_json::Map<String, Value>
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfileRequest {
    /// Leave out for a new one. Only profiles you made, and the host hasn't kept, can be changed.
    id: Option<String>,
    name: String,
    /// Control id -> true to lock it for everyone.
    #[serde(default)]
    locked_controls: serde_json::Map<String, Value>,
    /// Control id -> limits, as list_profiles shows them.
    #[serde(default)]
    limits: serde_json::Map<String, Value>,
    /// Viewer group id -> control id -> rule, as list_profiles shows them.
    #[serde(default)]
    group_overrides: serde_json::Map<String, Value>
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DraftKind {
    Control,
    Profile,
    Automation,
    Pattern
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRequest {
    kind: DraftKind,
    id: String,
    /// For a control: its group.
    group_id: Option<String>
}

/// Every control type, as in shared-ui's `KNOWN_CONTROL_TYPES`.
const CONTROL_TYPES: [&str; 12] =
    ["boolean", "boolean-group", "boolean-enum", "enum", "slider", "step-enum", "open-shock-shocker", "intiface-toy", "intiface-pattern", "preset", "bhaptics", "owo"];

/// A stored control of these groups, to change.
fn find_mut<'a>(groups: &'a mut [Value], group_id: &str, control_id: &str) -> Option<&'a mut Value> {
    groups
        .iter_mut()
        .find(|group| group.get("id").and_then(Value::as_str) == Some(group_id))?
        .get_mut("controls")?
        .as_array_mut()?
        .iter_mut()
        .find(|control| control.get("id").and_then(Value::as_str) == Some(control_id))
}

fn new_id() -> String {
    format!("{:016x}", rand::random::<u64>())
}

fn now_ms() -> u64 {
    chrono::Utc::now().timestamp_millis() as u64
}

/// Whether this draft is still this assistant's.
fn mine(created_by: Option<&CreatedBy>, name: &str) -> bool {
    created_by.is_some_and(|created_by| created_by.assistant == name)
}

impl Session {
    fn created_by(&self) -> CreatedBy {
        CreatedBy { assistant: self.name(), at: now_ms() }
    }

    /// The id to save under: a new one, or `id` when it's one of this assistant's drafts.
    fn draft_id(&self, id: Option<String>, existing: impl FnOnce(&str) -> Option<Option<CreatedBy>>) -> Result<String, String> {
        let Some(id) = id else {
            return Ok(new_id());
        };
        match existing(&id) {
            Some(created_by) if mine(created_by.as_ref(), &self.name()) => Ok(id),
            Some(_) => Err("That one isn't your draft: the host made it, or kept it".to_owned()),
            None => Err("No such id: leave id out to make a new one".to_owned())
        }
    }
}

#[tool_router(router = edit_router, vis = "pub(in crate::mcp)")]
impl Session {
    #[tool(description = "Saves an automation as a draft, turned off: the host turns it on after looking at it. Read tkosc://docs/automations for the shapes of when, then and onlyIf.")]
    fn save_automation(&self, Parameters(request): Parameters<AutomationRequest>) -> Result<String, String> {
        let id = self.draft_id(request.id, |id| crate::automations::list(&self.app).into_iter().find(|a| a.id == id).map(|a| a.created_by))?;
        let automation = Automation {
            id: id.clone(),
            name: request.name,
            enabled: false,
            when: request.when,
            then: request.then,
            cooldown_ms: request.cooldown_ms,
            retrigger: request.retrigger,
            only_if: request.only_if,
            only_if_any: request.only_if_any,
            created_by: Some(self.created_by())
        };
        crate::automations::save(&self.app, automation)?;
        json(json!({ "id": id, "enabled": false }))
    }

    #[tool(description = "Runs one of your draft automations' steps once, like the Test button, to check it does what you meant")]
    fn test_automation(&self, Parameters(IdRequest { id }): Parameters<IdRequest>) -> Result<String, String> {
        let automation = crate::automations::list(&self.app).into_iter().find(|a| a.id == id).ok_or("No such automation")?;
        if !mine(automation.created_by.as_ref(), &self.name()) {
            return Err("Only your own drafts can be tested".to_owned());
        }
        crate::automations::test(&self.app, &id)?;
        Ok("Ran its steps once".to_owned())
    }

    #[tool(description = "Saves a toy pattern as a draft, from intensities every 50 ms. Toy pattern controls can then offer it.")]
    fn save_pattern(&self, Parameters(request): Parameters<PatternRequest>) -> Result<String, String> {
        if request.samples.len() < 20 || request.samples.len() > MAX_PATTERN_SAMPLES {
            return Err("From 20 to 600 samples: 1 to 30 seconds".to_owned());
        }
        let id = self.draft_id(request.id, |id| crate::toys::saved::list(&self.app).into_iter().find(|p| p.id == id).map(|p| p.created_by))?;
        let samples = request.samples.iter().map(|sample| (sample.clamp(0.0, 1.0) * 100.0).round()).collect();
        crate::toys::saved::save(
            &self.app,
            crate::toys::saved::SavedPattern { id: id.clone(), name: request.name, icon: None, samples, updated_at: now_ms(), created_by: Some(self.created_by()) }
        );
        json(json!({ "id": id }))
    }

    #[tool(description = "Adds controls for the avatar you're wearing, as drafts in a hidden group named after you, so viewers don't see them until the host keeps them")]
    fn create_controls(&self, Parameters(request): Parameters<ControlsRequest>) -> Result<String, String> {
        let invalid = request.controls.iter().position(|control| {
            control.get("name").and_then(Value::as_str).is_none() || !control.get("type").and_then(Value::as_str).is_some_and(|kind| CONTROL_TYPES.contains(&kind))
        });
        if request.controls.is_empty() || invalid.is_some() {
            return Err(format!("Control {} needs a name and a type from tkosc://docs/control-types", invalid.map_or(1, |at| at + 1)));
        }
        let created_by = serde_json::to_value(self.created_by()).map_err(|err| err.to_string())?;
        let group_name = format!("From {}", self.name());
        let (group_id, ids) = crate::setup::edit_worn(&self.app, |file| {
            let at = match file.groups.iter().position(|group| group.get("name").and_then(Value::as_str) == Some(&group_name)) {
                Some(at) => at,
                None => {
                    file.groups.push(json!({ "id": new_id(), "name": group_name, "hidden": true, "controls": [] }));
                    file.groups.len() - 1
                }
            };
            let group = &mut file.groups[at];
            let group_id = group.get("id").and_then(Value::as_str).unwrap_or_default().to_owned();
            let controls = group.get_mut("controls").and_then(Value::as_array_mut).ok_or("That group has no controls list")?;
            let mut ids = Vec::new();
            for mut control in request.controls {
                crate::setup::strip_derived(&mut control);
                let id = new_id();
                control["id"] = json!(id);
                control["createdBy"] = created_by.clone();
                controls.push(control);
                ids.push(id);
            }
            Ok((group_id, ids))
        })?;
        json(json!({ "groupId": group_id, "controlIds": ids }))
    }

    #[tool(description = "Changes some settings of a control. While Edit is on Ask, the host sees the control and your changes before allowing it.")]
    fn update_control(&self, Parameters(request): Parameters<UpdateControlRequest>) -> Result<String, String> {
        crate::setup::edit_worn(&self.app, |file| {
            let control = find_mut(&mut file.groups, &request.group_id, &request.control_id).ok_or("No such control: get the ids from list_controls")?;
            let fields = control.as_object_mut().ok_or("That control isn't an object")?;
            for (key, value) in request.changes {
                if !matches!(key.as_str(), "id" | "type" | "createdBy") {
                    fields.insert(key, value);
                }
            }
            crate::setup::strip_derived(control);
            Ok(())
        })?;
        json(Value::Null)
    }

    #[tool(description = "Saves a profile as a draft: which controls it locks, their limits, and the rules per viewer group. It isn't active until the host switches to it.")]
    fn save_profile(&self, Parameters(request): Parameters<ProfileRequest>) -> Result<String, String> {
        let snapshot = crate::setup::snapshot(&self.app);
        let id = self.draft_id(request.id, |id| snapshot.profiles.iter().find(|p| p.id == id).map(|p| p.created_by.clone()))?;
        let group_overrides = request
            .group_overrides
            .into_iter()
            .map(|(group, rules)| Ok((group, serde_json::from_value(rules).map_err(|_| "groupOverrides is viewer group id -> control id -> rule")?)))
            .collect::<Result<_, String>>()?;
        let profile = crate::setup::Profile {
            id: id.clone(),
            name: request.name,
            locked_controls: request.locked_controls.into_iter().filter_map(|(control, locked)| Some((control, locked.as_bool()?))).collect(),
            limits: request.limits.into_iter().collect(),
            group_overrides,
            created_by: Some(self.created_by())
        };
        crate::setup::edit_worn(&self.app, |file| {
            match file.profiles.iter_mut().find(|existing| existing.id == profile.id) {
                Some(existing) => *existing = profile,
                None => file.profiles.push(profile)
            }
            Ok(())
        })?;
        json(json!({ "id": id }))
    }

    #[tool(description = "Deletes one of your drafts. Anything the host made, kept or edited stays.")]
    async fn delete_draft(&self, Parameters(request): Parameters<DeleteRequest>) -> Result<String, String> {
        let name = self.name();
        match request.kind {
            DraftKind::Automation => {
                let automation = crate::automations::list(&self.app).into_iter().find(|a| a.id == request.id).ok_or("No such automation")?;
                if !mine(automation.created_by.as_ref(), &name) {
                    return Err("That one isn't your draft".to_owned());
                }
                crate::automations::delete(&self.app, &request.id);
            }
            DraftKind::Pattern => {
                let pattern = crate::toys::saved::list(&self.app).into_iter().find(|p| p.id == request.id).ok_or("No such pattern")?;
                if !mine(pattern.created_by.as_ref(), &name) {
                    return Err("That one isn't your draft".to_owned());
                }
                crate::toys::saved::delete(&self.app, &request.id);
            }
            DraftKind::Control => {
                let group_id = request.group_id.as_deref().ok_or("A control needs its groupId")?;
                crate::setup::edit_worn(&self.app, |file| {
                    let control = find_mut(&mut file.groups, group_id, &request.id).ok_or("No such control")?;
                    let created_by = control.get("createdBy").cloned().and_then(|value| serde_json::from_value::<CreatedBy>(value).ok());
                    if !mine(created_by.as_ref(), &name) {
                        return Err("That one isn't your draft".to_owned());
                    }
                    for group in file.groups.iter_mut().filter(|group| group.get("id").and_then(Value::as_str) == Some(group_id)) {
                        if let Some(controls) = group.get_mut("controls").and_then(Value::as_array_mut) {
                            controls.retain(|control| control.get("id").and_then(Value::as_str) != Some(request.id.as_str()));
                        }
                    }
                    Ok(())
                })?;
            }
            DraftKind::Profile => {
                let active = crate::setup::snapshot(&self.app).active_profile;
                crate::setup::edit_worn(&self.app, |file| {
                    let profile = file.profiles.iter().find(|p| p.id == request.id).ok_or("No such profile")?;
                    if !mine(profile.created_by.as_ref(), &name) {
                        return Err("That one isn't your draft".to_owned());
                    }
                    file.profiles.retain(|p| p.id != request.id);
                    if file.default_profile.as_deref() == Some(request.id.as_str()) {
                        file.default_profile = None;
                    }
                    Ok(())
                })?;
                // Deleting the profile that's on leaves none on.
                if active.as_deref() == Some(request.id.as_str()) {
                    crate::setup::switch_profile(&self.app, None)?;
                }
            }
        }
        Ok("Deleted".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_types_match_the_docs() {
        // Every `const` in the generated schema is a control's `type`.
        fn consts(value: &Value, found: &mut std::collections::BTreeSet<String>) {
            match value {
                Value::Object(map) => {
                    if let Some(Value::String(kind)) = map.get("const") {
                        found.insert(kind.clone());
                    }
                    map.values().for_each(|value| consts(value, found));
                }
                Value::Array(items) => items.iter().for_each(|value| consts(value, found)),
                _ => {}
            }
        }
        let mut documented = std::collections::BTreeSet::new();
        consts(&serde_json::from_str(super::super::CONTROL_TYPES_DOCS).unwrap(), &mut documented);
        assert_eq!(CONTROL_TYPES.iter().map(|kind| kind.to_string()).collect::<std::collections::BTreeSet<_>>(), documented);
    }

    #[test]
    fn only_the_assistant_that_made_a_draft_owns_it() {
        let draft = CreatedBy { assistant: "claude-code".into(), at: 0 };
        assert!(mine(Some(&draft), "claude-code"));
        assert!(!mine(Some(&draft), "cursor"));
        assert!(!mine(None, "claude-code"), "kept or made by the host");
    }
}
