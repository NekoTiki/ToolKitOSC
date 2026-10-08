//! Every automation, persisted to `automations.json` under the app's data dir. Atomic writes like
//! `presets.rs` and `boards/store.rs`.
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use super::model::Automation;

#[derive(Debug, Default, Serialize, Deserialize)]
struct AutomationsFile {
    #[serde(default)]
    automations: Vec<Automation>
}

fn path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("automations.json"))
}

pub fn load(app: &AppHandle) -> Vec<Automation> {
    let Ok(path) = path(app) else {
        return Vec::new();
    };
    let Ok(data) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };

    match serde_json::from_str::<AutomationsFile>(&data) {
        Ok(file) => file.automations,
        Err(err) => {
            tracing::error!("Couldn't read {}: {err}", path.display());
            Vec::new()
        }
    }
}

pub fn save(app: &AppHandle, automations: &[Automation]) {
    let result = (|| -> Result<(), String> {
        let path = path(app)?;
        let tmp_path = path.with_extension("json.tmp");
        let file = AutomationsFile { automations: automations.to_vec() };
        let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;

        std::fs::write(&tmp_path, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())
    })();

    if let Err(err) = result {
        tracing::error!("Couldn't save automations.json: {err}");
    }
}
