//! The messages exchanged with a board, one JSON object per WebSocket text frame, tagged by `t`.
//! The full contract (fields, rules, error and close codes) is the firmware repo's PROTOCOL.md;
//! this only covers what the app sends and reads so far.
use serde::{Deserialize, Serialize};

/// Must match the board's `proto` (mDNS TXT record and `hello`).
pub const PROTO: u32 = 1;
pub const DEFAULT_PORT: u16 = 8787;
pub const PATH: &str = "/tkosc";

/// WebSocket close codes the board uses (see PROTOCOL.md).
pub const CLOSE_UNPAIRED_ON_BOARD: u16 = 4000;
pub const CLOSE_BAD_PROTO: u16 = 4400;
pub const CLOSE_UNAUTHORIZED: u16 = 4401;

#[derive(Debug, Serialize)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum Outgoing<'a> {
    Pair,
    Hello { proto: u32, token: &'a str },
    Ping,
    Unpair,
    Config { outputs: Vec<ConfigOutput> },
    Set { pin: u8, on: bool, ms: u32 },
    Stop
}

/// What automations send to an output. Owned, so it can travel through a link's channel.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum OutputMessage {
    Set { pin: u8, on: bool, ms: u32 },
    #[serde(rename_all = "camelCase")]
    Pulse { pin: u8, on_ms: u32, off_ms: u32, count: u32 },
    Pwm { pin: u8, duty: f32, ms: u32 }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigOutput {
    pub pin: u8,
    pub active_low: bool,
    pub max_on_ms: u32
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputPin {
    pub pin: u8,
    pub pwm: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Welcome {
    pub id: String,
    pub fw: String,
    pub name: String,
    pub chip: String,
    pub board: String,
    pub outputs: Vec<OutputPin>
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum Incoming {
    Pairing {
        #[serde(rename = "timeoutMs")]
        timeout_ms: u32
    },
    Paired {
        token: String
    },
    Welcome(Welcome),
    Pong,
    /// A pin changed. Not shown anywhere yet.
    #[allow(dead_code)]
    Out {
        pin: u8,
        on: bool,
        duty: f32
    },
    Err {
        code: String,
        #[serde(default, rename = "ref")]
        reference: Option<String>,
        #[serde(default)]
        pin: Option<u8>
    },
    /// Anything newer than this app knows about.
    #[serde(other)]
    Unknown
}

pub fn encode(message: &Outgoing) -> String {
    // Every variant is plain data, so this can't fail.
    serde_json::to_string(message).unwrap_or_default()
}

pub fn decode(text: &str) -> Option<Incoming> {
    serde_json::from_str(text).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_outgoing() {
        assert_eq!(encode(&Outgoing::Pair), r#"{"t":"pair"}"#);
        assert_eq!(
            encode(&Outgoing::Hello { proto: 1, token: "abc" }),
            r#"{"t":"hello","proto":1,"token":"abc"}"#
        );
        assert_eq!(
            encode(&Outgoing::Config { outputs: vec![ConfigOutput { pin: 4, active_low: false, max_on_ms: 10000 }] }),
            r#"{"t":"config","outputs":[{"pin":4,"activeLow":false,"maxOnMs":10000}]}"#
        );
        assert_eq!(
            encode(&Outgoing::Set { pin: 4, on: true, ms: 2000 }),
            r#"{"t":"set","pin":4,"on":true,"ms":2000}"#
        );
        assert_eq!(encode(&Outgoing::Stop), r#"{"t":"stop"}"#);
        assert_eq!(
            serde_json::to_string(&OutputMessage::Pulse { pin: 4, on_ms: 200, off_ms: 300, count: 3 }).unwrap(),
            r#"{"t":"pulse","pin":4,"onMs":200,"offMs":300,"count":3}"#
        );
        assert_eq!(
            serde_json::to_string(&OutputMessage::Pwm { pin: 25, duty: 0.5, ms: 1500 }).unwrap(),
            r#"{"t":"pwm","pin":25,"duty":0.5,"ms":1500}"#
        );
    }

    #[test]
    fn decodes_welcome() {
        let text = r#"{"t":"welcome","proto":1,"id":"a1b2c3","fw":"0.1.0","name":"Desk board","chip":"ESP32-S3","board":"Generic","outputs":[{"pin":4,"pwm":true},{"pin":7,"pwm":true,"label":"D8"}]}"#;
        let Some(Incoming::Welcome(welcome)) = decode(text) else {
            panic!("not a welcome");
        };
        assert_eq!(welcome.id, "a1b2c3");
        assert_eq!(welcome.outputs.len(), 2);
        assert_eq!(welcome.outputs[0].label, None);
        assert_eq!(welcome.outputs[1].label.as_deref(), Some("D8"));
    }

    #[test]
    fn decodes_pairing_and_errors() {
        assert_eq!(decode(r#"{"t":"pairing","timeoutMs":30000}"#), Some(Incoming::Pairing { timeout_ms: 30000 }));
        assert_eq!(
            decode(r#"{"t":"paired","token":"k7Qx"}"#),
            Some(Incoming::Paired { token: "k7Qx".into() })
        );
        assert_eq!(
            decode(r#"{"t":"err","code":"max-on","pin":4}"#),
            Some(Incoming::Err { code: "max-on".into(), reference: None, pin: Some(4) })
        );
        assert_eq!(decode(r#"{"t":"pong"}"#), Some(Incoming::Pong));
    }

    #[test]
    fn tolerates_unknown_messages() {
        assert_eq!(decode(r#"{"t":"something-new","x":1}"#), Some(Incoming::Unknown));
        assert_eq!(decode("not json"), None);
    }
}
