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
    only_if: Vec<Guard>
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
    async fn create_controls(&self, Parameters(request): Parameters<ControlsRequest>) -> Result<String, String> {
        json(self.ask_webview("create_controls", serde_json::to_value(request).map_err(|err| err.to_string())?).await?)
    }

    #[tool(description = "Changes some settings of a control. While Edit is on Ask, the host sees the control and your changes before allowing it.")]
    async fn update_control(&self, Parameters(request): Parameters<UpdateControlRequest>) -> Result<String, String> {
        json(self.ask_webview("update_control", serde_json::to_value(request).map_err(|err| err.to_string())?).await?)
    }

    #[tool(description = "Saves a profile as a draft: which controls it locks, their limits, and the rules per viewer group. It isn't active until the host switches to it.")]
    async fn save_profile(&self, Parameters(request): Parameters<ProfileRequest>) -> Result<String, String> {
        json(self.ask_webview("save_profile", serde_json::to_value(request).map_err(|err| err.to_string())?).await?)
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
            DraftKind::Control | DraftKind::Profile => {
                self.ask_webview("delete_draft", serde_json::to_value(&request).map_err(|err| err.to_string())?).await?;
            }
        }
        Ok("Deleted".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_assistant_that_made_a_draft_owns_it() {
        let draft = CreatedBy { assistant: "claude-code".into(), at: 0 };
        assert!(mine(Some(&draft), "claude-code"));
        assert!(!mine(Some(&draft), "cursor"));
        assert!(!mine(None, "claude-code"), "kept or made by the host");
    }
}
