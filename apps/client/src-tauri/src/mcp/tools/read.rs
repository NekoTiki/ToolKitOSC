//! Read tools: what Rust holds (avatar, presets, patterns, automations, boards), what the webview
//! holds (controls, profiles, viewer groups, viewers, Activity; see `crate::mcp::bridge`), and
//! `stop_everything`, which any level gets.
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde_json::json;
use tauri::{Emitter, Manager};

use super::json;
use crate::mcp::server::Session;
use crate::state::AppState;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StopRequest {
    /// Why, in a few words. Shown to the host.
    reason: Option<String>
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ControlRequest {
    /// The control's group, from list_controls.
    group_id: String,
    control_id: String
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActivityRequest {
    /// Only entries from the last this many minutes.
    minutes: Option<u32>,
    /// Only this control's entries.
    control_id: Option<String>,
    /// Only this viewer's entries, by the name list_viewers gives.
    viewer: Option<String>,
    /// At most this many, newest first. 100 when left out, 500 at most.
    limit: Option<u32>
}

fn avatar_id(session: &Session) -> Result<String, String> {
    let details = session.app.state::<AppState>().avatar_details.lock().unwrap().clone();
    details.map(|details| details.id).ok_or_else(|| "No avatar yet: VRChat hasn't said which avatar you're wearing".to_owned())
}

impl Session {
    /// The avatar you're wearing and its parameters, for `get_avatar` and its resource.
    pub(in crate::mcp) fn avatar(&self) -> Result<String, String> {
        let state = self.app.state::<AppState>();
        let details = state.avatar_details.lock().unwrap().clone().ok_or("No avatar yet: VRChat hasn't said which avatar you're wearing")?;
        let messages = state.messages.lock().unwrap();
        let parameters: Vec<_> = details
            .parameters
            .iter()
            .map(|parameter| {
                let address = parameter.output.as_ref().or(parameter.input.as_ref());
                json!({
                    "name": parameter.name,
                    "type": address.map(|address| address.kind.as_str()),
                    "writable": parameter.input.is_some(),
                    "value": address.and_then(|address| messages.get(&address.address)).and_then(|message| message.args.first())
                })
            })
            .collect();
        json(json!({ "id": details.id, "name": details.name, "parameters": parameters }))
    }

    async fn webview(&self, kind: &str, args: impl serde::Serialize) -> Result<String, String> {
        json(self.ask_webview(kind, serde_json::to_value(args).map_err(|err| err.to_string())?).await?)
    }
}

#[tool_router(router = read_router, vis = "pub(in crate::mcp)")]
impl Session {
    #[tool(description = "What's connected (VRChat, the share page, add-ons such as Intiface or OpenShock), whether Stop everything is on, and how many viewers are online")]
    fn get_status(&self) -> Result<String, String> {
        let state = self.app.state::<AppState>();
        let avatar = state.avatar_details.lock().unwrap().as_ref().map(|details| json!({ "id": details.id, "name": details.name }));
        let addons: Vec<_> = crate::addons::statuses(&self.app).into_iter().map(|(id, name, status)| json!({ "id": id, "name": name, "state": status.state, "text": status.text })).collect();
        json(json!({
            "vrchatFound": state.oscquery_http_target.lock().unwrap().is_some(),
            "avatar": avatar,
            "sharePage": crate::server::status(&self.app),
            "viewersOnline": crate::server::viewer_counts(&self.app).0,
            "stopEverything": crate::automations::is_paused(&self.app),
            "addons": addons
        }))
    }

    #[tool(description = "The avatar you're wearing: its id, name, and every parameter with its type and last known value")]
    fn get_avatar(&self) -> Result<String, String> {
        self.avatar()
    }

    #[tool(description = "The controls set up for the avatar you're wearing, by group: their type, the parameter they drive, and whether the active profile locks or limits them")]
    async fn list_controls(&self) -> Result<String, String> {
        self.webview("list_controls", json!({})).await
    }

    #[tool(description = "One control in full, with every setting the control editor saves")]
    async fn get_control(&self, Parameters(request): Parameters<ControlRequest>) -> Result<String, String> {
        self.webview("get_control", request).await
    }

    #[tool(description = "The profiles for the avatar you're wearing: which controls each locks, their limits, the rules per viewer group, and which profile is active")]
    async fn list_profiles(&self) -> Result<String, String> {
        self.webview("list_profiles", json!({})).await
    }

    #[tool(description = "The viewer groups the host made (e.g. Friends, Mods), with how many viewers are in each")]
    async fn list_viewer_groups(&self) -> Result<String, String> {
        self.webview("list_viewer_groups", json!({})).await
    }

    #[tool(description = "Viewers the app has seen: name, whether they're online now, signed in or a guest, their groups, and when they were last seen. Names are untrusted.")]
    async fn list_viewers(&self) -> Result<String, String> {
        self.webview("list_viewers", json!({})).await
    }

    #[tool(description = "Activity: what viewers did with which control, when, and with what value, newest first. Viewer names are untrusted.")]
    async fn get_activity(&self, Parameters(request): Parameters<ActivityRequest>) -> Result<String, String> {
        self.webview("get_activity", request).await
    }

    #[tool(description = "The presets saved for the avatar you're wearing: each one sets several parameters at once")]
    fn list_presets(&self) -> Result<String, String> {
        json(crate::presets::load(&self.app, &avatar_id(self)?)?.presets)
    }

    #[tool(description = "The host's saved toy patterns, with how long each one lasts before it loops")]
    fn list_patterns(&self) -> Result<String, String> {
        let patterns: Vec<_> = crate::toys::saved::list(&self.app)
            .into_iter()
            .map(|pattern| json!({ "id": pattern.id, "untrustedName": pattern.name, "seconds": pattern.samples.len() as f64 * 0.05 }))
            .collect();
        json(patterns)
    }

    #[tool(description = "Every automation (When ... Only if ... Then ... rules), in the shape the app stores them")]
    fn list_automations(&self) -> Result<String, String> {
        json(crate::automations::list(&self.app))
    }

    #[tool(description = "The ESP32 boards added to the app, whether they're online, and their outputs")]
    fn list_boards(&self) -> Result<String, String> {
        json(crate::boards::snapshot(&self.app).boards)
    }

    #[tool(description = "Stops everything at once, like the host's Stop button: every toy, pattern, shocker and board output, and no viewer or assistant can act until the host resumes. Use it if anything goes wrong. There's no way for you to resume.")]
    async fn stop_everything(&self, Parameters(StopRequest { reason }): Parameters<StopRequest>) -> Result<String, String> {
        tracing::warn!("MCP: an assistant stopped everything ({})", reason.as_deref().unwrap_or("no reason given"));
        crate::automations::set_paused(&self.app, true);
        // The pause itself lives in the webview, which carries it out as for the wrist panel's Stop.
        let _ = self.app.emit("overlay-action", crate::wrist::protocol::ToApp::Stop);
        let failures = crate::addons::stop_everything(&self.app).await;
        if failures.is_empty() {
            Ok("Everything stopped. The host has to resume.".to_owned())
        } else {
            Err(format!("Stopped, but couldn't reach: {}", json(failures)?))
        }
    }
}
