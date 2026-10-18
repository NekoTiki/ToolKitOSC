//! OpenShock, run from Rust so a shock waiting for its reel animation doesn't depend on the window,
//! and Stop everything can cancel it from here. The token lives in the system password store (see
//! `secrets`) and never reaches the webview. It's an add-on (see `addons`): the registry turns it
//! on and off.
//!
//! It fails closed: nothing goes out unless the last token check passed, the switch is on and
//! Stop everything isn't on. A shock still waiting for its animation checks again before it goes.
//!
//! The webview still rolls intensity and duration and checks the cooldown, because the tile
//! animation and the activity log need them straight away. Rust waits, checks and sends.
mod hub;
mod rest;
pub mod commands;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;

use crate::secrets;
use crate::state::AppState;
use hub::HubCommand;

pub const API: &str = "https://api.openshock.app";
/// OpenShock refuses requests without one.
pub const USER_AGENT: &str = concat!("ToolKitOSC/", env!("CARGO_PKG_VERSION"));
const TOKEN_KEY: &str = "openshock-token";
const FILE: &str = "openshock.json";
/// How long a Stop everything `Stop` lasts.
const STOP_DURATION_MS: u16 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Unconfigured,
    Checking,
    Valid,
    Invalid
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shocker {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub is_paused: bool
}

/// A shocker hub (the device), with the shockers paired to it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hub {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub shockers: Vec<Shocker>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ControlType {
    Stop,
    Shock,
    Vibrate,
    Sound
}

#[derive(Debug, Clone)]
pub struct Control {
    pub id: String,
    pub kind: ControlType,
    pub intensity: u8,
    pub duration: u16
}

impl Control {
    /// The REST API takes the type by name.
    fn rest_json(&self) -> Value {
        let kind = match self.kind {
            ControlType::Stop => "Stop",
            ControlType::Shock => "Shock",
            ControlType::Vibrate => "Vibrate",
            ControlType::Sound => "Sound"
        };
        json!({ "id": self.id, "type": kind, "intensity": self.intensity, "duration": self.duration })
    }

    /// The hub's JSON has no enum names, only numbers.
    fn hub_json(&self) -> Value {
        let kind = match self.kind {
            ControlType::Stop => 0,
            ControlType::Shock => 1,
            ControlType::Vibrate => 2,
            ControlType::Sound => 3
        };
        json!({ "id": self.id, "type": kind, "intensity": self.intensity, "duration": self.duration, "exclusive": false })
    }
}

/// One shock or vibration from a control, already rolled by the webview.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fire {
    pub shockers: Vec<String>,
    pub mode: ControlType,
    pub intensity: u8,
    pub duration: u16,
    /// The reel animation to wait for first.
    pub delay_ms: u64,
    /// Who it's for, shown in the OpenShock log after "ToolKitOSC".
    pub viewer_name: Option<String>
}

/// What the webview shows. The token itself never leaves Rust.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    enabled: bool,
    has_token: bool,
    status: Status,
    hubs: Vec<Hub>,
    /// Whether the live hub is connected. Only then is `online` known.
    live: bool,
    /// Shocker hubs that are online, while `live`.
    online: Vec<String>,
    /// Where the token is saved.
    secret_store: secrets::Backend
}

/// `openshock.json`. Now only a marker that the localStorage hand-off happened: the switch moved to
/// `addons.json`, and `enabled` is only read from files written before that.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Settings {
    #[serde(default, skip_serializing)]
    enabled: Option<bool>
}

struct Inner {
    loaded: bool,
    enabled: bool,
    token: Option<String>,
    status: Status,
    hubs: Vec<Hub>,
    /// Guards against an older check finishing after a newer one.
    check_id: u64,
    hub: Option<(u64, mpsc::UnboundedSender<HubCommand>)>,
    next_hub_id: u64,
    live: bool,
    online: HashSet<String>,
    pending: Vec<JoinHandle<()>>,
    /// When each control may shock again, in Unix ms.
    cooldowns: HashMap<String, u64>
}

pub struct OpenShock {
    inner: Mutex<Inner>,
    client: reqwest::Client
}

impl Default for OpenShock {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Inner {
                loaded: false,
                enabled: false,
                token: None,
                status: Status::Unconfigured,
                hubs: Vec::new(),
                check_id: 0,
                hub: None,
                next_hub_id: 0,
                live: false,
                online: HashSet::new(),
                pending: Vec::new(),
                cooldowns: HashMap::new()
            }),
            client: reqwest::Client::builder()
                .user_agent(USER_AGENT)
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default()
        }
    }
}

fn openshock(app: &AppHandle) -> &OpenShock {
    &app.state::<AppState>().inner().openshock
}

fn settings_path(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_data_dir().ok()?;
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join(FILE))
}

fn load_settings(app: &AppHandle) -> Option<Settings> {
    let data = std::fs::read_to_string(settings_path(app)?).ok()?;
    serde_json::from_str(&data).ok()
}

fn save_settings(app: &AppHandle, settings: &Settings) {
    let result = (|| -> Result<(), String> {
        let path = settings_path(app).ok_or("no app data folder")?;
        let tmp_path = path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
        std::fs::write(&tmp_path, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())
    })();

    if let Err(err) = result {
        tracing::error!("Couldn't save {FILE}: {err}");
    }
}

/// The add-on was turned on (or is on at launch): reads the token, then checks it. The password
/// store can be slow to answer, so this runs off the main thread.
fn start(app: &AppHandle) {
    openshock(app).inner.lock().unwrap().enabled = true;
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        load(&app);
        recheck(&app);
    });
}

/// The add-on was turned off: shocks still waiting are dropped and the live hub closes.
fn stop(app: &AppHandle) {
    {
        let mut inner = openshock(app).inner.lock().unwrap();
        inner.enabled = false;
        for task in inner.pending.drain(..) {
            task.abort();
        }
    }
    recheck(app);
}

/// Blocking: may read the password store.
fn load(app: &AppHandle) {
    if openshock(app).inner.lock().unwrap().loaded {
        return;
    }
    let token = secrets::get(app, TOKEN_KEY);

    let mut inner = openshock(app).inner.lock().unwrap();
    if !inner.loaded {
        inner.loaded = true;
        inner.token = token;
    }
}

/// The first launch after this moved out of the webview: takes over the token and switch it kept
/// in localStorage, and keeps the add-on on if it was. A no-op once `openshock.json` exists.
/// Blocking.
pub fn migrate(app: &AppHandle, token: Option<String>, enabled: Option<bool>) -> Result<(), String> {
    load(app);
    if load_settings(app).is_some() {
        return Ok(());
    }

    let token = token.filter(|token| !token.is_empty());
    if let Some(token) = &token {
        if secrets::get(app, TOKEN_KEY).is_none() {
            secrets::set(app, TOKEN_KEY, Some(token))?;
            openshock(app).inner.lock().unwrap().token = Some(token.clone());
        }
    }

    save_settings(app, &Settings::default());
    tracing::info!("Moved the OpenShock settings out of the webview");
    if enabled.unwrap_or(true) && token.is_some() {
        crate::addons::adopt(app, "openshock");
    }
    recheck(app);
    Ok(())
}

pub fn view(app: &AppHandle) -> View {
    let inner = openshock(app).inner.lock().unwrap();
    let mut online: Vec<String> = inner.online.iter().cloned().collect();
    online.sort();
    View {
        enabled: inner.enabled,
        has_token: inner.token.is_some(),
        status: inner.status,
        hubs: inner.hubs.clone(),
        live: inner.live,
        online,
        secret_store: secrets::backend()
    }
}

fn emit_state(app: &AppHandle) {
    let _ = app.emit("openshock-state", view(app));
    crate::addons::changed(app);
}

fn emit_error(app: &AppHandle, message: String) {
    tracing::warn!("OpenShock: {message}");
    let _ = app.emit("openshock-error", message);
}

/// Blocking: writes the password store. An empty token removes it.
pub fn set_token(app: &AppHandle, token: String) -> Result<(), String> {
    load(app);
    let token = Some(token.trim().to_owned()).filter(|token| !token.is_empty());
    secrets::set(app, TOKEN_KEY, token.as_deref())?;

    let old_hub = {
        let mut inner = openshock(app).inner.lock().unwrap();
        inner.token = token;
        inner.hubs.clear();
        inner.hub.take()
    };
    if let Some((_, tx)) = old_hub {
        let _ = tx.send(HubCommand::Shutdown);
    }
    recheck(app);
    Ok(())
}

/// Checks the token by fetching the shocker list, then starts or stops the live hub to match.
pub fn recheck(app: &AppHandle) {
    let (id, token) = {
        let mut inner = openshock(app).inner.lock().unwrap();
        inner.check_id += 1;

        let token = inner.token.clone().filter(|_| inner.enabled);
        if token.is_none() {
            inner.status = Status::Unconfigured;
        } else {
            inner.status = Status::Checking;
        }
        (inner.check_id, token)
    };

    let Some(token) = token else {
        stop_hub(app);
        emit_state(app);
        return;
    };
    emit_state(app);

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = rest::shockers(&openshock(&app).client, &token).await;

        let start_hub = {
            let mut inner = openshock(&app).inner.lock().unwrap();
            if inner.check_id != id {
                return;
            }
            match result {
                Ok(hubs) => {
                    inner.status = Status::Valid;
                    inner.hubs = hubs;
                    inner.hub.is_none()
                }
                Err(err) => {
                    tracing::warn!("OpenShock token check failed: {err}");
                    inner.status = Status::Invalid;
                    false
                }
            }
        };

        if start_hub {
            let mut inner = openshock(&app).inner.lock().unwrap();
            inner.next_hub_id += 1;
            let hub_id = inner.next_hub_id;
            inner.hub = Some((hub_id, hub::spawn(app.clone(), hub_id, token)));
        } else if openshock(&app).inner.lock().unwrap().status == Status::Invalid {
            stop_hub(&app);
        }
        emit_state(&app);
    });
}

fn stop_hub(app: &AppHandle) {
    let hub = {
        let mut inner = openshock(app).inner.lock().unwrap();
        inner.live = false;
        inner.online.clear();
        inner.hub.take()
    };
    if let Some((_, tx)) = hub {
        let _ = tx.send(HubCommand::Shutdown);
    }
}

/// The shocker list again, e.g. after the hub said something changed. Keeps the status.
pub fn refresh_hubs(app: &AppHandle) {
    let Some(token) = openshock(app).inner.lock().unwrap().token.clone() else {
        return;
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        match rest::shockers(&openshock(&app).client, &token).await {
            Ok(hubs) => {
                openshock(&app).inner.lock().unwrap().hubs = hubs;
                emit_state(&app);
            }
            Err(err) => tracing::warn!("Couldn't refresh the OpenShock shockers: {err}")
        }
    });
}

/// Fetches the shocker list for the control editor, and keeps it.
pub async fn shockers(app: &AppHandle) -> Result<Vec<Hub>, String> {
    let token = openshock(app).inner.lock().unwrap().token.clone().ok_or("No OpenShock token saved")?;
    let hubs = rest::shockers(&openshock(app).client, &token).await?;
    openshock(app).inner.lock().unwrap().hubs = hubs.clone();
    emit_state(app);
    Ok(hubs)
}

/// From live hub task `id`, ignored once it's been replaced.
fn hub_connected(app: &AppHandle, id: u64, connected: bool) {
    {
        let mut inner = openshock(app).inner.lock().unwrap();
        if inner.hub.as_ref().map(|(hub_id, _)| *hub_id) != Some(id) || inner.live == connected {
            return;
        }
        inner.live = connected;
        if !connected {
            inner.online.clear();
        }
    }
    emit_state(app);
}

/// Hubs that came online or went offline. `reset` forgets the ones known so far.
fn hubs_online(app: &AppHandle, id: u64, changes: Vec<(String, bool)>, reset: bool) {
    {
        let mut inner = openshock(app).inner.lock().unwrap();
        if inner.hub.as_ref().map(|(hub_id, _)| *hub_id) != Some(id) {
            return;
        }
        if reset {
            inner.online.clear();
        }
        for (device, online) in changes {
            if online {
                inner.online.insert(device);
            } else {
                inner.online.remove(&device);
            }
        }
    }
    emit_state(app);
}

fn available(app: &AppHandle) -> bool {
    let inner = openshock(app).inner.lock().unwrap();
    inner.enabled && inner.status == Status::Valid && inner.token.is_some() && !crate::automations::is_paused(app)
}

/// Sends a rolled shock once its reel animation is over. Refused straight away when OpenShock
/// isn't available, and dropped if that changes while it waits.
pub fn fire(app: &AppHandle, fire: Fire) -> Result<(), String> {
    if !available(app) {
        return Err("OpenShock isn't available".into());
    }

    let controls: Vec<Control> = fire
        .shockers
        .iter()
        .map(|id| Control { id: id.clone(), kind: fire.mode, intensity: fire.intensity, duration: fire.duration })
        .collect();
    let name = Some(match &fire.viewer_name {
        Some(viewer) => format!("ToolKitOSC · {viewer}"),
        None => "ToolKitOSC".to_owned()
    });

    let task_app = app.clone();
    let task = tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(fire.delay_ms)).await;
        if available(&task_app) {
            send(&task_app, controls, name);
        }
    });

    let mut inner = openshock(app).inner.lock().unwrap();
    inner.pending.retain(|task| !task.inner().is_finished());
    inner.pending.push(task);
    Ok(())
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or_default()
}

/// `floor(random × (max − min) + min)`, as the webview always rolled: `max` itself never comes up.
fn roll(range: Option<&Value>) -> f64 {
    let min = range.and_then(|r| r.get("min")).and_then(Value::as_f64).unwrap_or(0.0);
    let max = range.and_then(|r| r.get("max")).and_then(Value::as_f64).unwrap_or(min);
    (rand::random::<f64>() * (max - min) + min).floor()
}

/// Runs a Shocker control: checks its cooldown, rolls the intensity and duration, shows the roll on
/// its tile (the `shock-value` event, which the share page gets too), and sends the shock once the
/// reel animation is over. Returns what was rolled, with the shockers' names, or `None` when
/// OpenShock isn't available or the control is cooling down.
pub fn fire_control(app: &AppHandle, control: &Value, viewer_name: Option<String>) -> Option<Value> {
    if !available(app) {
        return None;
    }
    let control_id = control.get("id")?.as_str()?.to_owned();
    let now = now_ms();
    if openshock(app).inner.lock().unwrap().cooldowns.get(&control_id).is_some_and(|end| *end > now) {
        return None;
    }

    let mode = serde_json::from_value::<ControlType>(control.get("mode")?.clone()).ok()?;
    let shockers: Vec<String> = serde_json::from_value(control.get("shockers")?.clone()).ok()?;
    let intensity = roll(control.get("intensity"));
    let duration = roll(control.get("duration"));
    let animation = control.get("animationDuration").and_then(Value::as_f64).unwrap_or(0.0).max(0.0) as u64;
    let cooldown = control.get("cooldown").and_then(Value::as_f64).unwrap_or(0.0).max(0.0) as u64;
    let cooldown_end = now + cooldown + animation;

    let names: Vec<String> = {
        let mut inner = openshock(app).inner.lock().unwrap();
        inner.cooldowns.insert(control_id.clone(), cooldown_end);
        // Names now, not ids: a shocker can be renamed or removed by the time the log is read.
        shockers
            .iter()
            .filter_map(|id| inner.hubs.iter().flat_map(|hub| &hub.shockers).find(|s| &s.id == id).map(|s| s.name.clone()))
            .collect()
    };

    let _ = app.emit(
        "shock-value",
        json!({
            "controlId": control_id,
            "value": { "intensity": intensity, "duration": duration, "cooldownEnd": cooldown_end, "animationDuration": animation }
        })
    );

    let fired = fire(
        app,
        Fire {
            shockers,
            mode,
            intensity: intensity.clamp(0.0, 100.0) as u8,
            duration: duration.clamp(0.0, 65535.0) as u16,
            delay_ms: animation,
            viewer_name
        }
    );
    if let Err(err) = fired {
        tracing::warn!("Shock refused: {err}");
    }

    Some(json!({ "intensity": intensity, "duration": duration, "shockers": names }))
}

/// Over the live hub when it's connected, by REST otherwise.
fn send(app: &AppHandle, controls: Vec<Control>, name: Option<String>) {
    let hub = {
        let inner = openshock(app).inner.lock().unwrap();
        inner.hub.as_ref().filter(|_| inner.live).map(|(_, tx)| tx.clone())
    };

    match hub {
        Some(tx) => {
            if let Err(mpsc::error::SendError(HubCommand::Control(controls, name))) = tx.send(HubCommand::Control(controls, name)) {
                send_by_rest(app, controls, name);
            }
        }
        None => send_by_rest(app, controls, name)
    }
}

fn send_by_rest(app: &AppHandle, controls: Vec<Control>, _name: Option<String>) {
    let Some(token) = openshock(app).inner.lock().unwrap().token.clone() else {
        return;
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(err) = rest::control(&openshock(&app).client, &token, &controls).await {
            emit_error(&app, format!("Couldn't send to OpenShock: {err}"));
        }
    });
}

/// Stop everything: drops every shock still waiting, then sends `Stop` to every shocker on the
/// account, by REST so the result is known. The `Stop`s only go out while the token checks out,
/// whatever the switch says.
pub async fn stop_all(app: &AppHandle) -> Result<(), String> {
    let (token, controls) = {
        let mut inner = openshock(app).inner.lock().unwrap();
        for task in inner.pending.drain(..) {
            task.abort();
        }
        let controls: Vec<Control> = inner
            .hubs
            .iter()
            .flat_map(|hub| &hub.shockers)
            .map(|shocker| Control { id: shocker.id.clone(), kind: ControlType::Stop, intensity: 0, duration: STOP_DURATION_MS })
            .collect();
        (inner.token.clone().filter(|_| inner.status == Status::Valid), controls)
    };

    let (Some(token), false) = (token, controls.is_empty()) else {
        return Ok(());
    };
    rest::control(&openshock(app).client, &token, &controls).await
}

/// OpenShock as an add-on (see `crate::addons`).
pub struct Addon;

impl crate::addons::Addon for Addon {
    fn manifest(&self) -> crate::addons::Manifest {
        crate::addons::Manifest {
            id: "openshock",
            name: "OpenShock",
            group: "toys",
            pitch: "Shockers your viewers can fire, with the limits you set",
            platforms: &["windows", "linux"],
            badges: &["shocks"],
            needs: &[],
            needs_any: &[],
            control_types: &["open-shock-shocker"],
            automation_kinds: &[],
            pages: &[]
        }
    }

    fn start(&self, app: &AppHandle) {
        start(app);
    }

    fn stop(&self, app: &AppHandle) {
        stop(app);
    }

    fn status(&self, app: &AppHandle) -> crate::addons::Status {
        use crate::addons::{State, Status as S};
        let inner = openshock(app).inner.lock().unwrap();
        if inner.token.is_none() {
            return S::new(if inner.loaded { State::Warn } else { State::Busy }, if inner.loaded { "No API key yet" } else { "Starting…" });
        }
        let shockers: usize = inner.hubs.iter().map(|hub| hub.shockers.len()).sum();
        let count = format!("{shockers} shocker{}", if shockers == 1 { "" } else { "s" });
        match inner.status {
            Status::Unconfigured | Status::Checking => S::new(State::Busy, "Checking your API key…"),
            Status::Invalid => S::new(State::Error, "OpenShock refused your API key"),
            Status::Valid if inner.live && !inner.hubs.is_empty() && inner.online.is_empty() => S::new(State::Warn, "Hub offline"),
            Status::Valid if inner.live => S::new(State::Ok, format!("Connected · {count}")),
            Status::Valid => S::new(State::Ok, format!("Ready · {count}"))
        }
    }

    fn stop_everything(&self, app: &AppHandle) -> crate::addons::StopFuture {
        let app = app.clone();
        Box::pin(async move { stop_all(&app).await })
    }

    fn was_in_use(&self, app: &AppHandle) -> bool {
        load_settings(app).map_or(true, |settings| settings.enabled.unwrap_or(true)) && secrets::get(app, TOKEN_KEY).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_json_names_the_type() {
        let control = Control { id: "s1".into(), kind: ControlType::Shock, intensity: 10, duration: 500 };
        let value = control.rest_json();
        assert_eq!(value["type"], "Shock");
        assert_eq!(value["duration"], 500);
    }

    #[test]
    fn fire_parses_from_the_webview() {
        let fire: Fire = serde_json::from_value(json!({
            "shockers": ["a", "b"], "mode": "Vibrate", "intensity": 30, "duration": 1200, "delayMs": 3000, "viewerName": null
        }))
        .unwrap();
        assert_eq!(fire.mode, ControlType::Vibrate);
        assert_eq!(fire.delay_ms, 3000);
        assert!(fire.viewer_name.is_none());
    }

    #[test]
    fn hubs_parse_from_the_api() {
        let hubs: Vec<Hub> = serde_json::from_value(json!([{
            "id": "h1", "name": "Hub", "createdOn": "2024-01-01T00:00:00Z",
            "shockers": [{ "id": "s1", "rfId": 1, "model": "CaiXianlin", "name": "Leg", "isPaused": false, "createdOn": "x" }]
        }]))
        .unwrap();
        assert_eq!(hubs[0].shockers[0].name, "Leg");
    }
}
