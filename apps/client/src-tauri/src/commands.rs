//! Tauri commands — the `invoke()`-callable surface the frontend's TS bridge
//! (src/lib/tauri-bridge.ts) wraps, matching the shape of the old Electron `window.api`.
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::Ordering;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::automations::{self, model::Automation};
use crate::boards::{self, BoardView, BoardsSnapshot, OutputConfig, Target};
use crate::openshock;
use crate::osc::oscquery;
use crate::presets::{self, PresetStore};
use crate::server;
use crate::state::AppState;
use crate::types::OscCommand;
use crate::{steamvr, vrcx};

#[tauri::command]
pub fn send_osc_message(state: State<AppState>, msg: OscCommand) -> Result<(), String> {
    crate::osc::send(&state, &msg)
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

/// Whether anything is listening on Intiface's address. The webview's WebSocket reports "nothing
/// there" and "there, but closed on us" the same way (an error, then a close before it ever
/// opened), so the frontend asks this to tell "it's running but refused us" apart from the rest.
/// A closed port doesn't mean Intiface Central isn't running, though: it stops listening while a
/// client is connected (confirmed live) - see `intiface_central_running` for that.
#[tauri::command]
pub async fn intiface_port_open(host: String, port: u16) -> bool {
    let connect = tokio::net::TcpStream::connect((host.as_str(), port));
    matches!(tokio::time::timeout(std::time::Duration::from_secs(1), connect).await, Ok(Ok(_)))
}

/// Whether an Intiface Central process is running on this machine, so auto-launch never opens a
/// second copy - whether its server is busy with another client, stopped, or still starting up.
/// Matches the process name rather than the path found by `find_intiface_central`, so a copy
/// installed somewhere else still counts: `intiface_central.exe` on Windows, `intiface_central`
/// in the Flatpak, `Intiface Central` on macOS.
#[tauri::command]
pub fn intiface_central_running() -> bool {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());

    system.processes().values().any(|process| {
        let name = process.name().to_string_lossy().to_lowercase();
        name.starts_with("intiface_central") || name.starts_with("intiface central")
    })
}

fn steamvr_manifest_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("app.vrmanifest"))
}

/// Whether an optional integration (SteamVR, VRCX) can be offered, so Settings can tell "can't
/// work on this OS" (hide the toggle) apart from "not installed" (show it disabled, saying why).
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Availability {
    Available,
    NotInstalled,
    Unsupported,
}

impl Availability {
    fn new(supported: bool, installed: bool) -> Self {
        match (supported, installed) {
            (false, _) => Self::Unsupported,
            (true, false) => Self::NotInstalled,
            (true, true) => Self::Available,
        }
    }
}

#[tauri::command]
pub fn steamvr_available() -> Availability {
    Availability::new(steamvr::is_supported(), steamvr::find_openvr_api().is_some())
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
pub fn vrcx_available() -> Availability {
    Availability::new(vrcx::is_supported(), vrcx::is_vrcx_installed())
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

/// Fetches every current avatar parameter value from VRChat over OSCQuery and feeds each through
/// the normal message pipeline (so the frontend just sees ordinary `vrc-osc-message` events) -
/// see `osc::oscquery::force_pull_parameters` for why this is needed on top of the push
/// subscription. Returns the number of parameters actually pulled.
#[tauri::command]
pub async fn force_pull_parameters(app: AppHandle, missing_only: bool) -> Result<usize, String> {
    oscquery::force_pull_parameters(&app, missing_only).await
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

#[tauri::command]
pub fn boards_list(app: AppHandle) -> BoardsSnapshot {
    boards::snapshot(&app)
}

/// Pairs with a board found over mDNS. Resolves once someone pressed its BOOT button (progress
/// arrives as `board-pairing` events), or fails with a message to show.
#[tauri::command]
pub async fn board_add(app: AppHandle, id: String) -> Result<BoardView, String> {
    boards::pair(app, Target::Found(id)).await
}

/// Same, for a board mDNS can't see: `192.168.1.40`, `tkosc-a1b2c3.local`, optionally `:port`.
#[tauri::command]
pub async fn board_add_by_address(app: AppHandle, address: String) -> Result<BoardView, String> {
    boards::pair(app, Target::Address(address)).await
}

#[tauri::command]
pub fn board_pair_cancel(app: AppHandle) {
    boards::cancel_pairing(&app);
}

/// Unpairs the board if it's online, and forgets it.
#[tauri::command]
pub fn board_remove(app: AppHandle, id: String) {
    boards::remove(&app, &id);
}

/// Saves a board's outputs and sends them to it, now or when it next connects.
#[tauri::command]
pub fn board_save_outputs(app: AppHandle, id: String, outputs: Vec<OutputConfig>) -> Result<(), String> {
    boards::save_outputs(&app, &id, outputs)
}

/// Switches a saved output on for half a second.
#[tauri::command]
pub fn board_test_output(app: AppHandle, id: String, pin: u8) -> Result<(), String> {
    boards::test_output(&app, &id, pin)
}

/// Every output of every board off (Stop everything).
#[tauri::command]
pub fn boards_stop_all(app: AppHandle) {
    boards::stop_all(&app);
}

#[tauri::command]
pub fn automations_list(app: AppHandle) -> Vec<Automation> {
    automations::list(&app)
}

/// Creates or replaces an automation, or fails with a message to show.
#[tauri::command]
pub fn automation_save(app: AppHandle, automation: Automation) -> Result<(), String> {
    automations::save(&app, automation)
}

#[tauri::command]
pub fn automation_delete(app: AppHandle, id: String) {
    automations::delete(&app, &id);
}

#[tauri::command]
pub fn automation_set_enabled(app: AppHandle, id: String, enabled: bool) {
    automations::set_enabled(&app, &id, enabled);
}

/// Runs the automation's action once, without its trigger.
#[tauri::command]
pub fn automation_test(app: AppHandle, id: String) -> Result<(), String> {
    automations::test(&app, &id)
}

/// Mirrors Stop everything's paused state.
#[tauri::command]
pub fn automations_set_paused(app: AppHandle, paused: bool) {
    automations::set_paused(&app, paused);
}

/// Something happened in the app that automations can start from (a viewer used a control...).
#[tauri::command]
pub fn automation_event(app: AppHandle, event: automations::model::AppEvent) {
    automations::event(&app, &event);
}

/// The active profile and how many viewers are on, for "only if".
#[tauri::command]
pub fn automations_set_context(app: AppHandle, context: automations::model::Context) {
    automations::set_context(&app, context);
}

/// Connects to the share server's `/host` socket, or keeps the link that's already up to the same
/// server with the same token (a webview reload) and replays its state.
#[tauri::command]
pub fn server_connect(app: AppHandle, url: String, token: Option<String>, protocol_version: u32) {
    server::connect(&app, url, token, protocol_version);
}

#[tauri::command]
pub fn server_disconnect(app: AppHandle) {
    server::disconnect(&app);
}

/// One `{type, message}` envelope, as JSON text.
#[tauri::command]
pub fn server_send(app: AppHandle, text: String) {
    server::send(&app, text);
}

/// The avatar parameters the share page shows (see `server::set_forwarded`).
#[tauri::command]
pub fn server_set_forwarded_addresses(app: AppHandle, addresses: Vec<String>) {
    server::set_forwarded(&app, addresses);
}

#[tauri::command]
pub fn openshock_state(app: AppHandle) -> openshock::View {
    openshock::view(&app)
}

/// Once, from the webview's old localStorage values (see `openshock::migrate`).
#[tauri::command]
pub async fn openshock_migrate(app: AppHandle, token: Option<String>, enabled: Option<bool>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || openshock::migrate(&app, token, enabled))
        .await
        .map_err(|e| e.to_string())?
}

/// An empty token removes it.
#[tauri::command]
pub async fn openshock_set_token(app: AppHandle, token: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || openshock::set_token(&app, token))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn openshock_set_enabled(app: AppHandle, enabled: bool) {
    openshock::set_enabled(&app, enabled);
}

#[tauri::command]
pub fn openshock_recheck(app: AppHandle) {
    openshock::recheck(&app);
}

#[tauri::command]
pub async fn openshock_shockers(app: AppHandle) -> Result<Vec<openshock::Hub>, String> {
    openshock::shockers(&app).await
}

/// Rejects when OpenShock isn't available. Failures while sending arrive as `openshock-error`.
#[tauri::command]
pub fn openshock_fire(app: AppHandle, fire: openshock::Fire) -> Result<(), String> {
    openshock::fire(&app, fire)
}

#[tauri::command]
pub async fn openshock_stop_all(app: AppHandle) -> Result<(), String> {
    openshock::stop_all(&app).await
}
