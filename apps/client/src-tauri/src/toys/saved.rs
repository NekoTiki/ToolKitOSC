//! The patterns the host recorded on the Patterns page, in `patterns.json`, so they play while the
//! window is closed. Global, not per avatar: a pattern is about the toy. Copied once from the
//! webview's localStorage.
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;

const FILE: &str = "patterns.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedPattern {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Intensities 0–100, one per 50 ms. Playback loops them.
    pub samples: Vec<f64>,
    pub updated_at: u64,
    /// Set while it's an assistant's draft.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<crate::types::CreatedBy>
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct PatternsFile {
    #[serde(default)]
    patterns: Vec<SavedPattern>
}

#[derive(Default)]
pub struct Saved {
    /// `None` until read from disk.
    patterns: Mutex<Option<Vec<SavedPattern>>>
}

fn saved(app: &AppHandle) -> &Saved {
    &app.state::<AppState>().inner().saved_patterns
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_data_dir().ok()?;
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join(FILE))
}

fn read_file(app: &AppHandle) -> Option<Vec<SavedPattern>> {
    let data = std::fs::read_to_string(path(app)?).ok()?;
    match serde_json::from_str::<PatternsFile>(&data) {
        Ok(file) => Some(file.patterns),
        Err(err) => {
            tracing::error!("Couldn't read {FILE}: {err}");
            None
        }
    }
}

fn write_file(app: &AppHandle, patterns: &[SavedPattern]) {
    let result = (|| -> Result<(), String> {
        let path = path(app).ok_or("no app data folder")?;
        let tmp_path = path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(&PatternsFile { patterns: patterns.to_vec() }).map_err(|e| e.to_string())?;
        std::fs::write(&tmp_path, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())
    })();

    if let Err(err) = result {
        tracing::error!("Couldn't save {FILE}: {err}");
    }
}

/// Runs `change` on the list (read from disk the first time), then saves and announces it when
/// `change` says something changed.
fn with<T>(app: &AppHandle, change: impl FnOnce(&mut Vec<SavedPattern>) -> (T, bool)) -> T {
    let (result, list) = {
        let mut guard = saved(app).patterns.lock().unwrap();
        let list = guard.get_or_insert_with(|| read_file(app).unwrap_or_default());
        let (result, changed) = change(list);
        (result, changed.then(|| list.clone()))
    };
    if let Some(list) = list {
        write_file(app, &list);
        let _ = app.emit("saved-patterns", &list);
    }
    result
}

pub fn list(app: &AppHandle) -> Vec<SavedPattern> {
    with(app, |list| (list.clone(), false))
}

pub fn samples(app: &AppHandle, id: &str) -> Option<Vec<f64>> {
    with(app, |list| (list.iter().find(|pattern| pattern.id == id).map(|pattern| pattern.samples.clone()), false))
}

/// Adds it, or replaces the one with the same id.
pub fn save(app: &AppHandle, pattern: SavedPattern) {
    with(app, |list| {
        match list.iter_mut().find(|existing| existing.id == pattern.id) {
            Some(existing) => *existing = pattern,
            None => list.push(pattern)
        }
        ((), true)
    });
}

pub fn delete(app: &AppHandle, id: &str) {
    with(app, |list| {
        let before = list.len();
        list.retain(|pattern| pattern.id != id);
        ((), list.len() != before)
    });
}

/// The first launch after this moved out of the webview: takes over its recordings. A no-op once
/// `patterns.json` exists.
pub fn migrate(app: &AppHandle, patterns: Vec<SavedPattern>) {
    if path(app).is_some_and(|path| path.exists()) {
        return;
    }
    let patterns: Vec<_> = patterns.into_iter().filter(|pattern| !pattern.samples.is_empty()).collect();
    tracing::info!("Moved {} recorded pattern(s) out of the webview", patterns.len());
    with(app, |list| {
        *list = patterns;
        ((), true)
    });
}
