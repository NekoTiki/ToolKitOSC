//! Toasts in the headset, through XSOverlay or OVR Toolkit, whichever runs. At most one every
//! `GAP`: what arrives in between is merged into one ("Sam and 3 others turned on Ears"), except
//! shocks and Stop, which always go out on their own.
//!
//! Both formats come from working senders (VRCIM's `vrNotificationService.ts`, Vencord's XSOverlay
//! plugin), not from official docs.
use std::time::Duration;

use futures_util::SinkExt;
use serde_json::json;
use tauri::AppHandle;
use tokio::sync::mpsc;

use super::model::Target;

const GAP: Duration = Duration::from_secs(2);
const XSOVERLAY: &str = "127.0.0.1:42069";
const OVR_TOOLKIT: &str = "ws://127.0.0.1:11450/api";
/// How long a merged toast's text gets before "+N more".
const MAX_CONTENT: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Shock,
    Toy,
    Control,
    Viewer,
    Stop,
    Automation
}

impl Kind {
    /// Never merged with anything.
    fn alone(self) -> bool {
        matches!(self, Kind::Shock | Kind::Stop)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    pub kind: Kind,
    /// Who did it, when someone did.
    pub who: Option<String>,
    /// What happened, after the name: "turned on Ears".
    pub what: String,
    pub detail: String
}

impl Toast {
    fn title(&self) -> String {
        match &self.who {
            Some(who) => format!("{who} {}", self.what),
            None => capitalize(&self.what)
        }
    }
}

pub fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

/// The next toast to send from what's waiting, removed from `queue`: the first one if it goes out
/// alone, otherwise every waiting toast that can be merged, as one.
fn take_next(queue: &mut Vec<Toast>) -> Option<(String, String)> {
    let first = queue.first()?;
    if first.kind.alone() {
        let toast = queue.remove(0);
        return Some((toast.title(), toast.detail));
    }

    let merged: Vec<Toast> = {
        let (merged, rest): (Vec<Toast>, Vec<Toast>) = queue.drain(..).partition(|toast| !toast.kind.alone());
        *queue = rest;
        merged
    };
    if let [one] = merged.as_slice() {
        return Some((one.title(), one.detail.clone()));
    }
    let first = &merged[0];
    let others = merged.len() - 1;
    // The same thing by several people reads as one sentence.
    if merged.iter().all(|toast| toast.what == first.what) {
        let who = first.who.clone().unwrap_or_else(|| "Someone".into());
        let people = if others == 1 { "1 other".to_string() } else { format!("{others} others") };
        let detail = merged.last().map(|toast| toast.detail.clone()).unwrap_or_default();
        return Some((format!("{who} and {people} {}", first.what), detail));
    }
    let mut content = String::new();
    for (index, toast) in merged.iter().enumerate() {
        let line = toast.title();
        if !content.is_empty() && content.len() + line.len() + 3 > MAX_CONTENT {
            content.push_str(&format!(" · +{} more", merged.len() - index));
            break;
        }
        if !content.is_empty() {
            content.push_str(" · ");
        }
        content.push_str(&line);
    }
    Some((format!("{} things happened", merged.len()), content))
}

/// Sends what's queued, one toast every `GAP`, until the add-on stops.
pub async fn run(app: AppHandle, mut rx: mpsc::UnboundedReceiver<Toast>) {
    let mut queue: Vec<Toast> = Vec::new();
    let mut last: Option<tokio::time::Instant> = None;
    loop {
        if queue.is_empty() {
            match rx.recv().await {
                Some(toast) => queue.push(toast),
                None => return
            }
        }
        let ready = last.map_or_else(tokio::time::Instant::now, |last| last + GAP);
        tokio::select! {
            toast = rx.recv() => match toast {
                Some(toast) => queue.push(toast),
                None => return
            },
            _ = tokio::time::sleep_until(ready) => {
                if let Some((title, content)) = take_next(&mut queue) {
                    if let Err(err) = send(&app, &title, &content).await {
                        tracing::debug!("Toast not sent: {err}");
                    }
                    last = Some(tokio::time::Instant::now());
                }
            }
        }
    }
}

/// Sends one toast where the settings say, now. Fails when there's nowhere to send it.
pub async fn send(app: &AppHandle, title: &str, content: &str) -> Result<(), String> {
    match super::destination(app).ok_or("Neither XSOverlay nor OVR Toolkit is running")? {
        Target::Ovrtoolkit => send_ovr_toolkit(title, content).await,
        _ => send_xsoverlay(title, content).await
    }
}

async fn send_xsoverlay(title: &str, content: &str) -> Result<(), String> {
    let message = json!({
        "messageType": 1,
        "index": 0,
        "timeout": 4.0,
        "height": if content.is_empty() { 90.0 } else { 110.0 },
        "opacity": 1.0,
        "volume": 0.5,
        "audioPath": "",
        "title": title,
        "content": content,
        "useBase64Icon": false,
        "icon": "",
        "sourceApp": "ToolKitOSC"
    });
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.map_err(|e| e.to_string())?;
    socket.send_to(message.to_string().as_bytes(), XSOVERLAY).await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn send_ovr_toolkit(title: &str, content: &str) -> Result<(), String> {
    // `json` is a string holding JSON, not an object.
    let message = json!({
        "messageType": "SendNotification",
        "json": json!({ "title": title, "body": content }).to_string()
    });
    let (mut socket, _) = tokio::time::timeout(Duration::from_secs(2), tokio_tungstenite::connect_async(OVR_TOOLKIT))
        .await
        .map_err(|_| "OVR Toolkit didn't answer".to_string())?
        .map_err(|e| e.to_string())?;
    socket.send(message.to_string().into()).await.map_err(|e| e.to_string())?;
    let _ = socket.close(None).await;
    Ok(())
}

/// Which of the two runs, XSOverlay first, from the running processes' names in lower case.
pub fn detect(names: &[String]) -> Option<Target> {
    if names.iter().any(|name| name.starts_with("xsoverlay")) {
        Some(Target::Xsoverlay)
    } else if names.iter().any(|name| name.starts_with("ovr toolkit")) {
        Some(Target::Ovrtoolkit)
    } else {
        None
    }
}

/// What a viewer's command did, in words: "turned on", "set Face to Blush". A short take on
/// `describeCommand` (packages/shared-ui/src/utils/commandValue.ts).
pub fn describe(control: Option<&serde_json::Value>, kind: &str, name: &str, value: Option<&serde_json::Value>) -> (String, String) {
    let number = value.and_then(serde_json::Value::as_f64);
    let label = |list: &str, key: &str| -> Option<String> {
        let index = number? as usize;
        control?.get(list)?.get(index)?.get(key)?.as_str().map(str::to_owned)
    };
    let set = |to: String| (format!("set {name} to {to}"), String::new());
    match kind {
        "boolean" | "boolean-group" => {
            let on = value.is_some_and(|value| value.as_bool().unwrap_or(value.as_f64().is_some_and(|n| n > 0.0)));
            (format!("turned {} {name}", if on { "on" } else { "off" }), String::new())
        }
        "boolean-enum" => label("inputs", "name").map_or_else(|| (format!("used {name}"), String::new()), set),
        "enum" | "step-enum" => label("options", "name").map_or_else(|| (format!("used {name}"), String::new()), set),
        "slider" | "intiface-toy" => number.map_or_else(|| (format!("used {name}"), String::new()), |n| set(format!("{} %", (n * 100.0).round()))),
        "intiface-pattern" => {
            let id = value.and_then(serde_json::Value::as_str);
            let pattern = control
                .and_then(|control| control.get("allowedPatterns")?.as_array())
                .and_then(|patterns| patterns.iter().find(|p| p.get("id").and_then(serde_json::Value::as_str) == id))
                .and_then(|pattern| pattern.get("name")?.as_str());
            match pattern {
                Some(pattern) => set(pattern.to_owned()),
                None => (format!("played a pattern on {name}"), String::new())
            }
        }
        "open-shock-shocker" => {
            let vibrate = control.and_then(|c| c.get("mode")).and_then(serde_json::Value::as_str) == Some("Vibrate");
            let intensity = value.and_then(|v| v.get("intensity")).and_then(serde_json::Value::as_f64);
            let duration = value.and_then(|v| v.get("duration")).and_then(serde_json::Value::as_f64);
            let detail = match (intensity, duration) {
                (Some(i), Some(d)) => format!("{} % · {:.1} s", i.round(), d / 1000.0),
                _ => String::new()
            };
            (format!("fired {name}{}", if vibrate { " (vibrate)" } else { "" }), detail)
        }
        "owo" => (format!("played {} on {name}", value.and_then(serde_json::Value::as_str).unwrap_or("a sensation")), String::new()),
        "preset" => (format!("applied {name}"), String::new()),
        _ => (format!("used {name}"), String::new())
    }
}

/// The kind of toast a control type makes.
pub fn kind_of(control_type: &str) -> Kind {
    match control_type {
        "open-shock-shocker" | "owo" => Kind::Shock,
        "intiface-toy" | "intiface-pattern" => Kind::Toy,
        _ => Kind::Control
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toast(kind: Kind, who: &str, what: &str) -> Toast {
        Toast { kind, who: Some(who.into()), what: what.into(), detail: String::new() }
    }

    #[test]
    fn the_same_action_by_several_people_is_one_sentence() {
        let mut queue = vec![toast(Kind::Control, "Sam", "turned on Ears"), toast(Kind::Control, "Mika", "turned on Ears"), toast(Kind::Control, "Alex", "turned on Ears")];
        assert_eq!(take_next(&mut queue), Some(("Sam and 2 others turned on Ears".into(), String::new())));
        assert!(queue.is_empty());
    }

    #[test]
    fn shocks_and_stop_go_out_alone() {
        let mut queue = vec![
            toast(Kind::Shock, "Sam", "fired Shocker"),
            toast(Kind::Toy, "Mika", "set Lush to 80 %"),
            toast(Kind::Shock, "Alex", "fired Shocker"),
            toast(Kind::Viewer, "Kit", "joined")
        ];
        assert_eq!(take_next(&mut queue).unwrap().0, "Sam fired Shocker");
        // The toy and the join merge; Alex's shock waits for its own turn.
        assert_eq!(take_next(&mut queue).unwrap(), ("2 things happened".into(), "Mika set Lush to 80 % · Kit joined".into()));
        assert_eq!(take_next(&mut queue).unwrap().0, "Alex fired Shocker");
        assert_eq!(take_next(&mut queue), None);
    }

    #[test]
    fn a_long_merge_ends_with_more() {
        let mut queue: Vec<Toast> = (0..20).map(|i| toast(Kind::Control, &format!("Viewer{i}"), &format!("turned on Control{i}"))).collect();
        let (title, content) = take_next(&mut queue).unwrap();
        assert_eq!(title, "20 things happened");
        assert!(content.len() < MAX_CONTENT + 20 && content.ends_with("more"), "{content}");
    }

    #[test]
    fn describes_commands() {
        let control = json!({ "options": [{ "name": "Smile" }, { "name": "Blush" }], "mode": "Shock" });
        assert_eq!(describe(Some(&control), "enum", "Face", Some(&json!(1))).0, "set Face to Blush");
        assert_eq!(describe(None, "boolean", "Ears", Some(&json!(true))).0, "turned on Ears");
        assert_eq!(describe(None, "intiface-toy", "Lush", Some(&json!(0.8))).0, "set Lush to 80 %");
        assert_eq!(
            describe(Some(&control), "open-shock-shocker", "Shocker", Some(&json!({ "intensity": 40, "duration": 1000 }))),
            ("fired Shocker".into(), "40 % · 1.0 s".into())
        );
    }
}
