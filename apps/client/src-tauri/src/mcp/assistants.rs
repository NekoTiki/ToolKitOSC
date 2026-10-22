//! The assistants using the server, by name. Kept here rather than per MCP session: clients on
//! protocol 2026-07-28 have no handshake and no session, so each of their requests gets a fresh
//! handler. An assistant counts as connected while it has called a tool lately; nothing says when
//! one leaves.
use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::Serialize;

use super::guard::RateLimit;

/// How long after its last call an assistant still counts as connected.
pub const CONNECTED_FOR: Duration = Duration::from_secs(10 * 60);

/// An assistant connected now, as Settings › Assistants and the Viewers page show it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Client {
    id: u64,
    /// From its MCP client info, e.g. "claude-code".
    pub name: String,
    /// Unix ms.
    since: u64,
    calls: u32,
    last_tool: Option<String>
}

struct Assistant {
    client: Client,
    seen: Instant,
    rate: RateLimit,
    /// Tools the host allowed for 10 minutes, until when.
    allowed: HashMap<String, Instant>
}

#[derive(Default)]
pub struct Assistants {
    list: Vec<Assistant>,
    next_id: u64
}

/// What a call did to the list.
pub struct Call {
    /// Under the rate limit.
    pub allowed: bool,
    /// The assistant wasn't connected before.
    pub joined: bool
}

impl Assistants {
    /// Counts a call from `name` to `tool`. `since` is the wall clock, for showing.
    pub fn call(&mut self, name: &str, tool: &str, now: Instant, since: u64) -> Call {
        let joined = !self.list.iter().any(|a| a.client.name == name);
        if joined {
            self.next_id += 1;
            let client = Client { id: self.next_id, name: name.to_owned(), since, calls: 0, last_tool: None };
            self.list.push(Assistant { client, seen: now, rate: RateLimit::default(), allowed: HashMap::new() });
        }
        let assistant = self.list.iter_mut().find(|a| a.client.name == name).expect("added above");
        assistant.seen = now;
        assistant.client.calls += 1;
        assistant.client.last_tool = Some(tool.to_owned());
        Call { allowed: assistant.rate.allow(now), joined }
    }

    /// The host picked "Allow for 10 min" for this tool from this assistant.
    pub fn allow(&mut self, name: &str, tool: &str, until: Instant) {
        if let Some(assistant) = self.list.iter_mut().find(|a| a.client.name == name) {
            assistant.allowed.insert(tool.to_owned(), until);
        }
    }

    pub fn is_allowed(&self, name: &str, tool: &str, now: Instant) -> bool {
        self.list.iter().find(|a| a.client.name == name).and_then(|a| a.allowed.get(tool)).is_some_and(|until| now < *until)
    }

    /// Drops the ones quiet for too long. True when any left.
    pub fn prune(&mut self, now: Instant) -> bool {
        let before = self.list.len();
        self.list.retain(|a| now.duration_since(a.seen) < CONNECTED_FOR);
        self.list.len() != before
    }

    pub fn clients(&self) -> Vec<Client> {
        self.list.iter().map(|a| a.client.clone()).collect()
    }

    pub fn clear(&mut self) {
        self.list.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calls_make_an_assistant_connected_until_it_goes_quiet() {
        let mut assistants = Assistants::default();
        let start = Instant::now();
        assert!(assistants.call("claude-code", "get_status", start, 0).joined);
        let again = assistants.call("claude-code", "list_controls", start + Duration::from_secs(1), 0);
        assert!(!again.joined && again.allowed);
        assert!(assistants.call("cursor", "get_avatar", start, 0).joined);

        let clients = assistants.clients();
        assert_eq!(clients.len(), 2);
        assert_eq!(clients[0].calls, 2);
        assert_eq!(clients[0].last_tool.as_deref(), Some("list_controls"));

        assert!(!assistants.prune(start + Duration::from_secs(60)));
        assert!(assistants.prune(start + CONNECTED_FOR + Duration::from_millis(500)), "cursor went quiet");
        assert_eq!(assistants.clients().len(), 1);
    }

    #[test]
    fn rate_limit_and_approvals_are_per_assistant() {
        let mut assistants = Assistants::default();
        let now = Instant::now();
        assert!((0..10).all(|_| assistants.call("claude-code", "run_control", now, 0).allowed));
        assert!(!assistants.call("claude-code", "run_control", now, 0).allowed);
        assert!(assistants.call("cursor", "run_control", now, 0).allowed, "its own limit");

        assistants.allow("claude-code", "save_profile", now + Duration::from_secs(600));
        assert!(assistants.is_allowed("claude-code", "save_profile", now));
        assert!(!assistants.is_allowed("cursor", "save_profile", now));
        assert!(!assistants.is_allowed("claude-code", "save_profile", now + Duration::from_secs(601)));
    }
}
