//! Intiface (Buttplug v3), run from Rust so toys keep their timing while the window is hidden and
//! keep going through a webview reload. The connection, the device list, battery readings and
//! auto-launching Intiface Central live here; what to send to the toys comes from `toys`.
//!
//! The settings (URL, auto-launch) are in `intiface.json`, copied once from the webview's
//! localStorage. It's an add-on (see `addons`): the registry turns it on and off.
pub mod launch;
mod link;
pub mod protocol;
pub mod commands;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;

use crate::state::AppState;
pub use protocol::Device;

pub const DEFAULT_URL: &str = "ws://localhost:12345";
const FILE: &str = "intiface.json";
/// How many times auto-launch opens Intiface Central before giving up.
pub const MAX_LAUNCH_ATTEMPTS: u32 = 5;

/// The same names the webview has always used.
/// - `connecting`: an attempt is in flight, retries included.
/// - `connected`: the handshake (RequestServerInfo) succeeded. The only state toys work in.
/// - `error`: nothing listens at the URL, and auto-launch didn't apply.
/// - `refused`: something listens but wouldn't take us, e.g. another app is connected.
/// - `busy`: nothing listens, but Intiface Central runs here: busy with another client, or its
///   server is stopped.
/// - `launching`: auto-launch opened Intiface Central and is giving it time to start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Disabled,
    Connecting,
    Connected,
    Error,
    Refused,
    Busy,
    Launching
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    /// Empty for the default.
    #[serde(default)]
    url: String,
    /// Set by the add-on registry. Only read from files written before the switch moved to
    /// `addons.json`.
    #[serde(default, skip_serializing)]
    enabled: bool,
    #[serde(default)]
    auto_launch: bool
}

pub enum LinkCommand {
    /// A batch of Buttplug messages. Ids are filled in by the link.
    Send(Vec<Value>),
    Shutdown
}

#[derive(Default)]
struct Inner {
    loaded: bool,
    settings: Settings,
    status: Option<Status>,
    devices: BTreeMap<u32, Device>,
    installed: bool,
    launch_attempts: u32,
    last_launch: Option<Instant>,
    /// Intiface's own error message when it refused the handshake.
    refused_reason: String,
    link: Option<(u64, mpsc::UnboundedSender<LinkCommand>)>,
    next_link_id: u64
}

#[derive(Default)]
pub struct Intiface {
    inner: Mutex<Inner>
}

/// What the webview shows.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    url: String,
    default_url: &'static str,
    enabled: bool,
    auto_launch: bool,
    status: Status,
    devices: Vec<Device>,
    installed: bool,
    launch_attempts: u32,
    max_launch_attempts: u32,
    refused_reason: String
}

fn intiface(app: &AppHandle) -> &Intiface {
    &app.state::<AppState>().inner().intiface
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_data_dir().ok()?;
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join(FILE))
}

fn load_settings(app: &AppHandle) -> Option<Settings> {
    serde_json::from_str(&std::fs::read_to_string(path(app)?).ok()?).ok()
}

fn save_settings(app: &AppHandle, settings: &Settings) {
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

/// The add-on was turned on (or is on at launch): reads the settings once, looks for an Intiface
/// Central install, and connects.
fn start(app: &AppHandle) {
    let installed = launch::available();
    ensure_loaded(app);
    {
        let mut inner = intiface(app).inner.lock().unwrap();
        inner.settings.enabled = true;
        inner.installed = installed;
    }
    reset_launch_attempts(app);
    restart(app);
}

/// The add-on was turned off: patterns and toys stop first, then the connection closes.
fn stop(app: &AppHandle) {
    crate::toys::patterns::stop_all(app);
    crate::toys::stop_all(app);
    intiface(app).inner.lock().unwrap().settings.enabled = false;
    restart(app);
}

/// Reads `intiface.json` the first time anything needs the settings. The switch isn't in it any
/// more, so it keeps whatever the registry set.
fn ensure_loaded(app: &AppHandle) {
    if intiface(app).inner.lock().unwrap().loaded {
        return;
    }
    let settings = load_settings(app).unwrap_or_default();
    let mut inner = intiface(app).inner.lock().unwrap();
    if !inner.loaded {
        inner.loaded = true;
        inner.settings = Settings { enabled: inner.settings.enabled, ..settings };
    }
}

/// The first launch after this moved out of the webview: takes over its localStorage settings, and
/// keeps the add-on on if it was. A no-op once `intiface.json` exists.
pub fn migrate(app: &AppHandle, url: Option<String>, enabled: Option<bool>, auto_launch: Option<bool>) {
    if load_settings(app).is_some() {
        return;
    }
    ensure_loaded(app);
    let settings = {
        let mut inner = intiface(app).inner.lock().unwrap();
        inner.settings.url = normalize_url(url.as_deref());
        inner.settings.auto_launch = auto_launch.unwrap_or(false);
        inner.settings.clone()
    };
    save_settings(app, &settings);
    tracing::info!("Moved the Intiface settings out of the webview");
    if enabled.unwrap_or(false) {
        crate::addons::adopt(app, "intiface");
    } else if settings.enabled {
        restart(app);
    }
}

fn normalize_url(url: Option<&str>) -> String {
    match url.map(str::trim) {
        Some(url) if !url.is_empty() && url != DEFAULT_URL => url.to_owned(),
        _ => String::new()
    }
}

fn update_settings(app: &AppHandle, change: impl FnOnce(&mut Settings)) -> Settings {
    ensure_loaded(app);
    let settings = {
        let mut inner = intiface(app).inner.lock().unwrap();
        change(&mut inner.settings);
        inner.settings.clone()
    };
    save_settings(app, &settings);
    settings
}

/// Reconnects against the new URL right away.
pub fn set_url(app: &AppHandle, url: Option<String>) {
    update_settings(app, |settings| settings.url = normalize_url(url.as_deref()));
    reset_launch_attempts(app);
    restart(app);
}

/// Turning it on gives a fresh 5 tries, and retries right away if Intiface isn't running now.
pub fn set_auto_launch(app: &AppHandle, auto_launch: bool) {
    update_settings(app, |settings| settings.auto_launch = auto_launch);
    if !auto_launch {
        emit_state(app);
        return;
    }
    reset_launch_attempts(app);
    if intiface(app).inner.lock().unwrap().status == Some(Status::Error) {
        restart(app);
    } else {
        emit_state(app);
    }
}

/// Opens the local install. The connection loop picks it up once its server starts.
pub fn launch_now() -> Result<(), String> {
    launch::start()
}

fn reset_launch_attempts(app: &AppHandle) {
    let mut inner = intiface(app).inner.lock().unwrap();
    inner.launch_attempts = 0;
    inner.last_launch = None;
    inner.refused_reason.clear();
}

/// Stops the current link and starts a new one when the switch is on.
fn restart(app: &AppHandle) {
    let (old, url) = {
        let mut inner = intiface(app).inner.lock().unwrap();
        let old = inner.link.take();
        inner.devices.clear();
        let url = inner.settings.enabled.then(|| {
            if inner.settings.url.is_empty() { DEFAULT_URL.to_owned() } else { inner.settings.url.clone() }
        });
        if let Some(url) = &url {
            inner.next_link_id += 1;
            let id = inner.next_link_id;
            inner.link = Some((id, link::spawn(app.clone(), id, url.clone())));
            inner.status = Some(Status::Connecting);
        } else {
            inner.status = Some(Status::Disabled);
        }
        (old, url)
    };
    if let Some((_, tx)) = old {
        let _ = tx.send(LinkCommand::Shutdown);
    }
    if url.is_none() {
        tracing::debug!("Intiface is off");
    }
    crate::toys::devices_changed(app);
    emit_state(app);
}

pub fn view(app: &AppHandle) -> View {
    ensure_loaded(app);
    let inner = intiface(app).inner.lock().unwrap();
    View {
        url: inner.settings.url.clone(),
        default_url: DEFAULT_URL,
        enabled: inner.settings.enabled,
        auto_launch: inner.settings.auto_launch,
        status: inner.status.unwrap_or(Status::Disabled),
        devices: inner.devices.values().cloned().collect(),
        installed: inner.installed,
        launch_attempts: inner.launch_attempts,
        max_launch_attempts: MAX_LAUNCH_ATTEMPTS,
        refused_reason: inner.refused_reason.clone()
    }
}

fn emit_state(app: &AppHandle) {
    let _ = app.emit("intiface-state", view(app));
    crate::addons::changed(app);
}

fn is_current(inner: &Inner, id: u64) -> bool {
    inner.link.as_ref().is_some_and(|(link_id, _)| *link_id == id)
}

/// Whether the handshake is done and toys can be sent to.
pub fn connected(app: &AppHandle) -> bool {
    intiface(app).inner.lock().unwrap().status == Some(Status::Connected)
}

/// The devices Intiface has now, by index.
pub fn has_device(app: &AppHandle, index: u32) -> bool {
    intiface(app).inner.lock().unwrap().devices.contains_key(&index)
}

/// Sends Buttplug messages when connected. Dropped otherwise, as the webview's socket did.
pub fn send(app: &AppHandle, messages: Vec<Value>) {
    let inner = intiface(app).inner.lock().unwrap();
    if inner.status != Some(Status::Connected) {
        return;
    }
    if let Some((_, tx)) = &inner.link {
        let _ = tx.send(LinkCommand::Send(messages));
    }
}

// Called by the link task `id`. Each one is ignored once that task has been replaced.

fn set_status(app: &AppHandle, id: u64, status: Status) {
    {
        let mut inner = intiface(app).inner.lock().unwrap();
        if !is_current(&inner, id) {
            return;
        }
        if status == Status::Connected {
            inner.launch_attempts = 0;
            inner.last_launch = None;
            inner.refused_reason.clear();
        }
        inner.status = Some(status);
    }
    emit_state(app);
}

fn set_devices(app: &AppHandle, id: u64, change: impl FnOnce(&mut BTreeMap<u32, Device>)) {
    {
        let mut inner = intiface(app).inner.lock().unwrap();
        if !is_current(&inner, id) {
            return;
        }
        change(&mut inner.devices);
    }
    crate::toys::devices_changed(app);
    crate::haptics::devices_changed(app);
    emit_state(app);
}

pub(crate) fn devices(app: &AppHandle) -> Vec<Device> {
    intiface(app).inner.lock().unwrap().devices.values().cloned().collect()
}

/// A connection attempt failed before the handshake finished: works out why, and launches
/// Intiface Central when auto-launch applies.
/// - Something listens: Intiface is there but refused us (`refused`).
/// - A launch is still within its 15 s: give it time (`launching`).
/// - Nothing listens, but Intiface Central runs: busy, or its server is stopped (`busy`).
/// - Not running: launch it if auto-launch is on (`launching`), else `error`.
///
/// Only a local URL gets the process check and auto-launch: what runs here says nothing about an
/// Intiface on another machine.
async fn diagnose(app: &AppHandle, id: u64, url: &str, handshake_error: String) {
    let target = protocol::target(url);
    let local = target.as_ref().is_some_and(|(host, _)| ["localhost", "127.0.0.1", "::1"].contains(&host.as_str()));
    let port_open = match &target {
        Some((host, port)) => launch::port_open(host, *port).await,
        None => false
    };
    let launch_pending = intiface(app)
        .inner
        .lock()
        .unwrap()
        .last_launch
        .is_some_and(|at| at.elapsed() < link::LAUNCH_RETRY_DELAY);
    let running = !port_open && !launch_pending && local && {
        // Unknown counts as running: never risk opening a second copy.
        tauri::async_runtime::spawn_blocking(launch::running).await.unwrap_or(true)
    };

    let status = if port_open {
        Status::Refused
    } else if launch_pending {
        Status::Launching
    } else if running {
        Status::Busy
    } else if local && maybe_auto_launch(app, id) {
        Status::Launching
    } else {
        Status::Error
    };

    {
        let mut inner = intiface(app).inner.lock().unwrap();
        if !is_current(&inner, id) {
            return;
        }
        inner.refused_reason = if port_open { handshake_error } else { String::new() };
    }
    set_status(app, id, status);
}

fn maybe_auto_launch(app: &AppHandle, id: u64) -> bool {
    {
        let mut inner = intiface(app).inner.lock().unwrap();
        if !is_current(&inner, id)
            || !inner.settings.auto_launch
            || !inner.installed
            || inner.launch_attempts >= MAX_LAUNCH_ATTEMPTS
        {
            return false;
        }
        inner.launch_attempts += 1;
        inner.last_launch = Some(Instant::now());
    }
    if let Err(err) = launch::start() {
        tracing::error!("Failed to launch Intiface Central: {err}");
    }
    true
}

/// Intiface toys as an add-on (see `crate::addons`). It also owns the toys and patterns in
/// `crate::toys`.
pub struct Addon;

impl crate::addons::Addon for Addon {
    fn manifest(&self) -> crate::addons::Manifest {
        crate::addons::Manifest {
            id: "intiface",
            name: "Intiface toys",
            group: "toys",
            pitch: "Toys through Intiface Central, with patterns your viewers can play",
            platforms: &["windows", "linux"],
            badges: &[],
            needs: &[],
            needs_any: &[],
            control_types: &["intiface-toy", "intiface-pattern"],
            automation_kinds: &[],
            pages: &["patterns"]
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
        let inner = intiface(app).inner.lock().unwrap();
        let toys = inner.devices.len();
        match inner.status.unwrap_or(Status::Disabled) {
            Status::Disabled => S::new(State::Off, "Off"),
            Status::Connecting => S::new(State::Busy, "Connecting…"),
            Status::Launching => S::new(State::Busy, "Starting Intiface Central…"),
            Status::Connected if toys == 0 => S::new(State::Ok, "Connected · no toys yet"),
            Status::Connected => S::new(State::Ok, format!("Connected · {toys} toy{}", if toys == 1 { "" } else { "s" })),
            Status::Error => S::new(State::Warn, "Intiface Central isn't running"),
            Status::Busy => S::new(State::Warn, "Intiface Central is busy, or its server is stopped"),
            Status::Refused => S::new(State::Error, "Intiface Central refused the connection")
        }
    }

    fn stop_everything(&self, app: &AppHandle) -> crate::addons::StopFuture {
        crate::toys::patterns::stop_all(app);
        crate::toys::stop_all(app);
        Box::pin(async { Ok(()) })
    }

    fn was_in_use(&self, app: &AppHandle) -> bool {
        load_settings(app).is_some_and(|settings| settings.enabled)
    }
}
