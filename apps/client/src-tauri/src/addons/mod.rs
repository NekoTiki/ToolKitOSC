//! Add-ons: the integrations you turn on only if you own the gear (OpenShock, Intiface toys, ESP32
//! boards). They all ship in the app, and each stays off until you turn it on. One that's off opens
//! no connection, and the core never calls it: it asks `control_type_live` / `automation_kind_live`
//! here first, and Stop everything goes out through `stop_everything`.
//!
//! The switches are in `addons.json` (see `store`). Only the app window changes them, through
//! `addons_set_enabled`: no automation, viewer, controller or avatar parameter can.
mod store;
pub mod commands;

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;
use store::AddonsFile;

/// What an add-on is and what it brings, shown on the Add-ons page and used to gate the core.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub id: &'static str,
    pub name: &'static str,
    /// Which group of cards it's in on the Add-ons page.
    pub group: &'static str,
    /// One line on what it does, shown while it's off.
    pub pitch: &'static str,
    pub platforms: &'static [&'static str],
    /// A safety note shown as a badge, e.g. `shocks`.
    pub badge: Option<&'static str>,
    /// Add-ons it can't work without.
    pub needs: &'static [&'static str],
    /// The control types it owns (`types/controls.ts`).
    pub control_types: &'static [&'static str],
    /// The automation step kinds it owns (`automations::model::Action`).
    pub automation_kinds: &'static [&'static str],
    /// The rail pages it brings (router names).
    pub pages: &'static [&'static str]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum State {
    Off,
    /// Connecting, checking or starting.
    Busy,
    Ok,
    /// Works, but something needs you: Intiface Central isn't running, a hub is offline.
    Warn,
    Error
}

/// An add-on's live status, as its card shows it.
#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub state: State,
    pub text: String
}

impl Status {
    pub fn new(state: State, text: impl Into<String>) -> Self {
        Self { state, text: text.into() }
    }
}

pub type StopFuture = Pin<Box<dyn Future<Output = Result<(), String>> + Send>>;

/// What every add-on fills in. The modules keep their own state and Tauri commands; this is only
/// how the registry starts, stops and asks them.
pub trait Addon: Sync {
    fn manifest(&self) -> Manifest;
    /// Opens its connections. Called at launch when it's on, and when you turn it on.
    fn start(&self, app: &AppHandle);
    /// Stops its devices first, then disconnects. Nothing it made is deleted.
    fn stop(&self, app: &AppHandle);
    fn status(&self, app: &AppHandle) -> Status;
    /// Its part of Stop everything.
    fn stop_everything(&self, app: &AppHandle) -> StopFuture;
    /// For the first launch with add-ons: whether you were already using it, so it stays on.
    /// May block (OpenShock reads the password store).
    fn was_in_use(&self, app: &AppHandle) -> bool;
}

/// Every add-on, in the order the Add-ons page lists them.
static ADDONS: &[&dyn Addon] = &[&crate::openshock::Addon, &crate::intiface::Addon, &crate::haptics::Addon, &crate::leash::Addon, &crate::boards::Addon];

fn find(id: &str) -> Option<&'static dyn Addon> {
    ADDONS.iter().copied().find(|addon| addon.manifest().id == id)
}

pub struct Addons {
    file: Mutex<AddonsFile>,
    /// Hand-offs `adopt` got before `start` loaded the switches. None once they're loaded.
    queued: Mutex<Option<Vec<String>>>
}

impl Default for Addons {
    fn default() -> Self {
        Self { file: Mutex::new(AddonsFile::default()), queued: Mutex::new(Some(Vec::new())) }
    }
}

fn addons(app: &AppHandle) -> &Addons {
    &app.state::<AppState>().inner().addons
}

/// At launch, off the main thread and after `secrets::init`: loads the switches (deciding them
/// from what you already use the first time), then starts the add-ons that are on.
pub fn start(app: &AppHandle) {
    let file = store::load(app).unwrap_or_else(|| {
        let kept: Vec<String> = ADDONS
            .iter()
            .filter(|addon| addon.was_in_use(app))
            .map(|addon| addon.manifest().id.to_owned())
            .collect();
        let file = AddonsFile::migrated(&kept);
        tracing::info!("Add-ons decided from what was in use: {kept:?}");
        store::save(app, &file);
        file
    });

    {
        // Under the queue's lock, so a hand-off from the webview is either queued and applied
        // here, or arrives after and applies itself.
        let mut queued = addons(app).queued.lock().unwrap();
        *addons(app).file.lock().unwrap() = file;
        for id in queued.take().unwrap_or_default() {
            adopt_switch(app, &id);
        }
    }
    let on: HashSet<String> = addons(app).file.lock().unwrap().enabled_ids();

    let manifests: Vec<Manifest> = ADDONS.iter().map(|addon| addon.manifest()).collect();
    for id in start_order(&manifests, &on) {
        if let Some(addon) = find(&id) {
            tracing::info!("Starting add-on {id}");
            addon.start(app);
        }
    }
    changed(app);
}

/// The add-ons in `on`, each after the ones it needs. One whose needs aren't all on is left out.
fn start_order(manifests: &[Manifest], on: &HashSet<String>) -> Vec<String> {
    let mut order: Vec<String> = Vec::new();
    let mut left: Vec<&Manifest> = manifests.iter().filter(|m| on.contains(m.id)).collect();
    loop {
        let before = left.len();
        left.retain(|m| {
            let ready = m.needs.iter().all(|need| order.iter().any(|id| id == need));
            if ready {
                order.push(m.id.to_owned());
            }
            !ready
        });
        if left.is_empty() || left.len() == before {
            for m in left {
                tracing::warn!("Add-on {} needs {:?}, which isn't on", m.id, m.needs);
            }
            return order;
        }
    }
}

pub fn is_on(app: &AppHandle, id: &str) -> bool {
    addons(app).file.lock().unwrap().is_enabled(id)
}

fn manifests() -> Vec<Manifest> {
    ADDONS.iter().map(|addon| addon.manifest()).collect()
}

fn find_control_type(manifests: Vec<Manifest>, control_type: &str) -> Option<Manifest> {
    manifests.into_iter().find(|m| m.control_types.contains(&control_type))
}

fn find_automation_kind(manifests: Vec<Manifest>, kind: &str) -> Option<Manifest> {
    manifests.into_iter().find(|m| m.automation_kinds.contains(&kind))
}

/// The add-on that owns this control type, if any.
pub fn owner_of_control_type(control_type: &str) -> Option<Manifest> {
    find_control_type(manifests(), control_type)
}

/// Whether controls of this type can run: always for core types, only while its add-on is on
/// otherwise.
pub fn control_type_live(app: &AppHandle, control_type: &str) -> bool {
    owner_of_control_type(control_type).map_or(true, |owner| is_on(app, owner.id))
}

/// The add-on that owns this automation step kind, if any.
pub fn owner_of_automation_kind(kind: &str) -> Option<Manifest> {
    find_automation_kind(manifests(), kind)
}

/// The error a command of an add-on that's off answers with, e.g. "OpenShock is off".
pub fn require(app: &AppHandle, id: &str) -> Result<(), String> {
    if is_on(app, id) {
        return Ok(());
    }
    let name = find(id).map_or(id, |addon| addon.manifest().name);
    Err(format!("{name} is off"))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonView {
    #[serde(flatten)]
    manifest: Manifest,
    enabled: bool,
    status: Status
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonsView {
    addons: Vec<AddonView>,
    /// Whether you've been through "What do you use?" (or already used an add-on before).
    setup_done: bool,
    /// What the first launch with add-ons kept on, until the notice saying so is dismissed.
    kept: Vec<String>
}

pub fn view(app: &AppHandle) -> AddonsView {
    let (enabled, setup_done, kept) = {
        let file = addons(app).file.lock().unwrap();
        (file.enabled_ids(), file.setup_done, file.kept.clone())
    };
    AddonsView {
        addons: ADDONS
            .iter()
            .map(|addon| {
                let manifest = addon.manifest();
                let on = enabled.contains(manifest.id);
                AddonView {
                    status: if on { addon.status(app) } else { Status::new(State::Off, "Off") },
                    enabled: on,
                    manifest
                }
            })
            .collect(),
        setup_done,
        kept
    }
}

/// Something about an add-on changed: its switch or its status.
pub fn changed(app: &AppHandle) {
    let _ = app.emit("addons-update", view(app));
}

/// Turns an add-on on or off, saves it, and starts or stops it. Turning one off stops its devices
/// first.
pub fn set_enabled(app: &AppHandle, id: &str, enabled: bool) -> Result<(), String> {
    let addon = find(id).ok_or_else(|| format!("There's no add-on called {id}"))?;
    let file = {
        let mut file = addons(app).file.lock().unwrap();
        if file.is_enabled(id) == enabled {
            return Ok(());
        }
        file.set_enabled(id, enabled);
        file.clone()
    };
    store::save(app, &file);
    tracing::info!("Add-on {id} turned {}", if enabled { "on" } else { "off" });
    if enabled {
        addon.start(app);
    } else {
        addon.stop(app);
    }
    changed(app);
    Ok(())
}

/// A legacy hand-off from the webview's localStorage (`openshock::migrate`, `intiface::migrate`)
/// said this add-on was on: only old installs get here, before `addons.json` knew about it.
pub fn adopt(app: &AppHandle, id: &str) {
    {
        let mut queued = addons(app).queued.lock().unwrap();
        if let Some(queued) = queued.as_mut() {
            // The switches aren't loaded yet: `start` applies it, then starts what's on.
            queued.push(id.to_owned());
            return;
        }
    }
    if adopt_switch(app, id) {
        if let Some(addon) = find(id) {
            addon.start(app);
        }
        changed(app);
    }
}

/// Turns it on in `addons.json` and lists it as kept. False when it already was on.
fn adopt_switch(app: &AppHandle, id: &str) -> bool {
    let file = {
        let mut file = addons(app).file.lock().unwrap();
        if file.is_enabled(id) {
            return false;
        }
        file.set_enabled(id, true);
        file.setup_done = true;
        if !file.kept.iter().any(|k| k == id) {
            file.kept.push(id.to_owned());
        }
        file.clone()
    };
    store::save(app, &file);
    true
}

/// "What do you use?" was answered or skipped, or the notice about what was kept was dismissed.
pub fn finish_setup(app: &AppHandle, dismiss_kept: bool) {
    let file = {
        let mut file = addons(app).file.lock().unwrap();
        file.setup_done = true;
        if dismiss_kept {
            file.kept.clear();
        }
        file.clone()
    };
    store::save(app, &file);
    changed(app);
}

#[derive(Debug, Clone, Serialize)]
pub struct StopFailure {
    id: &'static str,
    message: String
}

/// Stop everything, for every add-on that's on. Returns the ones that couldn't be reached.
pub async fn stop_everything(app: &AppHandle) -> Vec<StopFailure> {
    let stops: Vec<(&'static str, StopFuture)> = ADDONS
        .iter()
        .filter(|addon| is_on(app, addon.manifest().id))
        .map(|addon| (addon.manifest().id, addon.stop_everything(app)))
        .collect();

    let mut failures = Vec::new();
    for (id, stop) in stops {
        if let Err(message) = stop.await {
            tracing::warn!("Stop everything couldn't reach {id}: {message}");
            failures.push(StopFailure { id, message });
        }
    }
    failures
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(id: &'static str, needs: &'static [&'static str]) -> Manifest {
        Manifest {
            id,
            name: id,
            group: "toys",
            pitch: "",
            platforms: &[],
            badge: None,
            needs,
            control_types: &[],
            automation_kinds: &[],
            pages: &[]
        }
    }

    fn on(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    #[test]
    fn start_order_puts_needs_first() {
        let manifests = [manifest("haptics", &["intiface"]), manifest("intiface", &[]), manifest("boards", &[])];
        assert_eq!(start_order(&manifests, &on(&["haptics", "intiface", "boards"])), ["intiface", "boards", "haptics"]);
    }

    #[test]
    fn start_order_leaves_out_what_misses_a_need() {
        let manifests = [manifest("haptics", &["intiface"]), manifest("intiface", &[])];
        assert_eq!(start_order(&manifests, &on(&["haptics"])), Vec::<String>::new());
        assert_eq!(start_order(&manifests, &on(&[])), Vec::<String>::new());
    }

    /// The real manifests, called directly: going through `ADDONS` would link the whole app (and
    /// Tauri's Windows dialogs) into the test binary.
    fn real() -> Vec<Manifest> {
        vec![
            Addon::manifest(&crate::openshock::Addon),
            Addon::manifest(&crate::intiface::Addon),
            Addon::manifest(&crate::haptics::Addon),
            Addon::manifest(&crate::leash::Addon),
            Addon::manifest(&crate::boards::Addon)
        ]
    }

    #[test]
    fn control_types_belong_to_their_add_on() {
        assert_eq!(find_control_type(real(), "open-shock-shocker").map(|m| m.id), Some("openshock"));
        assert_eq!(find_control_type(real(), "intiface-toy").map(|m| m.id), Some("intiface"));
        assert_eq!(find_control_type(real(), "intiface-pattern").map(|m| m.id), Some("intiface"));
        assert!(find_control_type(real(), "slider").is_none());
        assert_eq!(find_automation_kind(real(), "board-output").map(|m| m.id), Some("boards"));
        assert!(find_automation_kind(real(), "control").is_none());
    }
}
