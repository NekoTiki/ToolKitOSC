//! The tools, one file per level in Settings › Assistants (see `super::guard::level_of`).
mod edit;
mod play;
mod read;

/// tkosc://docs/automations: the shape `save_automation` takes, from the same types the app saves.
pub fn automation_docs() -> String {
    serde_json::to_string_pretty(&schemars::schema_for!(edit::AutomationRequest)).unwrap_or_default()
}

/// tkosc://docs/control-types: generated from shared-ui's `ControlType` by `npm run schema:controls`.
pub const CONTROL_TYPES_DOCS: &str = include_str!("../../../../../../packages/shared-ui/control-types.schema.json");

/// A tool's answer as JSON text, the way MCP clients show it to the model.
fn json(value: impl serde::Serialize) -> Result<String, String> {
    serde_json::to_string(&value).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn docs_describe_the_shapes() {
        let automations = super::automation_docs();
        assert!(automations.contains("\"when\"") && automations.contains("avatar-parameter"), "{automations}");
        let controls: serde_json::Value = serde_json::from_str(super::CONTROL_TYPES_DOCS).unwrap();
        assert!(controls.pointer("/definitions/ControlType/anyOf").is_some());
    }
}
