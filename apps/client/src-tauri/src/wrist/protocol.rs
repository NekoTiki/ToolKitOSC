//! What the app and the panel say to each other over the localhost WebSocket, as JSON. The app
//! sends the whole state whenever it changes (it's small); the panel sends what you pressed.
use serde::{Deserialize, Serialize};

use crate::overlay::model::PanelSettings;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LastAction {
    /// Unix time in ms.
    pub at: u64,
    pub text: String
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub paused: bool,
    /// Viewers on the share page now, and how many of them signed in.
    pub watching: usize,
    pub signed_in: usize,
    /// The active profile, None for no profile.
    pub profile: Option<String>,
    pub profiles: Vec<Profile>,
    pub last: Option<LastAction>,
    /// Viewer actions in the last minute.
    pub recent: usize,
    pub panel: PanelSettings
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ToApp {
    /// First, with the token the app passed on the command line.
    Hello { token: String },
    Stop,
    /// Only after the hold on Resume.
    Resume,
    Profile { id: Option<String> }
}
