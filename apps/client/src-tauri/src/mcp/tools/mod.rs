//! The tools, one file per level in Settings › Assistants (see `super::guard::level_of`).
mod read;

/// A tool's answer as JSON text, the way MCP clients show it to the model.
fn json(value: impl serde::Serialize) -> Result<String, String> {
    serde_json::to_string(&value).map_err(|err| err.to_string())
}
