//! What `mcp.json` holds: the port, and what assistants may do.
use serde::{Deserialize, Serialize};

/// The port Claude Code, Cursor and the others are set up with. Fixed so their configs stay valid
/// across restarts; changeable in Settings › Assistants if something else uses it.
pub const DEFAULT_PORT: u16 = 47631;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Permission {
    /// Its tools aren't even listed to the assistant.
    Off,
    /// Each call waits for you to allow it.
    Ask,
    On
}

/// What each group of tools may do.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Levels {
    /// Avatar, controls, Activity.
    pub read: Permission,
    /// Controls, presets, toys, chatbox.
    pub play: Permission,
    /// Drafts of controls, profiles, automations and patterns.
    pub edit: Permission,
    /// OpenShock controls.
    pub shock: Permission
}

impl Default for Levels {
    fn default() -> Self {
        Self { read: Permission::On, play: Permission::On, edit: Permission::Ask, shock: Permission::On }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct McpSettings {
    pub port: u16,
    pub levels: Levels,
    /// `set_parameter` writes avatar parameters directly.
    pub raw_parameters: bool,
    /// `switch_profile` changes the active profile, which decides what everyone may do.
    pub switch_profiles: bool
}

impl Default for McpSettings {
    fn default() -> Self {
        Self { port: DEFAULT_PORT, levels: Levels::default(), raw_parameters: true, switch_profiles: false }
    }
}
