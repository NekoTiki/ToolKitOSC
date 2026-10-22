//! What the app and the panel say to each other over the localhost WebSocket, as JSON. The app
//! sends the whole state whenever it changes (it's small); the panel sends what you pressed.
use serde::{Deserialize, Serialize};

use crate::overlay::model::{HudSettings, PanelSettings};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String
}

/// Something a viewer did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// Unix time in ms.
    pub at: u64,
    pub who: String,
    /// After the name: "turned on Ears".
    pub what: String
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Viewer {
    pub name: String,
    /// The names of the viewer groups they're in.
    pub groups: Vec<String>,
    pub signed_in: bool,
    /// What they did last.
    pub last: Option<Entry>
}

/// How a pinned tile works.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum TileKind {
    /// A press switches it.
    Toggle { on: bool },
    /// A press goes to the next option: an enum, or which of a boolean-enum's inputs is on.
    Choice { option: Option<String> },
    /// A press opens a list to pick from, on its own page: a toy pattern (Off first) or a bHaptics
    /// effect (Buzz first). `current` is highlighted.
    Menu { options: Vec<String>, current: Option<usize> },
    /// Dragged along, 0–1: a slider or a toy.
    Level { value: f64 },
    /// A press applies it: a preset.
    Press,
    /// Fires after a hold: a shocker or the OWO Skin.
    Hold,
    /// Only in the app: a type the panel doesn't know.
    AppOnly
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tile {
    pub group_id: String,
    pub control_id: String,
    pub name: String,
    #[serde(flatten)]
    pub kind: TileKind,
    /// The active profile locks it, or it can't be used right now.
    pub locked: bool
}

/// An add-on that's on, as its card in the app shows it, with a level for what has one (the leash).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddonRow {
    pub name: String,
    /// Its card's dot.
    pub state: crate::addons::State,
    pub status: String,
    pub level: Option<f64>,
    /// Intiface Central is installed but not running: the row has a button that starts it.
    #[serde(default)]
    pub launch: bool
}

/// A toy control of the avatar you're wearing, with its level now.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToyRow {
    pub name: String,
    pub level: f64,
    /// Avatar touch is driving it right now.
    pub touched: bool
}

/// The app's primary and secondary colors, as the panel draws them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    pub primary: [u8; 3],
    pub secondary: [u8; 3]
}

impl Default for Theme {
    /// Violet and cyan, the app's own defaults.
    fn default() -> Self {
        Self { primary: [0xa7, 0x8b, 0xfa], secondary: [0x22, 0xd3, 0xee] }
    }
}

/// The Home page.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Home {
    /// Viewers on the share page now, and how many of them signed in.
    pub watching: usize,
    pub signed_in: usize,
    /// Viewer actions in the last minute.
    pub recent: usize,
    /// The active profile, None for no profile.
    pub profile: Option<String>,
    pub profiles: Vec<Profile>
}

/// The Pinned page.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pinned {
    /// The avatar you're wearing.
    pub avatar: Option<String>,
    /// The controls pinned on that avatar, or its first 4 when nothing is pinned yet. The panel
    /// shows them a page at a time.
    pub tiles: Vec<Tile>,
    pub nothing_pinned: bool
}

/// Everything the panel shows, grouped by the page that shows it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub paused: bool,
    pub panel: PanelSettings,
    pub hud: HudSettings,
    pub theme: Theme,
    pub home: Home,
    /// What viewers did, newest first, at most 20: Home's Last and Viewers' Activity.
    pub activity: Vec<Entry>,
    pub viewers: Vec<Viewer>,
    pub pinned: Pinned,
    pub addons: Vec<AddonRow>,
    pub toys: Vec<ToyRow>,
    /// Someone pulls your leash, for the HUD.
    pub leash: Option<Pull>
}

/// Who pulls your leash, how hard, and where it moves you.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pull {
    pub who: String,
    /// 0–1.
    pub stretch: f64,
    /// Where it walks you, -1–1: to your right, and forward.
    pub right: f64,
    pub forward: f64
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ToApp {
    /// First, with the token the app passed on the command line.
    Hello { token: String },
    Stop,
    /// Only after the hold on Resume.
    Resume,
    Profile { id: Option<String> },
    /// Shows or hides the HUD.
    Hud { on: bool },
    /// Starts Intiface Central.
    LaunchIntiface,
    /// A pinned tile pressed: toggled, the next option, a preset, a shock after its hold. `level`
    /// for a slider or toy dragged to it, `pick` for what was picked from a `TileKind::Menu`.
    #[serde(rename_all = "camelCase")]
    Control {
        group_id: String,
        control_id: String,
        level: Option<f64>,
        #[serde(default)]
        pick: Option<usize>
    }
}
