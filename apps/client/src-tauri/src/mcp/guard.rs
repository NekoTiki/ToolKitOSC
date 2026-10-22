//! What every tool call passes before it runs: its level allowed in Settings › Assistants, the
//! avatar's `TKOSC/Assistants` switch and Stop everything, and the rate limit. Kept free of Tauri
//! so it's tested on its own.
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::model::{McpSettings, Permission};

/// The levels in Settings › Assistants, plus the tools any level gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// `stop_everything`: an assistant may always stop, never resume.
    Always,
    Read,
    Play,
    /// Makes drafts.
    Edit,
    Shock
}

/// The level a tool belongs to. A tool missing here is refused, so a new one can't slip out at a
/// level nobody picked.
pub fn level_of(tool: &str) -> Option<Level> {
    Some(match tool {
        "stop_everything" => Level::Always,
        "get_status" | "get_avatar" | "list_presets" | "list_patterns" | "list_automations" | "list_boards" | "list_controls" | "get_control" | "list_profiles" | "list_viewer_groups" | "list_viewers" | "get_activity" => Level::Read,
        "run_control" | "send_chatbox" | "set_parameter" | "switch_profile" => Level::Play,
        "save_automation" | "test_automation" | "save_pattern" | "create_controls" | "update_control" | "save_profile" | "delete_draft" => Level::Edit,
        "fire_shocker" => Level::Shock,
        _ => return None
    })
}

/// What Settings › Assistants allows for this tool: its level, and its own switch for the two
/// Play tools that have one.
fn permission(settings: &McpSettings, tool: &str) -> Permission {
    let switched_off = (tool == "set_parameter" && !settings.raw_parameters) || (tool == "switch_profile" && !settings.switch_profiles);
    match level_of(tool) {
        None => Permission::Off,
        Some(_) if switched_off => Permission::Off,
        Some(Level::Always) => Permission::On,
        Some(Level::Read) => settings.levels.read,
        Some(Level::Play) => settings.levels.play,
        Some(Level::Edit) => settings.levels.edit,
        Some(Level::Shock) => settings.levels.shock
    }
}

/// Whether the assistant sees the tool at all.
pub fn listed(settings: &McpSettings, tool: &str) -> bool {
    permission(settings, tool) != Permission::Off
}

/// What else holds tools back right now.
#[derive(Debug, Default, Clone, Copy)]
pub struct Blocks {
    /// The avatar turned `TKOSC/Assistants` off.
    pub muted: bool,
    /// Stop everything is on.
    pub paused: bool
}

/// Whether a call runs right away (On) or waits for the host (Ask). Err is why it can't run, as
/// the assistant reads it.
pub fn decide(settings: &McpSettings, tool: &str, blocks: Blocks) -> Result<Permission, &'static str> {
    let acts = level_of(tool).is_some_and(|level| !matches!(level, Level::Always | Level::Read));
    match permission(settings, tool) {
        Permission::Off => Err("This tool is turned off in ToolKitOSC"),
        _ if acts && blocks.paused => Err("Stopped by host: Stop everything is on, and only the host can resume"),
        _ if acts && blocks.muted => Err("The host turned assistants off from their avatar"),
        permission => Ok(permission)
    }
}

/// Calls per session per second.
const RATE: usize = 10;

/// A sliding one-second window of a session's calls.
#[derive(Default)]
pub struct RateLimit(VecDeque<Instant>);

impl RateLimit {
    /// Counts the call, false when it's over the limit.
    pub fn allow(&mut self, now: Instant) -> bool {
        while self.0.front().is_some_and(|t| now.duration_since(*t) >= Duration::from_secs(1)) {
            self.0.pop_front();
        }
        if self.0.len() >= RATE {
            return false;
        }
        self.0.push_back(now);
        true
    }
}

/// `Authorization: Bearer <key>`, compared in constant time.
pub fn key_matches(header: Option<&str>, key: &str) -> bool {
    let Some(given) = header.and_then(|h| h.strip_prefix("Bearer ")) else {
        return false;
    };
    given.len() == key.len() && given.bytes().zip(key.bytes()).fold(0, |acc, (a, b)| acc | (a ^ b)) == 0
}

/// Parameters an assistant may never set directly: the app's own (`TKOSC/Assistants` among them,
/// so it can't turn itself back on).
pub fn parameter_reserved(name: &str) -> bool {
    name.starts_with("TKOSC/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::model::Levels;

    fn with(levels: Levels) -> McpSettings {
        McpSettings { levels, ..McpSettings::default() }
    }

    #[test]
    fn levels_decide_what_runs_and_what_is_listed() {
        let settings = with(Levels { read: Permission::Off, ..Levels::default() });
        assert!(!listed(&settings, "get_avatar"));
        assert!(decide(&settings, "get_avatar", Blocks::default()).is_err());
        assert!(listed(&settings, "stop_everything"), "Stop is there at any level");
        assert_eq!(decide(&settings, "stop_everything", Blocks::default()), Ok(Permission::On));

        let settings = with(Levels { read: Permission::Ask, ..Levels::default() });
        assert!(listed(&settings, "get_status"));
        assert_eq!(decide(&settings, "get_status", Blocks::default()), Ok(Permission::Ask));
        assert_eq!(decide(&McpSettings::default(), "get_status", Blocks::default()), Ok(Permission::On));
        assert_eq!(decide(&McpSettings::default(), "save_automation", Blocks::default()), Ok(Permission::Ask), "Edit starts on Ask");
    }

    #[test]
    fn unknown_tools_are_refused() {
        assert!(!listed(&McpSettings::default(), "resume_everything"));
        assert!(decide(&McpSettings::default(), "resume_everything", Blocks::default()).is_err());
    }

    #[test]
    fn raw_parameters_and_profiles_have_their_own_switch() {
        let mut settings = McpSettings::default();
        assert!(listed(&settings, "set_parameter"), "raw parameters start on");
        assert!(!listed(&settings, "switch_profile"), "switching profiles starts off");
        settings.raw_parameters = false;
        settings.switch_profiles = true;
        assert!(!listed(&settings, "set_parameter"));
        assert!(listed(&settings, "switch_profile"));
    }

    #[test]
    fn stop_and_the_avatar_switch_hold_back_play_but_not_read() {
        let settings = McpSettings::default();
        for blocks in [Blocks { paused: true, muted: false }, Blocks { paused: false, muted: true }] {
            assert!(decide(&settings, "run_control", blocks).is_err());
            assert!(decide(&settings, "send_chatbox", blocks).is_err());
            assert!(decide(&settings, "fire_shocker", blocks).is_err());
            assert_eq!(decide(&settings, "get_avatar", blocks), Ok(Permission::On));
            assert_eq!(decide(&settings, "stop_everything", blocks), Ok(Permission::On));
        }
    }

    #[test]
    fn rate_limit_is_ten_per_second() {
        let mut limit = RateLimit::default();
        let start = Instant::now();
        assert!((0..10).all(|_| limit.allow(start)));
        assert!(!limit.allow(start + Duration::from_millis(500)));
        assert!(limit.allow(start + Duration::from_secs(1)));
    }

    #[test]
    fn key_needs_the_exact_bearer() {
        assert!(key_matches(Some("Bearer abc"), "abc"));
        assert!(!key_matches(Some("Bearer abd"), "abc"));
        assert!(!key_matches(Some("Bearer ab"), "abc"));
        assert!(!key_matches(Some("abc"), "abc"));
        assert!(!key_matches(None, "abc"));
    }

    #[test]
    fn the_apps_own_parameters_are_reserved() {
        assert!(parameter_reserved("TKOSC/Assistants"));
        assert!(parameter_reserved("TKOSC/Panic"));
        assert!(!parameter_reserved("Ears"));
    }
}
