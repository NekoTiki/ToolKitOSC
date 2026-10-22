//! `ToolKitOSC --mcp`: for MCP clients that only start a program and talk to it over stdin and
//! stdout (Claude Desktop). It runs instead of the app, before Tauri and its single-instance check,
//! and passes each JSON-RPC line on to the running app's server with the key, which it reads itself
//! so it never sits in the client's config. It never starts the app: while the app or the add-on
//! is off, every request gets "ToolKitOSC isn't running".
use std::io::{BufRead, Write};
use std::path::PathBuf;

use serde_json::{json, Value};

/// Tauri's app data folder for this app: `identifier` in tauri.conf.json, under the OS data folder.
fn data_dir() -> Option<PathBuf> {
    Some(directories::BaseDirs::new()?.data_dir().join("io.github.nekotiki.toolkitosc"))
}

fn port(dir: Option<&PathBuf>) -> u16 {
    dir.and_then(|dir| std::fs::read_to_string(dir.join("mcp.json")).ok())
        .and_then(|data| serde_json::from_str::<super::model::McpSettings>(&data).ok())
        .map_or(super::model::DEFAULT_PORT, |settings| settings.port)
}

/// The JSON-RPC error a request gets when the app can't answer.
fn error_reply(message: &Value, text: &str) -> Option<Value> {
    let id = message.get("id")?;
    Some(json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32000, "message": text } }))
}

struct Link {
    client: reqwest::Client,
    url: String,
    key: Option<String>,
    session: Option<String>,
    /// Sent again when the app restarted and forgot the session.
    initialize: Option<Value>
}

enum Sent {
    Reply(Option<Value>),
    /// The app doesn't know this session: it restarted.
    SessionGone,
    Down(String)
}

impl Link {
    async fn post(&mut self, message: &Value) -> Sent {
        let mut request = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .bearer_auth(self.key.as_deref().unwrap_or_default())
            .body(message.to_string());
        if let Some(session) = &self.session {
            request = request.header("Mcp-Session-Id", session);
        }
        let response = match request.send().await {
            Ok(response) => response,
            Err(_) => return Sent::Down("ToolKitOSC isn't running, or AI assistants are off in it".to_owned())
        };
        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND && self.session.is_some() {
            return Sent::SessionGone;
        }
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Sent::Down("ToolKitOSC refused the key: it was renewed in Settings › Assistants".to_owned());
        }
        if let Some(session) = response.headers().get("mcp-session-id").and_then(|value| value.to_str().ok()) {
            self.session = Some(session.to_owned());
        }
        let text = response.text().await.unwrap_or_default();
        Sent::Reply(parse(&text))
    }

    /// Passes one message on, starting the session again if the app restarted in between.
    async fn forward(&mut self, message: &Value) -> Option<Value> {
        if message.get("method").and_then(Value::as_str) == Some("initialize") {
            self.session = None;
            self.initialize = Some(message.clone());
        }
        let mut sent = self.post(message).await;
        if matches!(sent, Sent::SessionGone) {
            self.session = None;
            if let Some(initialize) = self.initialize.clone() {
                let _ = self.post(&initialize).await;
                let _ = self.post(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })).await;
            }
            sent = self.post(message).await;
        }
        match sent {
            Sent::Reply(reply) => reply,
            Sent::SessionGone => error_reply(message, "ToolKitOSC restarted: reconnect the assistant"),
            Sent::Down(text) => error_reply(message, &text)
        }
    }
}

/// The reply in a response body: plain JSON, or the last `data:` event of an SSE stream.
fn parse(body: &str) -> Option<Value> {
    serde_json::from_str(body).ok().or_else(|| {
        body.lines()
            .filter_map(|line| line.strip_prefix("data:"))
            .filter_map(|data| serde_json::from_str::<Value>(data.trim()).ok())
            .filter(|event| event.get("id").is_some())
            .last()
    })
}

pub fn run() -> std::process::ExitCode {
    // reqwest's TLS wants it even for plain http (see lib.rs).
    let _ = rustls::crypto::ring::default_provider().install_default();
    let dir = data_dir();
    let key = dir.as_ref().and_then(|dir| crate::secrets::get_in(dir, super::KEY_SECRET));
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread().enable_all().build() else {
        return std::process::ExitCode::FAILURE;
    };
    let mut link = Link { client: reqwest::Client::new(), url: format!("http://127.0.0.1:{}/mcp", port(dir.as_ref())), key, session: None, initialize: None };

    let mut stdout = std::io::stdout().lock();
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let Ok(message) = serde_json::from_str::<Value>(&line) else { continue };
        if let Some(reply) = runtime.block_on(link.forward(&message)) {
            if writeln!(stdout, "{reply}").and_then(|()| stdout.flush()).is_err() {
                break;
            }
        }
    }
    std::process::ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replies_come_as_json_or_as_the_last_event() {
        assert_eq!(parse(r#"{"jsonrpc":"2.0","id":1,"result":{}}"#).unwrap()["id"], 1);
        let sse = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/progress\"}\n\nevent: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{}}\n\n";
        assert_eq!(parse(sse).unwrap()["id"], 2);
        assert!(parse("").is_none());
    }

    #[test]
    fn only_requests_get_an_error_reply() {
        assert!(error_reply(&json!({ "id": 3, "method": "tools/list" }), "down").is_some());
        assert!(error_reply(&json!({ "method": "notifications/initialized" }), "down").is_none());
    }
}
