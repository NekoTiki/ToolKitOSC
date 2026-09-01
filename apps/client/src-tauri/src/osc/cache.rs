//! Per-avatar last-known-OSC-value cache, persisted to a JSON file under the app's data dir.
//! Mirrors the original `getAvatarArgsCache`/`saveAvatarArgsCache` (same on-disk shape: a JSON
//! array of `[address, OscMessage]` tuples — `Array.from(Map.entries())` on the JS side).
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};

use crate::types::OscMessage;

pub struct CacheThrottle {
    last_saved: Mutex<Option<Instant>>
}

impl Default for CacheThrottle {
    fn default() -> Self {
        Self { last_saved: Mutex::new(None) }
    }
}

fn cache_path(app: &AppHandle, avatar_id: &str) -> Option<PathBuf> {
    let dir = app.path().app_data_dir().ok()?;
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join(format!("avatar-args-cache-{avatar_id}.json")))
}

pub fn load(app: &AppHandle, avatar_id: &str) -> HashMap<String, OscMessage> {
    let Some(path) = cache_path(app, avatar_id) else {
        return HashMap::new();
    };

    let Ok(data) = std::fs::read_to_string(&path) else {
        return HashMap::new();
    };

    if data.trim().is_empty() {
        return HashMap::new();
    }

    serde_json::from_str::<Vec<(String, OscMessage)>>(&data)
        .map(|entries| entries.into_iter().collect())
        .unwrap_or_default()
}

/// Saves at most once every 30s, matching the original's `lastTimeSaved` throttle. The actual
/// file write happens on a blocking task so a slow disk never stalls the OSC receive loop (the
/// original Node implementation used a synchronous `fs.writeFileSync` here, which did stall it).
pub fn save_throttled(
    app: AppHandle,
    throttle: &CacheThrottle,
    avatar_id: String,
    cache: HashMap<String, OscMessage>
) {
    {
        let mut last = throttle.last_saved.lock().unwrap();
        if let Some(t) = *last {
            if t.elapsed() < Duration::from_secs(30) {
                return;
            }
        }
        *last = Some(Instant::now());
    }

    tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = cache_path(&app, &avatar_id) else { return };
        let entries: Vec<(&String, &OscMessage)> = cache.iter().collect();

        match serde_json::to_string(&entries) {
            Ok(json) => {
                if let Err(err) = std::fs::write(&path, json) {
                    eprintln!("Failed to save avatar args cache to {path:?}: {err}");
                } else {
                    println!("Saved avatar args cache to \"{}\"", path.display());
                }
            }
            Err(err) => eprintln!("Failed to serialize avatar args cache: {err}")
        }
    });
}
