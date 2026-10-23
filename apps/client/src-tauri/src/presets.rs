//! Per-avatar preset storage, persisted to a JSON file under the app's data dir - same location
//! and shape of module as `osc::cache`, but for durable user data (created/edited explicitly via
//! the Presets UI) rather than a disposable last-known-value cache, hence the two differences from
//! it: writes are atomic (tmp file + rename, so a crash mid-write can't corrupt a saved preset) and
//! unthrottled (a preset is only ever saved in response to an explicit user action, never a hot
//! OSC-receive loop).
use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;
use crate::types::{OscArg, OscMessage};

/// The id of the session's "Original state" preset (see `keep_original_state`).
pub const ORIGINAL_STATE_ID: &str = "original";

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
    if !crate::setup::valid_avatar_id(avatar_id) {
        return Err(format!("Not an avatar id: {avatar_id}"));
    }
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

#[tauri::command]
pub fn load_presets(app: AppHandle, avatar_id: String) -> Result<PresetStore, String> {
    load(&app, &avatar_id)
}

#[tauri::command]
pub fn save_presets(app: AppHandle, avatar_id: String, store: PresetStore) -> Result<(), String> {
    save(&app, &avatar_id, &store)?;
    // A preset control is unavailable without its preset.
    crate::setup::refresh(&app);
    Ok(())
}

/// Keeps the avatar's values from its first load this session as an unsaved preset, "Original
/// state", so you can go back to how it looked. Called once VRChat's real values are in
/// (`osc::oscquery::pull_on_avatar_load`); a later load of the same avatar keeps the first one.
pub fn keep_original_state(app: &AppHandle, avatar_id: &str, values: &HashMap<String, OscMessage>) {
    let state = app.state::<AppState>();
    if state.original_states.lock().unwrap().contains_key(avatar_id) {
        return;
    }
    let Some(details) = state.avatar_details.lock().unwrap().clone().filter(|d| d.id == avatar_id) else {
        return;
    };

    // Only what can be sent back: the parameters the avatar takes as input.
    let parameters: Vec<PresetParameter> = details
        .parameters
        .iter()
        .filter_map(|param| {
            let input = param.input.as_ref()?;
            let value = values.get(&input.address)?.args.first()?.clone();
            Some(PresetParameter { address: input.address.clone(), name: param.name.clone(), kind: input.kind.clone(), value })
        })
        .collect();
    let now = chrono::Utc::now().to_rfc3339();
    let preset = Preset {
        id: ORIGINAL_STATE_ID.to_owned(),
        avatar_id: avatar_id.to_owned(),
        avatar_name: details.name,
        name: "Original state".to_owned(),
        icon: Some("i-lucide-history".to_owned()),
        created_at: now.clone(),
        updated_at: now,
        parameters
    };

    state.original_states.lock().unwrap().insert(avatar_id.to_owned(), preset.clone());
    let _ = app.emit("original-state", &preset);
}

#[tauri::command]
pub fn original_state(app: AppHandle, avatar_id: String) -> Option<Preset> {
    app.state::<AppState>().original_states.lock().unwrap().get(&avatar_id).cloned()
}
