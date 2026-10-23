//! Stop everything: every toy, pattern, shocker and board output stops, and no viewer, assistant or
//! automation can act until you resume. Kept in `stop.json` so a crash or a restart never quietly
//! hands control back. Pressed from the app, its hotkey, `TKOSC/Panic`, the wrist panel, an
//! automation or an assistant; only the app and the panel resume. The webview follows
//! `paused-changed`.
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::automations::model::AppEvent;

const FILE: &str = "stop.json";

/// Turning this Bool on (e.g. from the expression menu) stops everything. Turning it off does
/// nothing, so a slip in the menu can't give control back.
pub const PANIC_ADDRESS: &str = "/avatar/parameters/TKOSC/Panic";

#[derive(Debug, Default, Serialize, Deserialize)]
struct StopFile {
    #[serde(default)]
    paused: bool
}

fn path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(FILE))
}

fn read_file(app: &AppHandle) -> Option<StopFile> {
    let data = std::fs::read_to_string(path(app).ok()?).ok()?;
    serde_json::from_str(&data).ok()
}

fn write_file(app: &AppHandle, paused: bool) -> Result<(), String> {
    let path = path(app)?;
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, serde_json::to_string(&StopFile { paused }).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())
}

/// At startup: paused stays paused.
pub fn load(app: &AppHandle) {
    if read_file(app).is_some_and(|file| file.paused) {
        crate::automations::set_paused(app, true);
    }
}

/// The first launch after this moved out of the webview: a pause it kept stays on. A no-op once
/// `stop.json` exists.
pub fn migrate(app: &AppHandle, paused: bool) {
    if path(app).map_or(true, |path| path.exists()) {
        return;
    }
    if paused {
        set_paused(app, true);
    } else if let Err(err) = write_file(app, false) {
        tracing::error!("Couldn't save Stop everything: {err}");
    }
}

pub fn is_paused(app: &AppHandle) -> bool {
    crate::automations::is_paused(app)
}

fn set_paused(app: &AppHandle, paused: bool) {
    if is_paused(app) == paused {
        return;
    }
    if let Err(err) = write_file(app, paused) {
        tracing::error!("Couldn't save Stop everything: {err}");
    }
    crate::automations::set_paused(app, paused);
    let _ = app.emit("paused-changed", paused);
    send_to_server(app);
    if !paused {
        crate::automations::event(app, &AppEvent::Resumed);
    }
}

/// Tells the share page, which says so to viewers. Dropped while signed out.
pub fn send_to_server(app: &AppHandle) {
    if crate::server::authed(app) {
        crate::server::send_message(app, "pause-state", serde_json::json!({ "paused": is_paused(app) }));
    }
}

/// Stops every add-on that's on and pauses. Returns the add-ons that couldn't be reached. Works
/// while already paused too: it stops anything you turned on yourself since.
pub async fn stop(app: &AppHandle) -> Vec<crate::addons::StopFailure> {
    set_paused(app, true);
    crate::addons::stop_everything(app).await
}

/// `stop` without waiting for the add-ons, for callers that can't.
pub fn stop_now(app: &AppHandle) {
    set_paused(app, true);
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        crate::addons::stop_everything(&handle).await;
    });
}

pub fn resume(app: &AppHandle) {
    set_paused(app, false);
}

/// A live `TKOSC/Panic` from VRChat. Not the values read when an avatar loads: a parameter saved as
/// on must not pause you every time you switch into that avatar.
pub fn on_parameter(app: &AppHandle, message: &crate::types::OscMessage) {
    use crate::types::OscArg;
    if message.address != PANIC_ADDRESS {
        return;
    }
    if matches!(message.args.first(), Some(OscArg::Bool(true))) || matches!(message.args.first(), Some(OscArg::Number(n)) if *n == 1.0) {
        tracing::info!("TKOSC/Panic: stopping everything");
        stop_now(app);
    }
}
