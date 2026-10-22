//! `mcp.json` in the app data folder: the port and what assistants may do. Rust owns it so the
//! server keeps its rules while the window is closed.
use std::path::PathBuf;

use tauri::{AppHandle, Emitter, Manager};

use super::model::McpSettings;

const FILE: &str = "mcp.json";

fn path(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_data_dir().ok()?;
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join(FILE))
}

fn read_file(app: &AppHandle) -> McpSettings {
    let Some(data) = path(app).and_then(|path| std::fs::read_to_string(path).ok()) else {
        return McpSettings::default();
    };
    serde_json::from_str(&data).unwrap_or_else(|err| {
        tracing::error!("Couldn't read {FILE}: {err}");
        McpSettings::default()
    })
}

fn write_file(app: &AppHandle, settings: &McpSettings) {
    let result = (|| -> Result<(), String> {
        let path = path(app).ok_or("no app data folder")?;
        let tmp_path = path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
        std::fs::write(&tmp_path, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())
    })();

    if let Err(err) = result {
        tracing::error!("Couldn't save {FILE}: {err}");
    }
}

/// Runs `change` on the settings (read from disk the first time), then saves and announces them
/// when `change` says something changed.
pub fn with<T>(app: &AppHandle, change: impl FnOnce(&mut McpSettings) -> (T, bool)) -> T {
    let (result, changed) = {
        let mut guard = super::mcp(app).settings.lock().unwrap();
        let settings = guard.get_or_insert_with(|| read_file(app));
        let (result, changed) = change(settings);
        (result, changed.then(|| settings.clone()))
    };
    if let Some(settings) = changed {
        write_file(app, &settings);
        let _ = app.emit("mcp-settings", &settings);
    }
    result
}

pub fn get(app: &AppHandle) -> McpSettings {
    with(app, |settings| (settings.clone(), false))
}
