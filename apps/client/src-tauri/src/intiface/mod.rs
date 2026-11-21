//! Intiface (Buttplug v3), run from Rust so toys keep their timing while the window is hidden and
//! keep going through a webview reload. The connections, the device lists, battery readings and
//! auto-launching Intiface Central live here; what to send to the toys comes from `toys`.
//!
//! You can connect to several Intiface servers at once (this PC, a phone running Intiface Central…),
//! each by its address. Every toy carries the id of the server it's on: toy indexes are only unique
//! within one server.
//!
//! Devices running Intiface that announce themselves over mDNS (see `discovery`) are added on their
//! own unless you opt out, known by their hostname, and followed when their address changes.
//!
//! The settings (servers, devices, auto-launch) are in `intiface.json`, copied once from the webview's
//! localStorage. It's an add-on (see `addons`): the registry turns it on and off.
mod discovery;
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
pub const DEFAULT_PORT: u16 = 12345;
/// The server everything saved before there were several points at: what used to be "the" URL.
pub const THIS_PC: &str = "this-pc";
const FILE: &str = "intiface.json";
/// How many times auto-launch opens Intiface Central before giving up.
pub const MAX_LAUNCH_ATTEMPTS: u32 = 5;

/// For `#[serde(default = …)]` on toys saved before there were several servers.
pub fn this_pc() -> String {
    THIS_PC.to_owned()
}

/// The same names the webview has always used, now per server.
/// - `connecting`: an attempt is in flight, retries included.
/// - `connected`: the handshake (RequestServerInfo) succeeded. The only state toys work in.
/// - `error`: nothing listens at the address, and auto-launch didn't apply.
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

/// An Intiface server you added, by address.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub id: String,
    /// Your name for it. Empty shows the address.
    #[serde(default)]
    pub label: String,
    pub host: String,
    pub port: u16,
    /// wss://, only for an address saved that way before there were several servers.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub secure: bool
}

impl Server {
    fn url(&self) -> String {
        let host = if self.host.contains(':') { format!("[{}]", self.host) } else { self.host.clone() };
        format!("{}://{host}:{}", if self.secure { "wss" } else { "ws" }, self.port)
    }

    /// On this PC: the only kind auto-launch and the "is it running" check apply to.
    fn local(&self) -> bool {
        is_local(&self.host)
    }
}

fn is_local(host: &str) -> bool {
    ["localhost", "127.0.0.1", "::1"].contains(&host)
}

/// A device running Intiface found over mDNS (a phone, another PC). Known by its hostname: when it
/// shows up at a new address, the connection follows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetDevice {
    pub id: String,
    /// Your name for it. Empty shows the hostname.
    #[serde(default)]
    pub label: String,
    /// Without `.local`.
    pub hostname: String,
    /// Where it was last announced.
    pub host: String,
    pub port: u16
}

impl NetDevice {
    /// Connected like any server, at its last known address.
    fn as_server(&self) -> Server {
        Server { id: self.id.clone(), label: self.label.clone(), host: self.host.clone(), port: self.port, secure: false }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    /// The one URL from before there were several servers (empty for the default). Read once to
    /// make the "this PC" server, never written.
    #[serde(default, skip_serializing)]
    url: String,
    /// Set by the add-on registry. Only read from files written before the switch moved to
    /// `addons.json`.
    #[serde(default, skip_serializing)]
    enabled: bool,
    #[serde(default)]
    auto_launch: bool,
    /// None in a file from before there were several servers (see `upgrade`).
    #[serde(default)]
    servers: Option<Vec<Server>>,
    #[serde(default)]
    devices: Vec<NetDevice>,
    /// Opted out of adding devices found on the network: they're listed, and you add them.
    #[serde(default)]
    discover_only: bool,
    /// Hostnames of devices you removed, so they aren't added again on their own.
    #[serde(default)]
    ignored: Vec<String>,
    /// Every toy Intiface has listed, by server and index, so a control's toy list keeps the ones
    /// that are off right now.
    #[serde(default)]
    known_devices: Vec<Device>
}

impl Settings {
    fn servers(&self) -> &[Server] {
        self.servers.as_deref().unwrap_or_default()
    }

    /// Everything to connect to: the servers you added, then the devices.
    fn connections(&self) -> Vec<Server> {
        self.servers().iter().cloned().chain(self.devices.iter().map(NetDevice::as_server)).collect()
    }

    /// A file from before there were several servers: its URL becomes the "this PC" server.
    fn upgrade(&mut self) {
        if self.servers.is_some() {
            return;
        }
        let url = if self.url.is_empty() { DEFAULT_URL } else { &self.url };
        let (host, port) = protocol::target(url).unwrap_or_else(|| ("localhost".into(), DEFAULT_PORT));
        let secure = url.starts_with("wss://");
        self.servers = Some(vec![Server { id: THIS_PC.into(), label: String::new(), host, port, secure }]);
    }
}

pub enum LinkCommand {
    /// A batch of Buttplug messages. Ids are filled in by the link.
    Send(Vec<Value>),
    Shutdown
}

/// One server's connection while the add-on is on.
struct Link {
    /// Which task this is: callbacks from a replaced one are ignored.
    generation: u64,
    url: String,
    tx: mpsc::UnboundedSender<LinkCommand>,
    status: Status,
    devices: BTreeMap<u32, Device>,
    /// Intiface's own error message when it refused the handshake.
    refused_reason: String,
    launch_attempts: u32,
    last_launch: Option<Instant>
}

#[derive(Default)]
struct Inner {
    loaded: bool,
    settings: Settings,
    installed: bool,
    links: BTreeMap<String, Link>,
    next_generation: u64,
    /// Browsing for devices while the add-on is on.
    discovery: Option<mdns_sd::ServiceDaemon>,
    /// Announced now and not added: opted out of adding them, or removed before.
    found: Vec<discovery::Found>
}

impl Inner {
    fn current(&mut self, server: &str, generation: u64) -> Option<&mut Link> {
        self.links.get_mut(server).filter(|link| link.generation == generation)
    }
}

#[derive(Default)]
pub struct Intiface {
    inner: Mutex<Inner>
}

/// One server as the webview shows it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerView {
    #[serde(flatten)]
    server: Server,
    local: bool,
    status: Status,
    devices: Vec<Device>,
    refused_reason: String,
    launch_attempts: u32
}

/// A device as the webview shows it: its connection, and the hostname it's known by.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetDeviceView {
    #[serde(flatten)]
    connection: ServerView,
    hostname: String
}

/// What the webview shows.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    enabled: bool,
    auto_launch: bool,
    servers: Vec<ServerView>,
    network_devices: Vec<NetDeviceView>,
    discover_only: bool,
    found: Vec<discovery::Found>,
    known_devices: Vec<Device>,
    default_port: u16,
    installed: bool,
    max_launch_attempts: u32
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
/// Central install, and connects to every server.
fn start(app: &AppHandle) {
    let installed = launch::available();
    ensure_loaded(app);
    {
        let mut inner = intiface(app).inner.lock().unwrap();
        inner.settings.enabled = true;
        inner.installed = installed;
    }
    sync_links(app, |_| false);
    let daemon = discovery::start(app.clone());
    intiface(app).inner.lock().unwrap().discovery = daemon;
}

/// The add-on was turned off: patterns and toys stop first, then the connections close.
fn stop(app: &AppHandle) {
    crate::toys::patterns::stop_all(app);
    crate::toys::stop_all(app);
    let daemon = {
        let mut inner = intiface(app).inner.lock().unwrap();
        inner.settings.enabled = false;
        inner.found.clear();
        inner.discovery.take()
    };
    if let Some(Err(err)) = daemon.map(|daemon| daemon.shutdown()) {
        tracing::debug!("mDNS for Intiface didn't shut down cleanly: {err}");
    }
    sync_links(app, |_| false);
}

/// Reads `intiface.json` the first time anything needs the settings. The switch isn't in it any
/// more, so it keeps whatever the registry set.
fn ensure_loaded(app: &AppHandle) {
    if intiface(app).inner.lock().unwrap().loaded {
        return;
    }
    let mut settings = load_settings(app).unwrap_or_default();
    let upgraded = settings.servers.is_none();
    settings.upgrade();
    let saved = {
        let mut inner = intiface(app).inner.lock().unwrap();
        if inner.loaded {
            return;
        }
        inner.loaded = true;
        inner.settings = Settings { enabled: inner.settings.enabled, ..settings };
        inner.settings.clone()
    };
    if upgraded && path(app).is_some_and(|path| path.exists()) {
        tracing::info!("Intiface: your address is now the \"this PC\" server");
        save_settings(app, &saved);
    }
}

/// The first launch after this moved out of the webview: takes over its localStorage settings, and
/// keeps the add-on on if it was. A no-op once `intiface.json` exists.
pub fn migrate(app: &AppHandle, url: Option<String>, enabled: Option<bool>, auto_launch: Option<bool>) {
    if load_settings(app).is_some() {
        return;
    }
    let mut settings = Settings { url: url.map(|url| url.trim().to_owned()).unwrap_or_default(), auto_launch: auto_launch.unwrap_or(false), ..Settings::default() };
    settings.upgrade();
    let settings = {
        let mut inner = intiface(app).inner.lock().unwrap();
        inner.loaded = true;
        inner.settings = Settings { enabled: inner.settings.enabled, ..settings };
        inner.settings.clone()
    };
    save_settings(app, &settings);
    tracing::info!("Moved the Intiface settings out of the webview");
    if enabled.unwrap_or(false) {
        crate::addons::adopt(app, "intiface");
    } else if settings.enabled {
        sync_links(app, |_| false);
    }
}

fn update_settings<T>(app: &AppHandle, change: impl FnOnce(&mut Settings) -> Result<T, String>) -> Result<T, String> {
    ensure_loaded(app);
    let (result, settings) = {
        let mut inner = intiface(app).inner.lock().unwrap();
        let result = change(&mut inner.settings)?;
        (result, inner.settings.clone())
    };
    save_settings(app, &settings);
    Ok(result)
}

/// "192.168.1.20", "ws://phone.local:12345/", "[::1]": the host on its own, lowercased.
fn clean_host(host: &str) -> Result<String, String> {
    let host = host.trim();
    let host = host.strip_prefix("wss://").or_else(|| host.strip_prefix("ws://")).unwrap_or(host);
    let host = host.split('/').next().unwrap_or_default().trim_start_matches('[').trim_end_matches(']');
    if host.is_empty() || host.contains(char::is_whitespace) {
        return Err("Enter the address of the device running Intiface".into());
    }
    Ok(host.to_lowercase())
}

/// Adds a server and connects to it. Returns its id.
pub fn add_server(app: &AppHandle, label: String, host: String, port: u16) -> Result<String, String> {
    let host = clean_host(&host)?;
    let id = update_settings(app, |settings| {
        let servers = settings.servers.get_or_insert_with(Vec::new);
        if servers.iter().any(|server| server.host == host && server.port == port) {
            return Err(format!("{host}:{port} is already in your list"));
        }
        let id = format!("server-{:08x}", rand::random::<u32>());
        servers.push(Server { id: id.clone(), label: label.trim().to_owned(), host, port, secure: false });
        Ok(id)
    })?;
    sync_links(app, |_| false);
    Ok(id)
}

/// Renames a server or moves it to a new address (reconnecting to it).
pub fn update_server(app: &AppHandle, id: &str, label: String, host: String, port: u16) -> Result<(), String> {
    let host = clean_host(&host)?;
    update_settings(app, |settings| {
        let servers = settings.servers.get_or_insert_with(Vec::new);
        if servers.iter().any(|server| server.id != id && server.host == host && server.port == port) {
            return Err(format!("{host}:{port} is already in your list"));
        }
        let server = servers.iter_mut().find(|server| server.id == id).ok_or("That server is gone")?;
        let moved = server.host != host || server.port != port;
        server.label = label.trim().to_owned();
        if moved {
            server.host = host;
            server.port = port;
            server.secure = false;
        }
        Ok(())
    })?;
    sync_links(app, |_| false);
    Ok(())
}

/// Removes a server: its toys stop, and controls on them show as unavailable.
pub fn remove_server(app: &AppHandle, id: &str) -> Result<(), String> {
    stop_server(app, id);
    update_settings(app, |settings| {
        settings.servers.get_or_insert_with(Vec::new).retain(|server| server.id != id);
        settings.known_devices.retain(|device| device.server != id);
        Ok(())
    })?;
    sync_links(app, |_| false);
    Ok(())
}

/// Turning it on gives a fresh 5 tries, and retries right away where Intiface isn't running now.
pub fn set_auto_launch(app: &AppHandle, auto_launch: bool) {
    let _ = update_settings(app, |settings| {
        settings.auto_launch = auto_launch;
        Ok(())
    });
    if !auto_launch {
        emit_state(app);
        return;
    }
    reset_launch_attempts(app);
    sync_links(app, |link| link.status == Status::Error);
}

/// Opens the local install. The connection loop picks it up once its server starts.
pub fn launch_now() -> Result<(), String> {
    launch::start()
}

fn reset_launch_attempts(app: &AppHandle) {
    let mut inner = intiface(app).inner.lock().unwrap();
    for link in inner.links.values_mut() {
        link.launch_attempts = 0;
        link.last_launch = None;
        link.refused_reason.clear();
    }
}

/// Brings the connections in line with the settings: one per server while the add-on is on, a new
/// one where the address changed or `restart` says so. The others keep going untouched.
fn sync_links(app: &AppHandle, restart: impl Fn(&Link) -> bool) {
    let (closed, changed) = {
        let mut inner = intiface(app).inner.lock().unwrap();
        let wanted: Vec<(String, String)> = if inner.settings.enabled {
            inner.settings.connections().iter().map(|server| (server.id.clone(), server.url())).collect()
        } else {
            Vec::new()
        };

        let stale: Vec<String> = inner
            .links
            .iter()
            .filter(|(id, link)| restart(link) || !wanted.iter().any(|(wanted_id, url)| wanted_id == *id && *url == link.url))
            .map(|(id, _)| id.clone())
            .collect();
        let mut closed = Vec::new();
        for id in &stale {
            if let Some(link) = inner.links.remove(id) {
                closed.push(link.tx);
            }
        }

        let mut changed = !stale.is_empty();
        for (id, url) in wanted {
            if inner.links.contains_key(&id) {
                continue;
            }
            inner.next_generation += 1;
            let generation = inner.next_generation;
            let tx = link::spawn(app.clone(), id.clone(), generation, url.clone());
            inner.links.insert(
                id,
                Link { generation, url, tx, status: Status::Connecting, devices: BTreeMap::new(), refused_reason: String::new(), launch_attempts: 0, last_launch: None }
            );
            changed = true;
        }
        (closed, changed)
    };
    for tx in closed {
        let _ = tx.send(LinkCommand::Shutdown);
    }
    if changed {
        crate::toys::devices_changed(app);
    }
    emit_state(app);
}

pub fn view(app: &AppHandle) -> View {
    ensure_loaded(app);
    let inner = intiface(app).inner.lock().unwrap();
    let connection = |server: Server| {
        let link = inner.links.get(&server.id);
        ServerView {
            local: server.local(),
            status: link.map_or(Status::Disabled, |link| link.status),
            devices: link.map(|link| link.devices.values().cloned().collect()).unwrap_or_default(),
            refused_reason: link.map(|link| link.refused_reason.clone()).unwrap_or_default(),
            launch_attempts: link.map_or(0, |link| link.launch_attempts),
            server
        }
    };
    View {
        enabled: inner.settings.enabled,
        auto_launch: inner.settings.auto_launch,
        servers: inner.settings.servers().iter().cloned().map(connection).collect(),
        network_devices: inner
            .settings
            .devices
            .iter()
            .map(|device| NetDeviceView { connection: connection(device.as_server()), hostname: device.hostname.clone() })
            .collect(),
        discover_only: inner.settings.discover_only,
        found: inner.found.clone(),
        known_devices: inner.settings.known_devices.clone(),
        default_port: DEFAULT_PORT,
        installed: inner.installed,
        max_launch_attempts: MAX_LAUNCH_ATTEMPTS
    }
}

/// What to do with a device announced on the network.
#[derive(Debug, PartialEq)]
enum Seen {
    /// Already added, at the same address.
    Known,
    /// Already added: it moved, follow it.
    Moved(usize),
    /// Not added: add it.
    Add,
    /// Not added, and not to be added on its own (opted out, or you removed it): list it.
    List,
    /// The same address as a server you added by hand.
    Duplicate
}

fn seen(settings: &Settings, found: &discovery::Found) -> Seen {
    if let Some(at) = settings.devices.iter().position(|device| device.hostname == found.hostname) {
        let device = &settings.devices[at];
        return if device.host == found.host && device.port == found.port { Seen::Known } else { Seen::Moved(at) };
    }
    if settings.servers().iter().any(|server| server.host == found.host && server.port == found.port) {
        return Seen::Duplicate;
    }
    if settings.discover_only || settings.ignored.contains(&found.hostname) { Seen::List } else { Seen::Add }
}

fn new_device(found: &discovery::Found) -> NetDevice {
    NetDevice { id: format!("device-{}", found.hostname), label: String::new(), hostname: found.hostname.clone(), host: found.host.clone(), port: found.port }
}

/// A device announced itself (see `discovery`).
fn found(app: &AppHandle, found: discovery::Found) {
    ensure_loaded(app);
    let saved = {
        let mut inner = intiface(app).inner.lock().unwrap();
        if !inner.settings.enabled {
            return;
        }
        inner.found.retain(|known| known.hostname != found.hostname);
        match seen(&inner.settings, &found) {
            Seen::Known | Seen::Duplicate => None,
            Seen::Moved(at) => {
                let device = &mut inner.settings.devices[at];
                tracing::info!("Intiface on {} moved to {}:{}", device.hostname, found.host, found.port);
                device.host = found.host.clone();
                device.port = found.port;
                Some(inner.settings.clone())
            }
            Seen::Add => {
                tracing::info!("Added Intiface on {} ({}:{})", found.hostname, found.host, found.port);
                inner.settings.devices.push(new_device(&found));
                Some(inner.settings.clone())
            }
            Seen::List => {
                inner.found.push(found);
                None
            }
        }
    };
    match saved {
        Some(settings) => {
            save_settings(app, &settings);
            sync_links(app, |_| false);
        }
        None => emit_state(app),
    }
}

/// A device stopped announcing itself. An added one stays: it's tried at its last address.
fn lost(app: &AppHandle, fullname: &str) {
    let changed = {
        let mut inner = intiface(app).inner.lock().unwrap();
        let before = inner.found.len();
        inner.found.retain(|found| found.fullname != fullname);
        inner.found.len() != before
    };
    if changed {
        emit_state(app);
    }
}

/// Adds a device listed as found on the network.
pub fn add_found(app: &AppHandle, hostname: &str) -> Result<(), String> {
    let found = {
        let mut inner = intiface(app).inner.lock().unwrap();
        let at = inner.found.iter().position(|found| found.hostname == hostname).ok_or("That device isn't on the network any more")?;
        inner.found.remove(at)
    };
    update_settings(app, |settings| {
        settings.ignored.retain(|ignored| ignored != hostname);
        if !settings.devices.iter().any(|device| device.hostname == hostname) {
            settings.devices.push(new_device(&found));
        }
        Ok(())
    })?;
    sync_links(app, |_| false);
    Ok(())
}

/// Your name for a device. Empty shows its hostname.
pub fn rename_device(app: &AppHandle, id: &str, label: String) -> Result<(), String> {
    update_settings(app, |settings| {
        let device = settings.devices.iter_mut().find(|device| device.id == id).ok_or("That device is gone")?;
        device.label = label.trim().to_owned();
        Ok(())
    })?;
    sync_links(app, |_| false);
    Ok(())
}

/// Removes a device: its toys stop, and it isn't added again on its own (it's listed as found).
pub fn remove_device(app: &AppHandle, id: &str) -> Result<(), String> {
    stop_server(app, id);
    update_settings(app, |settings| {
        if let Some(device) = settings.devices.iter().find(|device| device.id == id) {
            let hostname = device.hostname.clone();
            if !settings.ignored.contains(&hostname) {
                settings.ignored.push(hostname);
            }
        }
        settings.devices.retain(|device| device.id != id);
        settings.known_devices.retain(|device| device.server != id);
        Ok(())
    })?;
    sync_links(app, |_| false);
    Ok(())
}

/// Opting back in adds what's on the network now, except devices you removed.
pub fn set_discover_only(app: &AppHandle, discover_only: bool) {
    let found = std::mem::take(&mut intiface(app).inner.lock().unwrap().found);
    let _ = update_settings(app, |settings| {
        settings.discover_only = discover_only;
        Ok(())
    });
    // Sorted out again under the new setting.
    for device in found {
        self::found(app, device);
    }
    sync_links(app, |_| false);
}

fn emit_state(app: &AppHandle) {
    let _ = app.emit("intiface-state", view(app));
    crate::addons::changed(app);
}

/// Whether at least one server is connected, so toys can be sent to.
pub fn connected(app: &AppHandle) -> bool {
    intiface(app).inner.lock().unwrap().links.values().any(|link| link.status == Status::Connected)
}

/// Whether that server is connected and has that toy now.
pub fn has_device(app: &AppHandle, server: &str, index: u32) -> bool {
    intiface(app).inner.lock().unwrap().links.get(server).is_some_and(|link| link.status == Status::Connected && link.devices.contains_key(&index))
}

/// Every motor of every toy on every server now, for automation steps on all toys.
pub fn actuators(app: &AppHandle) -> Vec<crate::toys::ActuatorRef> {
    devices(app)
        .iter()
        .flat_map(|device| {
            device.actuators.iter().map(|actuator| crate::toys::ActuatorRef {
                server: device.server.clone(),
                device_index: device.index,
                actuator_index: actuator.index,
                actuator_type: actuator.actuator_type.clone()
            })
        })
        .collect()
}

/// Sends Buttplug messages to that server when it's connected. Dropped otherwise, as the webview's
/// socket did.
pub fn send(app: &AppHandle, server: &str, messages: Vec<Value>) {
    let inner = intiface(app).inner.lock().unwrap();
    if let Some(link) = inner.links.get(server).filter(|link| link.status == Status::Connected) {
        let _ = link.tx.send(LinkCommand::Send(messages));
    }
}

/// Sends the same messages to every connected server (Stop everything).
pub fn send_all(app: &AppHandle, messages: Vec<Value>) {
    let inner = intiface(app).inner.lock().unwrap();
    for link in inner.links.values().filter(|link| link.status == Status::Connected) {
        let _ = link.tx.send(LinkCommand::Send(messages.clone()));
    }
}

/// Stops that server's toys before it goes away.
fn stop_server(app: &AppHandle, server: &str) {
    send(app, server, vec![protocol::message("StopAllDevices", serde_json::json!({}))]);
}

// Called by the link task (`server`, `generation`). Each one is ignored once that task has been
// replaced.

fn set_status(app: &AppHandle, server: &str, generation: u64, status: Status) {
    {
        let mut inner = intiface(app).inner.lock().unwrap();
        let Some(link) = inner.current(server, generation) else { return };
        if status == Status::Connected {
            link.launch_attempts = 0;
            link.last_launch = None;
            link.refused_reason.clear();
        }
        link.status = status;
    }
    emit_state(app);
}

/// What a link last showed, so a retry doesn't flicker through `connecting`.
fn shown_status(app: &AppHandle, server: &str, generation: u64) -> Option<Status> {
    intiface(app).inner.lock().unwrap().current(server, generation).map(|link| link.status)
}

fn set_devices(app: &AppHandle, server: &str, generation: u64, change: impl FnOnce(&mut BTreeMap<u32, Device>)) {
    ensure_loaded(app);
    let (learned, events) = {
        let mut inner = intiface(app).inner.lock().unwrap();
        let Some(link) = inner.current(server, generation) else { return };
        let before = link.devices.clone();
        change(&mut link.devices);
        let events = toy_events(&before, &link.devices);
        let devices: Vec<Device> = link.devices.values().cloned().collect();
        (remember(&mut inner.settings.known_devices, &devices).then(|| inner.settings.clone()), events)
    };
    if let Some(settings) = learned {
        save_settings(app, &settings);
    }
    for event in events {
        crate::automations::toy_event(app, event);
    }
    crate::toys::devices_changed(app);
    crate::haptics::devices_changed(app);
    emit_state(app);
}

/// What automations hear about a change of one server's device list: toys that came, went or ran low.
fn toy_events(before: &BTreeMap<u32, Device>, after: &BTreeMap<u32, Device>) -> Vec<crate::automations::model::ToyEvent> {
    use crate::automations::model::{ToyEvent, LOW_BATTERY};
    let low = |device: &Device| device.battery.is_some_and(|level| level < LOW_BATTERY);
    let mut events = Vec::new();
    if after.keys().any(|index| !before.contains_key(index)) {
        events.push(ToyEvent::Connected);
    }
    if before.keys().any(|index| !after.contains_key(index)) {
        events.push(ToyEvent::Disconnected);
    }
    if after.iter().any(|(index, device)| low(device) && !before.get(index).is_some_and(low)) {
        events.push(ToyEvent::BatteryLow);
    }
    events
}

/// Adds or refreshes `devices` in `known`, kept by server then index. Whether anything changed.
fn remember<'a>(known: &mut Vec<Device>, devices: impl IntoIterator<Item = &'a Device>) -> bool {
    let mut changed = false;
    for device in devices {
        let device = Device { battery: None, ..device.clone() };
        match known.binary_search_by(|known| (known.server.as_str(), known.index).cmp(&(device.server.as_str(), device.index))) {
            Ok(at) if known[at] == device => {}
            Ok(at) => {
                known[at] = device;
                changed = true;
            }
            Err(at) => {
                known.insert(at, device);
                changed = true;
            }
        }
    }
    changed
}

/// Every toy on every server now.
pub(crate) fn devices(app: &AppHandle) -> Vec<Device> {
    let inner = intiface(app).inner.lock().unwrap();
    inner.links.values().filter(|link| link.status == Status::Connected).flat_map(|link| link.devices.values().cloned()).collect()
}

/// One server's toys now, for its battery reads.
fn server_devices(app: &AppHandle, server: &str) -> Vec<Device> {
    intiface(app).inner.lock().unwrap().links.get(server).map(|link| link.devices.values().cloned().collect()).unwrap_or_default()
}

/// A connection attempt failed before the handshake finished: works out why, and launches
/// Intiface Central when auto-launch applies.
/// - Something listens: Intiface is there but refused us (`refused`).
/// - A launch is still within its 15 s: give it time (`launching`).
/// - Nothing listens, but Intiface Central runs: busy, or its server is stopped (`busy`).
/// - Not running: launch it if auto-launch is on (`launching`), else `error`.
///
/// Only a local address gets the process check and auto-launch: what runs here says nothing about
/// an Intiface on another machine.
async fn diagnose(app: &AppHandle, server: &str, generation: u64, url: &str, handshake_error: String) {
    let target = protocol::target(url);
    let local = target.as_ref().is_some_and(|(host, _)| is_local(host));
    let port_open = match &target {
        Some((host, port)) => launch::port_open(host, *port).await,
        None => false
    };
    let launch_pending = intiface(app)
        .inner
        .lock()
        .unwrap()
        .current(server, generation)
        .and_then(|link| link.last_launch)
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
    } else if local && maybe_auto_launch(app, server, generation) {
        Status::Launching
    } else {
        Status::Error
    };

    {
        let mut inner = intiface(app).inner.lock().unwrap();
        let Some(link) = inner.current(server, generation) else { return };
        link.refused_reason = if port_open { handshake_error } else { String::new() };
    }
    set_status(app, server, generation, status);
}

fn maybe_auto_launch(app: &AppHandle, server: &str, generation: u64) -> bool {
    {
        let mut inner = intiface(app).inner.lock().unwrap();
        let (auto_launch, installed) = (inner.settings.auto_launch, inner.installed);
        let Some(link) = inner.current(server, generation) else { return false };
        if !auto_launch || !installed || link.launch_attempts >= MAX_LAUNCH_ATTEMPTS {
            return false;
        }
        link.launch_attempts += 1;
        link.last_launch = Some(Instant::now());
    }
    if let Err(err) = launch::start() {
        tracing::error!("Failed to launch Intiface Central: {err}");
    }
    true
}

/// The add-on card's line: all servers summed up, or the one server's own state.
fn summary(statuses: &[(Status, usize)]) -> crate::addons::Status {
    use crate::addons::{State, Status as S};
    let toys: usize = statuses.iter().filter(|(status, _)| *status == Status::Connected).map(|(_, toys)| toys).sum();
    let connected = statuses.iter().filter(|(status, _)| *status == Status::Connected).count();
    let toys_text = match toys {
        0 => "no toys yet".to_owned(),
        1 => "1 toy".to_owned(),
        n => format!("{n} toys")
    };
    match statuses {
        [] => S::new(State::Warn, "No Intiface server added"),
        _ if connected == statuses.len() && connected > 1 => S::new(State::Ok, format!("{connected} servers · {toys_text}")),
        [(Status::Connected, _)] => S::new(State::Ok, format!("Connected · {toys_text}")),
        // Toys work as soon as one server is connected: green, and the count says the rest.
        _ if connected > 0 => S::new(State::Ok, format!("{connected} of {} servers connected · {toys_text}", statuses.len())),
        [(status, _)] => match status {
            Status::Disabled => S::new(State::Off, "Off"),
            Status::Connecting => S::new(State::Busy, "Connecting…"),
            Status::Launching => S::new(State::Busy, "Starting Intiface Central…"),
            Status::Error => S::new(State::Warn, "Intiface Central isn't running"),
            Status::Busy => S::new(State::Warn, "Intiface Central is busy, or its server is stopped"),
            Status::Refused => S::new(State::Error, "Intiface Central refused the connection"),
            Status::Connected => unreachable!()
        },
        _ if statuses.iter().any(|(status, _)| matches!(status, Status::Connecting | Status::Launching)) => S::new(State::Busy, "Connecting…"),
        _ => S::new(State::Warn, "No Intiface server reachable")
    }
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
            automation_kinds: &["toy", "toy-pattern", "stop-toys", "toy-event"],
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
        let inner = intiface(app).inner.lock().unwrap();
        if !inner.settings.enabled {
            return crate::addons::Status::new(crate::addons::State::Off, "Off");
        }
        let statuses: Vec<(Status, usize)> = inner
            .settings
            .connections()
            .iter()
            .map(|server| inner.links.get(&server.id).map_or((Status::Connecting, 0), |link| (link.status, link.devices.len())))
            .collect();
        summary(&statuses)
    }

    fn devices(&self, app: &AppHandle) -> Option<usize> {
        Some(devices(app).len())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn toy(server: &str, index: u32, name: &str) -> Device {
        Device { server: server.into(), index, name: name.into(), actuators: Vec::new(), battery_sensor_index: None, battery: Some(0.5) }
    }

    #[test]
    fn toy_events_for_automations() {
        use crate::automations::model::ToyEvent;
        let before = BTreeMap::from([(0, toy(THIS_PC, 0, "Lush"))]);
        let mut after = before.clone();
        after.insert(1, toy(THIS_PC, 1, "Edge"));
        assert_eq!(toy_events(&before, &after), vec![ToyEvent::Connected]);
        after.remove(&0);
        assert_eq!(toy_events(&before, &after), vec![ToyEvent::Connected, ToyEvent::Disconnected]);
        let mut low = before.clone();
        low.get_mut(&0).unwrap().battery = Some(0.1);
        assert_eq!(toy_events(&before, &low), vec![ToyEvent::BatteryLow]);
        assert!(toy_events(&low, &low).is_empty(), "only once");
    }

    #[test]
    fn remember_keeps_toys_apart_by_server() {
        let mut known = Vec::new();
        assert!(remember(&mut known, &[toy(THIS_PC, 2, "Edge"), toy(THIS_PC, 0, "Lush")]));
        assert_eq!(known.iter().map(|d| d.index).collect::<Vec<_>>(), [0, 2]);
        assert_eq!(known[0].battery, None);
        assert!(!remember(&mut known, &[toy(THIS_PC, 0, "Lush")]));
        assert!(remember(&mut known, &[toy(THIS_PC, 2, "Max")]));
        assert_eq!(known[1].name, "Max");
        // Same index on another server: a different toy.
        assert!(remember(&mut known, &[toy("phone", 0, "Gush")]));
        assert_eq!(known.len(), 3);
        assert_eq!(known.iter().filter(|d| d.index == 0).count(), 2);
    }

    #[test]
    fn an_old_file_becomes_the_this_pc_server() {
        let mut settings: Settings = serde_json::from_str(r#"{"url":"ws://192.168.1.5:12346","autoLaunch":true}"#).unwrap();
        settings.upgrade();
        assert_eq!(settings.servers(), [Server { id: THIS_PC.into(), label: String::new(), host: "192.168.1.5".into(), port: 12346, secure: false }]);

        let mut default: Settings = serde_json::from_str("{}").unwrap();
        default.upgrade();
        assert_eq!(default.servers()[0].url(), DEFAULT_URL);

        // Once it has a list, even an empty one, it's left alone.
        let mut emptied: Settings = serde_json::from_str(r#"{"servers":[]}"#).unwrap();
        emptied.upgrade();
        assert!(emptied.servers().is_empty());
    }

    #[test]
    fn saved_toys_without_a_server_are_on_this_pc() {
        let device: Device = serde_json::from_str(r#"{"index":1,"name":"Lush","actuators":[]}"#).unwrap();
        assert_eq!(device.server, THIS_PC);
    }

    #[test]
    fn hosts_are_cleaned_up() {
        assert_eq!(clean_host(" ws://Phone.local/ ").unwrap(), "phone.local");
        assert_eq!(clean_host("[::1]").unwrap(), "::1");
        assert!(clean_host("  ").is_err());
        assert_eq!(Server { id: "x".into(), label: String::new(), host: "::1".into(), port: 1, secure: false }.url(), "ws://[::1]:1");
    }

    fn announced(hostname: &str, host: &str) -> discovery::Found {
        discovery::Found { hostname: hostname.into(), host: host.into(), port: DEFAULT_PORT, fullname: String::new() }
    }

    #[test]
    fn devices_found_on_the_network() {
        let mut settings = Settings::default();
        settings.upgrade();
        assert_eq!(seen(&settings, &announced("pixel-7", "192.168.1.20")), Seen::Add, "added on its own by default");

        settings.devices.push(new_device(&announced("pixel-7", "192.168.1.20")));
        assert_eq!(seen(&settings, &announced("pixel-7", "192.168.1.20")), Seen::Known);
        assert_eq!(seen(&settings, &announced("pixel-7", "192.168.1.31")), Seen::Moved(0), "same hostname, new address: followed");

        settings.servers.as_mut().unwrap().push(Server { id: "s".into(), label: String::new(), host: "192.168.1.40".into(), port: DEFAULT_PORT, secure: false });
        assert_eq!(seen(&settings, &announced("tablet", "192.168.1.40")), Seen::Duplicate, "already added by address");

        settings.ignored.push("laptop".into());
        assert_eq!(seen(&settings, &announced("laptop", "192.168.1.50")), Seen::List, "removed before: not added again");

        settings.discover_only = true;
        assert_eq!(seen(&settings, &announced("ipad", "192.168.1.60")), Seen::List, "opted out: only listed");
        assert_eq!(new_device(&announced("ipad", "192.168.1.60")).as_server().url(), "ws://192.168.1.60:12345");
    }

    #[test]
    fn summary_reads_well() {
        assert_eq!(summary(&[(Status::Connected, 2)]).text, "Connected · 2 toys");
        assert_eq!(summary(&[(Status::Connected, 1), (Status::Connected, 0)]).text, "2 servers · 1 toy");
        assert_eq!(summary(&[(Status::Connected, 0), (Status::Error, 0)]).text, "1 of 2 servers connected · no toys yet");
        assert_eq!(summary(&[(Status::Connected, 0), (Status::Error, 0)]).state, crate::addons::State::Ok, "one connected is enough");
        assert_eq!(summary(&[(Status::Error, 0)]).text, "Intiface Central isn't running");
        assert_eq!(summary(&[]).text, "No Intiface server added");
    }
}
