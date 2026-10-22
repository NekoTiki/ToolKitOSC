//! What every tool call passes before it runs: its level allowed in Settings › Assistants, and the
//! rate limit. Kept free of Tauri so it's tested on its own.
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::model::{Levels, Permission};

/// The levels in Settings › Assistants, plus the tools any level gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// `stop_everything`: an assistant may always stop, never resume.
    Always,
    Read
}

/// The level a tool belongs to. A tool missing here is refused, so a new one can't slip out at a
/// level nobody picked.
pub fn level_of(tool: &str) -> Option<Level> {
    Some(match tool {
        "stop_everything" => Level::Always,
        "get_status" | "get_avatar" | "list_presets" | "list_patterns" | "list_automations" | "list_boards" | "list_controls" | "get_control" | "list_profiles" | "list_viewer_groups" | "list_viewers" | "get_activity" => Level::Read,
        _ => return None
    })
}

pub fn permission(levels: &Levels, level: Level) -> Permission {
    match level {
        Level::Always => Permission::On,
        Level::Read => levels.read
    }
}

/// Whether the assistant sees the tool at all.
pub fn listed(levels: &Levels, tool: &str) -> bool {
    level_of(tool).is_some_and(|level| permission(levels, level) != Permission::Off)
}

/// Whether a call runs right away (On) or waits for the host (Ask). Err is why it can't run, as
/// the assistant reads it.
pub fn decide(levels: &Levels, tool: &str) -> Result<Permission, &'static str> {
    match level_of(tool).map(|level| permission(levels, level)) {
        None | Some(Permission::Off) => Err("This tool is turned off in ToolKitOSC"),
        Some(permission) => Ok(permission)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_decide_what_runs_and_what_is_listed() {
        let levels = Levels { read: Permission::Off, ..Levels::default() };
        assert!(!listed(&levels, "get_avatar"));
        assert!(decide(&levels, "get_avatar").is_err());
        assert!(listed(&levels, "stop_everything"), "Stop is there at any level");
        assert_eq!(decide(&levels, "stop_everything"), Ok(Permission::On));

        let levels = Levels { read: Permission::Ask, ..Levels::default() };
        assert!(listed(&levels, "get_status"));
        assert_eq!(decide(&levels, "get_status"), Ok(Permission::Ask));
        assert_eq!(decide(&Levels::default(), "get_status"), Ok(Permission::On));
    }

    #[test]
    fn unknown_tools_are_refused() {
        assert!(!listed(&Levels::default(), "resume_everything"));
        assert!(decide(&Levels::default(), "resume_everything").is_err());
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
}
