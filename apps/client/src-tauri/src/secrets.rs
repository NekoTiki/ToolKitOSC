//! Tokens the webview must never see (the OpenShock API token, later the share server's JWT), kept
//! in the system's password store: Windows Credential Manager, or the Secret Service on Linux
//! (GNOME Keyring, KWallet). When no store takes them, e.g. a bare window manager with no keyring
//! running, or a Credential Manager that reads but refuses new entries, they go to `secrets.json`
//! in the app data folder instead, and Settings says so. Each launch tries the store again and
//! moves them over once it takes them.
//!
//! Every call can block on the store, so call these from sync commands or `spawn_blocking`, never
//! straight from an async task.
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager};

const SERVICE: &str = "ToolKitOSC";
const FILE: &str = "secrets.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Backend {
    Keyring,
    File
}

/// Probed once per launch, and switched to the file if the store starts refusing writes.
static BACKEND: Mutex<Option<Backend>> = Mutex::new(None);
/// Serializes reads and writes of the fallback file.
static FILE_LOCK: Mutex<()> = Mutex::new(());

/// Named `ToolKitOSC.<key>` (e.g. `ToolKitOSC.openshock-token`), so the app's entries sort
/// together in Credential Manager. The crate's own default would be `<key>.ToolKitOSC`.
fn entry(key: &str) -> keyring::Result<keyring::Entry> {
    keyring::Entry::new_with_target(&format!("{SERVICE}.{key}"), SERVICE, key)
}

/// Whether the store takes a new entry, not only whether it answers: Credential Manager can keep
/// reading while it refuses every write.
fn probe() -> Backend {
    let works = entry("probe").and_then(|entry| {
        entry.set_password("probe")?;
        let read = entry.get_password();
        let _ = entry.delete_credential();
        read.map(|_| ())
    });

    match works {
        Ok(()) => Backend::Keyring,
        Err(err) => {
            tracing::warn!("The system password store doesn't take tokens, keeping them in {FILE}: {err}");
            Backend::File
        }
    }
}

pub fn backend() -> Backend {
    *BACKEND.lock().unwrap().get_or_insert_with(probe)
}

fn fall_back_to_file() {
    *BACKEND.lock().unwrap() = Some(Backend::File);
}

/// At startup: finds the store, and moves tokens a previous launch had to put in the file into it.
pub fn init(app: &AppHandle) {
    if backend() != Backend::Keyring {
        return;
    }

    let _guard = FILE_LOCK.lock().unwrap();
    let mut file = read_file(app);
    if file.is_empty() {
        return;
    }

    file.retain(|key, value| match entry(key).and_then(|entry| entry.set_password(value)) {
        Ok(()) => false,
        Err(err) => {
            tracing::error!("Couldn't move {key} to the password store: {err}");
            true
        }
    });
    if let Err(err) = write_file(app, &file) {
        tracing::error!("Couldn't save {FILE}: {err}");
    }
    tracing::info!("Moved the saved tokens to the system password store");
}

fn file_get(app: &AppHandle, key: &str) -> Option<String> {
    let _guard = FILE_LOCK.lock().unwrap();
    read_file(app).remove(key)
}

fn file_set(app: &AppHandle, key: &str, value: Option<&str>) -> Result<(), String> {
    let _guard = FILE_LOCK.lock().unwrap();
    let mut file = read_file(app);
    let changed = match value {
        Some(value) => file.insert(key.to_owned(), value.to_owned()).as_deref() != Some(value),
        None => file.remove(key).is_some()
    };
    if changed {
        write_file(app, &file)?;
    }
    Ok(())
}

/// From the store, or from the file when it isn't there: the store may have started refusing
/// writes after launch, or a token is still waiting in the file to move over.
pub fn get(app: &AppHandle, key: &str) -> Option<String> {
    if backend() == Backend::Keyring {
        match entry(key).and_then(|entry| entry.get_password()) {
            Ok(value) => return Some(value),
            Err(keyring::Error::NoEntry) => {}
            Err(err) => tracing::error!("Couldn't read {key} from the password store: {err}")
        }
    }
    file_get(app, key)
}

/// `None` removes it, from both places. A store that refuses the write sends this token, and every
/// later one this launch, to the file.
pub fn set(app: &AppHandle, key: &str, value: Option<&str>) -> Result<(), String> {
    let Some(value) = value else {
        if let Ok(entry) = entry(key) {
            match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => {}
                Err(err) => tracing::warn!("Couldn't remove {key} from the password store: {err}")
            }
        }
        return file_set(app, key, None);
    };

    if backend() == Backend::Keyring {
        match entry(key).and_then(|entry| entry.set_password(value)) {
            // An older copy in the file would otherwise come back if the store loses this one.
            Ok(()) => return file_set(app, key, None),
            Err(err) => {
                tracing::warn!("The password store refused {key}, keeping it in {FILE}: {err}");
                fall_back_to_file();
            }
        }
    }
    file_set(app, key, Some(value))
}

/// For `ToolKitOSC --mcp`, which runs without the app: from the store, or from the file in the app
/// data folder `dir`. Read only.
pub fn get_in(dir: &std::path::Path, key: &str) -> Option<String> {
    if let Ok(value) = entry(key).and_then(|entry| entry.get_password()) {
        return Some(value);
    }
    let data = std::fs::read_to_string(dir.join(FILE)).ok()?;
    serde_json::from_str::<BTreeMap<String, String>>(&data).ok()?.remove(key)
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_data_dir().ok()?;
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join(FILE))
}

fn read_file(app: &AppHandle) -> BTreeMap<String, String> {
    path(app)
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

/// Atomic, like `boards/store.rs`. An empty map removes the file.
fn write_file(app: &AppHandle, file: &BTreeMap<String, String>) -> Result<(), String> {
    let path = path(app).ok_or("no app data folder")?;

    if file.is_empty() {
        match std::fs::remove_file(&path) {
            Err(err) if err.kind() != std::io::ErrorKind::NotFound => Err(err.to_string()),
            _ => Ok(())
        }
    } else {
        (|| -> Result<(), String> {
            let tmp_path = path.with_extension("json.tmp");
            let json = serde_json::to_string_pretty(file).map_err(|e| e.to_string())?;
            std::fs::write(&tmp_path, json).map_err(|e| e.to_string())?;
            std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())
        })()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Touches the real password store, so it only runs on request: `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn keyring_round_trip() {
        assert_eq!(probe(), Backend::Keyring);
        let entry = entry("test-round-trip").unwrap();
        entry.set_password("value").unwrap();
        assert_eq!(entry.get_password().unwrap(), "value");
        entry.delete_credential().unwrap();
        assert!(matches!(entry.get_password(), Err(keyring::Error::NoEntry)));
    }
}

