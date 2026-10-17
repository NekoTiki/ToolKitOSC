//! bHaptics Player's local WebSocket API (the older one, `/v2/feedbacks`, which needs no app
//! registration). It isn't documented by bHaptics; the messages are the ones bHapticsLib (MIT,
//! github.com/HerpDerpinstine/bHapticsLib) sends and reads.
//!
//! We send `{ "Register": [], "Submit": [...] }` packets: a `frame` per position with one dot per
//! motor (intensity 0–100), or `turnOff` / `turnOffAll`. Player answers with what's connected and
//! which keys are playing.
use serde::Deserialize;
use serde_json::{json, Value};

use super::model::Position;

pub const PORT: u16 = 15881;
pub const APP_ID: &str = "com.nekotiki.toolkitosc";
pub const APP_NAME: &str = "ToolKitOSC";
/// How long each frame plays. Longer than the tick, so motors stay on between frames and stop by
/// themselves soon after we stop sending.
pub const FRAME_MS: u32 = 150;

pub fn url(port: u16) -> String {
    format!("ws://127.0.0.1:{port}/v2/feedbacks?app_id={APP_ID}&app_name={APP_NAME}")
}

/// One position's motors, 0–100 each.
#[derive(Debug, Clone, PartialEq)]
pub enum Submit {
    Frame { position: Position, intensities: Vec<u8> },
    TurnOff(Position),
    TurnOffAll
}

fn submit(submit: &Submit) -> Value {
    match submit {
        Submit::Frame { position, intensities } => json!({
            "type": "frame",
            "key": position.key(),
            "Frame": {
                "position": position.name(),
                "durationMillis": FRAME_MS,
                "dotPoints": intensities
                    .iter()
                    .enumerate()
                    .filter(|(_, intensity)| **intensity > 0)
                    .map(|(index, intensity)| json!({ "index": index, "intensity": intensity }))
                    .collect::<Vec<_>>(),
                "pathPoints": []
            }
        }),
        Submit::TurnOff(position) => json!({ "type": "turnOff", "key": position.key() }),
        Submit::TurnOffAll => json!({ "type": "turnOffAll" })
    }
}

/// One packet with every submit, as text.
pub fn packet(submits: &[Submit]) -> String {
    json!({ "Register": [], "Submit": submits.iter().map(submit).collect::<Vec<_>>() }).to_string()
}

/// What Player says about itself. It sends this on its own, and after what we send.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct Response {
    pub connected_device_count: u32,
    /// `Vest`, `Head`, `ForearmL`…: see `Position::device`.
    pub connected_positions: Vec<String>,
    /// Keys playing now, ours (`TKOSC_…`) and any other app's.
    pub active_keys: Vec<String>
}

pub fn decode(text: &str) -> Option<Response> {
    serde_json::from_str(text).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_sends_only_the_motors_that_play() {
        let text = packet(&[Submit::Frame { position: Position::VestFront, intensities: vec![0, 0, 0, 0, 0, 0, 70, 0] }]);
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            value,
            json!({
                "Register": [],
                "Submit": [{
                    "type": "frame",
                    "key": "TKOSC_VestFront",
                    "Frame": { "position": "VestFront", "durationMillis": 150, "dotPoints": [{ "index": 6, "intensity": 70 }], "pathPoints": [] }
                }]
            })
        );
    }

    #[test]
    fn turning_off() {
        let value: Value = serde_json::from_str(&packet(&[Submit::TurnOff(Position::Head), Submit::TurnOffAll])).unwrap();
        assert_eq!(value["Submit"], json!([{ "type": "turnOff", "key": "TKOSC_Head" }, { "type": "turnOffAll" }]));
    }

    #[test]
    fn reads_what_player_answers() {
        let response = decode(
            r#"{"ConnectedDeviceCount":2,"ConnectedPositions":["Vest","Head"],"ActiveKeys":["TKOSC_VestFront"],"RegisteredKeys":[],"Status":{"VestFront":[0,0,70]}}"#
        )
        .unwrap();
        assert_eq!(response.connected_device_count, 2);
        assert_eq!(response.connected_positions, ["Vest", "Head"]);
        assert_eq!(response.active_keys, ["TKOSC_VestFront"]);
        assert_eq!(decode("{}"), Some(Response::default()));
        assert_eq!(decode("not json"), None);
    }

    #[test]
    fn the_url_names_the_app() {
        assert_eq!(url(PORT), "ws://127.0.0.1:15881/v2/feedbacks?app_id=com.nekotiki.toolkitosc&app_name=ToolKitOSC");
    }
}
