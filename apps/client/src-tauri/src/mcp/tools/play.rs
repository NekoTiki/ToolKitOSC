//! Play tools: the assistant uses your controls like a viewer in the Assistants group (see
//! `crate::server::viewer_commands::assistant`), writes to the chatbox, sets raw parameters, and
//! switches profiles. Stop everything and `TKOSC/Assistants` hold them all back (see `guard`).
use std::sync::Mutex;
use std::time::{Duration, Instant};

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde_json::{json, Value};
use tauri::Manager;

use super::json;
use crate::mcp::guard;
use crate::mcp::server::Session;
use crate::state::AppState;
use crate::types::{OscArg, OscCommand};

/// VRChat's chatbox shows at most this many characters.
const CHATBOX_MAX: usize = 144;
/// One chatbox line per assistant this often, so it can't flood the room.
const CHATBOX_EVERY: Duration = Duration::from_secs(5);

static LAST_CHATBOX: Mutex<Option<Instant>> = Mutex::new(None);

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunRequest {
    /// The control's group, from list_controls.
    group_id: String,
    control_id: String,
    /// What to set, by the control's type: boolean and boolean-group true or false; slider and
    /// intiface-toy a number from 0 to 1; enum, step-enum and boolean-enum the index of the option
    /// or input; intiface-pattern the id of one of its allowedPatterns, or "off"; preset nothing.
    value: Option<Value>,
    /// Other fields a bhaptics or owo control needs, as the control's settings describe them.
    #[serde(default)]
    extra: serde_json::Map<String, Value>
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ChatboxRequest {
    /// Up to 144 characters.
    text: String
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ParameterRequest {
    /// The parameter's name, as get_avatar lists it (without /avatar/parameters/).
    name: String,
    /// true or false for a Bool, a whole number for an Int, a number from -1 to 1 for a Float.
    value: Value
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct ProfileRequest {
    /// The profile's id from list_profiles, or null for no profile.
    id: Option<String>
}

/// Whether any control drives this address: it names it somewhere in its settings.
fn mentions(value: &Value, address: &str) -> bool {
    match value {
        Value::String(text) => text == address,
        Value::Array(items) => items.iter().any(|item| mentions(item, address)),
        Value::Object(map) => map.values().any(|item| mentions(item, address)),
        _ => false
    }
}

/// A control locked for assistants in the active profile drives this address.
fn locked_for_assistants(controls: &[Value], address: &str) -> bool {
    let groups = [crate::mcp::ASSISTANTS_GROUP.to_owned()];
    controls
        .iter()
        .filter_map(|group| group.get("controls").and_then(Value::as_array))
        .flatten()
        .filter(|control| mentions(control, address))
        .any(|control| crate::server::limits::resolve_control_for_viewer(control, &groups).get("locked").and_then(Value::as_bool).unwrap_or(false))
}

#[tool_router(router = play_router, vis = "pub(in crate::mcp)")]
impl Session {
    #[tool(description = "Uses one of the host's controls, the way a viewer would from the share page, within what the active profile allows assistants. Not for shocker controls.")]
    fn run_control(&self, Parameters(request): Parameters<RunRequest>) -> Result<String, String> {
        let control = crate::server::find_control(&self.app, &request.group_id, &request.control_id).ok_or("No such control: get the ids from list_controls")?;
        let kind = control.get("type").cloned().unwrap_or(Value::Null);
        if kind == "open-shock-shocker" {
            return Err("Shocker controls aren't something this tool fires".to_owned());
        }
        let mut message = request.extra;
        message.insert("groupId".into(), json!(request.group_id));
        message.insert("controlId".into(), json!(request.control_id));
        message.insert("type".into(), kind);
        if let Some(value) = request.value {
            message.insert("value".into(), value);
        }
        json(crate::server::viewer_commands::assistant(&self.app, &self.name(), &Value::Object(message))?)
    }

    #[tool(description = "Posts a line to the VRChat chatbox above the host's avatar, for everyone nearby to read. At most once every 5 seconds.")]
    fn send_chatbox(&self, Parameters(ChatboxRequest { text }): Parameters<ChatboxRequest>) -> Result<String, String> {
        let text: String = text.chars().take(CHATBOX_MAX).collect();
        {
            let mut last = LAST_CHATBOX.lock().unwrap();
            if last.is_some_and(|at| at.elapsed() < CHATBOX_EVERY) {
                return Err("Too soon: one chatbox line every 5 seconds".to_owned());
            }
            *last = Some(Instant::now());
        }
        let message = rosc::OscMessage { addr: "/chatbox/input".into(), args: vec![rosc::OscType::String(text), rosc::OscType::Bool(true), rosc::OscType::Bool(false)] };
        crate::osc::send_message(&self.app.state::<AppState>(), message)?;
        Ok("Posted".to_owned())
    }

    #[tool(description = "Sets one of the avatar's parameters directly, without a control. Refused for the app's own TKOSC/ parameters and for any parameter a control locked for assistants drives.")]
    fn set_parameter(&self, Parameters(ParameterRequest { name, value }): Parameters<ParameterRequest>) -> Result<String, String> {
        if guard::parameter_reserved(&name) {
            return Err("That's one of ToolKitOSC's own parameters".to_owned());
        }
        let address = format!("/avatar/parameters/{name}");
        if crate::server::with_control_groups(&self.app, |controls| locked_for_assistants(controls, &address)) {
            return Err("A control the active profile locks for assistants drives that parameter".to_owned());
        }
        let arg = match value {
            Value::Bool(on) => OscArg::Bool(on),
            Value::Number(number) => OscArg::Number(number.as_f64().ok_or("Not a number")?),
            _ => return Err("The value must be true, false or a number".to_owned())
        };
        crate::osc::send(&self.app.state::<AppState>(), &OscCommand { address, args: vec![arg] })?;
        Ok("Set".to_owned())
    }

    #[tool(description = "Switches the active profile, which decides what everyone may do with the controls")]
    async fn switch_profile(&self, Parameters(request): Parameters<ProfileRequest>) -> Result<String, String> {
        self.ask_webview("switch_profile", serde_json::to_value(request).map_err(|err| err.to_string())?).await?;
        Ok("Switched".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_locked_control_guards_its_parameters() {
        let controls = [json!({ "id": "g", "controls": [
            { "id": "ears", "type": "boolean", "inputAddress": "/avatar/parameters/Ears", "locked": true },
            { "id": "tail", "type": "boolean", "inputAddress": "/avatar/parameters/Tail", "locked": true,
              "groupRules": { "assistants": { "locked": false } } },
            { "id": "hat", "type": "boolean-enum", "inputs": [{ "id": "a", "inputAddress": { "true": ["/avatar/parameters/Hat"], "false": [] } }] }
        ]})];
        assert!(locked_for_assistants(&controls, "/avatar/parameters/Ears"));
        assert!(!locked_for_assistants(&controls, "/avatar/parameters/Tail"), "the Assistants group opens it");
        assert!(!locked_for_assistants(&controls, "/avatar/parameters/Hat"));
        assert!(!locked_for_assistants(&controls, "/avatar/parameters/Nothing"));
    }
}
