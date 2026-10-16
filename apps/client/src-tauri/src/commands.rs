//! Tauri commands — the `invoke()`-callable surface the frontend's TS bridge
//! (src/lib/tauri-bridge.ts) wraps, matching the shape of the old Electron `window.api`.
use std::path::PathBuf;
use std::sync::atomic::Ordering;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::automations::{self, model::Automation};
use crate::boards::{self, BoardView, BoardsSnapshot, OutputConfig, Target};
use crate::openshock;
use crate::{intiface, toys};
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
pub fn server_connect(app: AppHandle, url: String, protocol_version: u32) {
    server::connect(&app, url, protocol_version);
}

/// A request to the server's HTTP API, signed with that server's token.
#[tauri::command]
pub async fn server_request(
    app: AppHandle,
    url: String,
    method: String,
    path: String,
    body: Option<serde_json::Value>
) -> Result<server::auth::Response, String> {
    server::auth::request(&app, &url, &method, &path, body).await
}

#[tauri::command]
pub fn auth_state(app: AppHandle) -> server::auth::View {
    server::auth::view(&app)
}

/// Once, from the tokens the webview used to keep in localStorage (server URL -> JWT).
#[tauri::command]
pub async fn auth_migrate(app: AppHandle, tokens: std::collections::BTreeMap<String, String>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || server::auth::migrate(&app, tokens))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn auth_sign_out(app: AppHandle, url: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || server::auth::set_token(&app, &url, None))
        .await
        .map_err(|e| e.to_string())?
}

/// Starts signing in to `url`. The browser link arrives as `auth-uri`, the result as `auth-changed`.
#[tauri::command]
pub fn auth_sign_in(app: AppHandle, url: String) {
    server::auth::sign_in(&app, url);
}

#[tauri::command]
pub fn auth_sign_in_cancel(app: AppHandle) {
    server::auth::sign_in_cancel(&app);
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

#[tauri::command]
pub fn intiface_state(app: AppHandle) -> intiface::View {
    intiface::view(&app)
}

/// Once, from the settings the webview used to keep in localStorage.
#[tauri::command]
pub fn intiface_migrate(app: AppHandle, url: Option<String>, enabled: Option<bool>, auto_launch: Option<bool>) {
    intiface::migrate(&app, url, enabled, auto_launch);
}

/// Empty or missing goes back to the default URL.
#[tauri::command]
pub fn intiface_set_url(app: AppHandle, url: Option<String>) {
    intiface::set_url(&app, url);
}

#[tauri::command]
pub fn intiface_set_enabled(app: AppHandle, enabled: bool) {
    intiface::set_enabled(&app, enabled);
}

#[tauri::command]
pub fn intiface_set_auto_launch(app: AppHandle, auto_launch: bool) {
    intiface::set_auto_launch(&app, auto_launch);
}

/// Opens the local Intiface Central install.
#[tauri::command]
pub fn intiface_launch() -> Result<(), String> {
    intiface::launch_now()
}

/// Sets these actuators to `level` (0–1).
#[tauri::command]
pub fn toys_set(app: AppHandle, actuators: Vec<toys::ActuatorRef>, level: f64) {
    toys::set(&app, &actuators, level);
}

/// Stops every connected toy.
#[tauri::command]
pub fn toys_stop_all(app: AppHandle) {
    toys::stop_all(&app);
}

/// Plays a pattern on these actuators for this control, after taking them from any other pattern.
/// Returns the run's number (`pattern-ended` carries it), or nothing for an unknown pattern.
#[tauri::command]
pub fn patterns_play(
    app: AppHandle,
    control_id: String,
    source: toys::patterns::Source,
    actuators: Vec<toys::ActuatorRef>,
    stop_after_ms: Option<u64>,
    by: Option<String>
) -> Option<u64> {
    toys::patterns::play(&app, &control_id, source, actuators, stop_after_ms, by)
}

/// Stops this control's pattern and zeroes its toys.
#[tauri::command]
pub fn patterns_off(app: AppHandle, control_id: String, actuators: Vec<toys::ActuatorRef>) {
    toys::patterns::off(&app, &control_id, &actuators);
}

/// Stops this control's pattern without touching its toys.
#[tauri::command]
pub fn patterns_stop(app: AppHandle, control_id: String) {
    toys::patterns::stop(&app, &control_id);
}

/// Stops every other pattern on these actuators, before something else drives them.
#[tauri::command]
pub fn patterns_release(app: AppHandle, actuators: Vec<toys::ActuatorRef>, except: Option<String>) {
    toys::patterns::release(&app, &actuators, except.as_deref());
}

#[tauri::command]
pub fn patterns_stop_all(app: AppHandle) {
    toys::patterns::stop_all(&app);
}

#[tauri::command]
pub fn patterns_saved_list(app: AppHandle) -> Vec<toys::saved::SavedPattern> {
    toys::saved::list(&app)
}

/// Adds a recorded pattern, or replaces the one with its id.
#[tauri::command]
pub fn patterns_save(app: AppHandle, pattern: toys::saved::SavedPattern) {
    toys::saved::save(&app, pattern);
}

#[tauri::command]
pub fn patterns_delete(app: AppHandle, id: String) {
    toys::saved::delete(&app, &id);
}

/// Once, from the recordings the webview used to keep in localStorage.
#[tauri::command]
pub fn patterns_migrate(app: AppHandle, patterns: Vec<toys::saved::SavedPattern>) {
    toys::saved::migrate(&app, patterns);
}
