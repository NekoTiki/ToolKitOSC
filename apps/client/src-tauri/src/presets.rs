//! Per-avatar preset storage, persisted to a JSON file under the app's data dir - same location
//! and shape of module as `osc::cache`, but for durable user data (created/edited explicitly via
//! the Presets UI) rather than a disposable last-known-value cache, hence the two differences from
//! it: writes are atomic (tmp file + rename, so a crash mid-write can't corrupt a saved preset) and
//! unthrottled (a preset is only ever saved in response to an explicit user action, never a hot
//! OSC-receive loop).
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::types::OscArg;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetParameter {
    pub address: String,
    pub name: String,
    pub kind: String, // 'Bool' | 'Float' | 'Int'
    pub value: OscArg
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub avatar_id: String,
    pub avatar_name: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub parameters: Vec<PresetParameter>
}

/// One file per avatar (`presets-<avatarId>.json`) holds both this avatar's presets and its
/// custom parameter exclude list - the exclude list only ever matters alongside the presets it
/// filters, so there's no reason to split it into a second file/command pair.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PresetStore {
    #[serde(default)]
    pub excluded_addresses: Vec<String>,
    #[serde(default)]
    pub presets: Vec<Preset>
}

fn store_path(app: &AppHandle, avatar_id: &str) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(format!("presets-{avatar_id}.json")))
}

pub fn load(app: &AppHandle, avatar_id: &str) -> Result<PresetStore, String> {
    let path = store_path(app, avatar_id)?;

    let Ok(data) = std::fs::read_to_string(&path) else {
        return Ok(PresetStore::default());
    };

    if data.trim().is_empty() {
        return Ok(PresetStore::default());
    }

    serde_json::from_str(&data).map_err(|e| e.to_string())
}

pub fn save(app: &AppHandle, avatar_id: &str, store: &PresetStore) -> Result<(), String> {
    let path = store_path(app, avatar_id)?;
    let tmp_path = path.with_extension("json.tmp");

    let json = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;

    std::fs::write(&tmp_path, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())?;

    Ok(())
}
