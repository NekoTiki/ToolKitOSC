//! Tauri commands — the `invoke()`-callable surface the frontend's TS bridge
//! (src/lib/tauri-bridge.ts) wraps, matching the shape of the old Electron `window.api`.
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter, Manager, State};

use crate::osc::codec;
use crate::presets::{self, PresetStore};
use crate::state::AppState;
use crate::types::OscCommand;
use crate::{steamvr, vrcx};

#[tauri::command]
pub fn send_osc_message(state: State<AppState>, msg: OscCommand) -> Result<(), String> {
    let socket = state.sender_socket.lock().unwrap().clone();
    let Some(socket) = socket else {
        return Err("OSC sender not ready yet".into());
    };

    let avatar_details = state.avatar_details.lock().unwrap().clone();
    let packet = rosc::OscPacket::Message(codec::encode_command(&msg, avatar_details.as_ref()));
    let bytes = rosc::encoder::encode(&packet).map_err(|e| e.to_string())?;

    // OSCQuery discovery (osc::oscquery) may know VRChat's real address, which can differ from
    // the fixed send port 9000 (multiple VRChat instances, custom --osc launch args); fall back
    // to the socket's pre-connected fixed target when discovery hasn't found anything (yet, or
    // ever, on a network where mDNS doesn't work).
    let target = *state.oscquery_target.lock().unwrap();

    // send()/send_to() on a UDP socket is a plain non-blocking syscall in practice; spawn it so
    // this command handler never has to be async just to satisfy the socket API.
    tauri::async_runtime::spawn(async move {
        let result = match target {
            Some(addr) => socket.send_to(&bytes, addr).await.map(|_| ()),
            None => socket.send(&bytes).await.map(|_| ()),
        };
        if let Err(err) = result {
            tracing::warn!("Failed to send OSC message: {err}");
        }
    });

    Ok(())
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

/// Best-effort lookup for a local Intiface Central install, so Settings can offer to launch it
/// instead of just reporting a failed connection. Intiface Central doesn't register itself
/// anywhere queryable (no protocol handler, no registry key we can rely on), so this only checks
/// the well-known per-OS install locations from its own installer/packaging — a custom install
/// directory won't be found. One path (or bundle/app id) per OS, so unlike Windows/macOS this
/// doesn't need `find` — a single tail expression per `#[cfg]` block is the whole function body.
fn find_intiface_central() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // The installer runs per-user (unprivileged) by default, installing under %APPDATA%; an
        // admin/all-users install instead lands under %ProgramFiles%. Check both.
        [
            std::env::var("ProgramFiles").ok(),
            std::env::var("APPDATA").ok(),
        ]
        .into_iter()
        .flatten()
        .map(|dir| {
            PathBuf::from(dir)
                .join("IntifaceCentral")
                .join("intiface_central.exe")
        })
        .find(|path| path.is_file())
    }

    #[cfg(target_os = "macos")]
    {
        let path = PathBuf::from("/Applications/Intiface Central.app");
        path.is_dir().then_some(path)
    }

    #[cfg(target_os = "linux")]
    {
        // The Flathub package (com.nonpolynomial.intiface_central) is the officially distributed
        // build - other install methods (AUR, manual builds) aren't detected.
        let user_install = std::env::var("HOME").ok().map(|home| {
            PathBuf::from(home).join(".local/share/flatpak/app/com.nonpolynomial.intiface_central")
        });

        [
            user_install,
            Some(PathBuf::from(
                "/var/lib/flatpak/app/com.nonpolynomial.intiface_central",
            )),
        ]
        .into_iter()
        .flatten()
        .find(|path| path.is_dir())
    }
}

#[tauri::command]
pub fn intiface_central_available() -> bool {
    find_intiface_central().is_some()
}

#[tauri::command]
pub fn start_intiface_central() -> Result<(), String> {
    let path = find_intiface_central()
        .ok_or_else(|| "Intiface Central was not found on this system".to_string())?;

    #[cfg(target_os = "windows")]
    {
        Command::new(&path).spawn().map_err(|e| e.to_string())?;
    }

    // .app bundles are launched through `open`, not by executing a binary inside them directly -
    // it's the standard way to start a macOS application and avoids having to know/guess the
    // bundle's internal executable name.
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-a")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("flatpak")
            .args(["run", "com.nonpolynomial.intiface_central"])
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn steamvr_manifest_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("app.vrmanifest"))
}

#[tauri::command]
pub fn steamvr_available() -> bool {
    steamvr::find_openvr_api_dll().is_some()
}

#[tauri::command]
pub fn steamvr_set_auto_launch(app: AppHandle, enable: bool) -> Result<(), String> {
    steamvr::set_auto_launch(&steamvr_manifest_path(&app)?, enable)
}

#[tauri::command]
pub fn steamvr_get_auto_launch(app: AppHandle) -> Result<bool, String> {
    steamvr::get_auto_launch(&steamvr_manifest_path(&app)?)
}

#[tauri::command]
pub fn vrcx_available() -> bool {
    vrcx::is_vrcx_installed()
}

#[tauri::command]
pub fn vrcx_set_auto_launch(enable: bool) -> Result<(), String> {
    vrcx::set_auto_launch(enable)
}

#[tauri::command]
pub fn vrcx_get_auto_launch() -> bool {
    vrcx::get_auto_launch()
}

/// Keeps the window's native close handler and the tray icon (see `lib.rs`) in sync with the
/// frontend's localStorage-backed "keep running in the tray" setting - called once at startup and
/// on every change, since the setting itself lives in the webview, out of reach of either.
#[tauri::command]
pub fn set_minimize_to_tray(state: State<AppState>, enabled: bool) {
    state.minimize_to_tray.store(enabled, Ordering::Relaxed);

    if let Some(tray) = state.tray_icon.lock().unwrap().as_ref() {
        let _ = tray.set_visible(enabled);
    }
}

#[tauri::command]
pub fn load_presets(app: AppHandle, avatar_id: String) -> Result<PresetStore, String> {
    presets::load(&app, &avatar_id)
}

#[tauri::command]
pub fn save_presets(app: AppHandle, avatar_id: String, store: PresetStore) -> Result<(), String> {
    presets::save(&app, &avatar_id, &store)
}

/// Replays current state to a (re)connecting frontend — equivalent to the original's
/// `preload-ready` IPC handler.
#[tauri::command]
pub fn ready(app: AppHandle, state: State<AppState>) {
    if let Some(details) = state.avatar_details.lock().unwrap().clone() {
        let _ = app.emit("vrc-avatar-details", &details);
    }

    let messages: Vec<_> = state.messages.lock().unwrap().values().cloned().collect();
    for msg in messages {
        let _ = app.emit("vrc-osc-message", &msg);
    }
}
