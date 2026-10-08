//! The added boards, persisted to `boards.json` under the app's data dir. Writes are atomic (a tmp
//! file, then a rename), like `presets.rs`: this holds the pairing tokens, and losing them means
//! pressing BOOT on every board again.
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use super::protocol::OutputPin;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardRecord {
    /// The board's id (last 3 bytes of its MAC), stable across IP changes and resets.
    pub id: String,
    pub name: String,
    /// Never sent to the frontend.
    pub token: String,
    /// Last address that worked, or the latest one mDNS announced.
    pub host: String,
    pub port: u16,
    /// From the board's last `welcome`.
    #[serde(default)]
    pub fw: Option<String>,
    #[serde(default)]
    pub chip: Option<String>,
    #[serde(default)]
    pub board: Option<String>,
    #[serde(default)]
    pub outputs: Vec<OutputPin>,
    /// The outputs set up in this app. None until they're first saved: until then nothing is sent,
    /// and the board keeps whatever it already has.
    #[serde(default)]
    pub config: Option<Vec<OutputConfig>>
}

/// One output ("Fan on GPIO 4"). The label only lives in the app; the board gets the rest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputConfig {
    pub pin: u8,
    pub label: String,
    pub active_low: bool,
    pub max_on_ms: u32
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct BoardsFile {
    #[serde(default)]
    boards: Vec<BoardRecord>
}

fn path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("boards.json"))
}

pub fn load(app: &AppHandle) -> Vec<BoardRecord> {
    let Ok(path) = path(app) else {
        return Vec::new();
    };
    let Ok(data) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };

    match serde_json::from_str::<BoardsFile>(&data) {
        Ok(file) => file.boards,
        Err(err) => {
            tracing::error!("Couldn't read {}: {err}", path.display());
            Vec::new()
        }
    }
}

pub fn save(app: &AppHandle, boards: &[BoardRecord]) {
    let result = (|| -> Result<(), String> {
        let path = path(app)?;
        let tmp_path = path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(&BoardsFile { boards: boards.to_vec() }).map_err(|e| e.to_string())?;

        std::fs::write(&tmp_path, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())
    })();

    if let Err(err) = result {
        tracing::error!("Couldn't save boards.json: {err}");
    }
}
