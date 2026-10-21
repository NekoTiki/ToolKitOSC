//! VR overlay: ToolKitOSC in the headset. Toasts through XSOverlay or OVR Toolkit when a viewer
//! shocks you, someone joins, Stop everything goes on or off (see `notify`), and a panel on your
//! wrist with Stop always one poke away (see `panel`, and `crate::wrist` for the panel itself).
//! Built in Rust so both keep going while the window is in the tray. `overlay.json` (see `store`)
//! holds what sends a toast and where the panel sits.
pub mod commands;
pub mod model;
mod notify;
mod panel;
mod store;

use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

use crate::state::AppState;
use crate::wrist::protocol::Profile;
use model::{NotifySettings, Target};
use notify::{Kind, Toast};

/// How often it looks for XSOverlay, OVR Toolkit and SteamVR.
const SCAN_EVERY: Duration = Duration::from_secs(3);

#[derive(Default)]
pub struct Overlay {
    /// `None` until read from disk (see `store`).
    settings: Mutex<Option<model::OverlaySettings>>,
    /// Toasts to send, while the add-on is on.
    toasts: Mutex<Option<mpsc::UnboundedSender<Toast>>>,
    tasks: Mutex<Vec<tauri::async_runtime::JoinHandle<()>>>,
    /// Which of the two runs, as last found.
    found: Mutex<Option<Target>>,
    /// The last Stop everything state. The first one is restored at launch, not a change.
    paused: Mutex<Option<bool>>,
    panel: Mutex<panel::Panel>
}

fn overlay(app: &AppHandle) -> &Overlay {
    &app.state::<AppState>().inner().overlay
}

/// Where toasts go now: the one picked, or the one that runs. None when off or nothing runs.
fn destination(app: &AppHandle) -> Option<Target> {
    match store::get(app).notify.to {
        Target::Off => None,
        Target::Auto => *overlay(app).found.lock().unwrap(),
        picked => Some(picked)
    }
}

/// Every running process's name, in lower case.
fn process_names() -> Vec<String> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    system.processes().values().map(|process| process.name().to_string_lossy().to_lowercase()).collect()
}

fn start(app: &AppHandle) {
    let (tx, rx) = mpsc::unbounded_channel();
    *overlay(app).toasts.lock().unwrap() = Some(tx);
    let sender = tauri::async_runtime::spawn(notify::run(app.clone(), rx));
    let listener = tauri::async_runtime::spawn(panel::listen(app.clone()));
    let handle = app.clone();
    let scanner = tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(SCAN_EVERY);
        loop {
            ticker.tick().await;
            let handle = handle.clone();
            let _ = tauri::async_runtime::spawn_blocking(move || {
                let names = process_names();
                let found = notify::detect(&names);
                if std::mem::replace(&mut *overlay(&handle).found.lock().unwrap(), found) != found {
                    crate::addons::changed(&handle);
                }
                panel::supervise(&handle, names.iter().any(|name| name.starts_with("vrserver")));
            })
            .await;
        }
    });
    *overlay(app).tasks.lock().unwrap() = vec![sender, listener, scanner];
}

fn stop(app: &AppHandle) {
    overlay(app).toasts.lock().unwrap().take();
    for task in overlay(app).tasks.lock().unwrap().drain(..) {
        task.abort();
    }
    *overlay(app).found.lock().unwrap() = None;
    panel::stop(app);
}

/// Queues a toast, when the add-on is on and `wanted` says this kind sends one.
fn push(app: &AppHandle, wanted: impl FnOnce(&NotifySettings) -> bool, toast: Toast) {
    if !crate::addons::is_on(app, "overlay") || !wanted(&store::get(app).notify) {
        return;
    }
    if let Some(tx) = overlay(app).toasts.lock().unwrap().as_ref() {
        let _ = tx.send(toast);
    }
}

/// A viewer's command ran.
pub fn viewer_command(app: &AppHandle, who: &str, control: Option<&serde_json::Value>, control_type: &str, name: &str, value: Option<&serde_json::Value>) {
    let kind = notify::kind_of(control_type);
    let (what, detail) = notify::describe(control, control_type, name, value);
    if crate::addons::is_on(app, "overlay") {
        panel::note_action(app, who, &what);
    }
    let wanted = move |n: &NotifySettings| match kind {
        Kind::Shock => n.shocks,
        Kind::Toy => n.toys,
        _ => n.controls
    };
    push(app, wanted, Toast { kind, who: Some(who.to_owned()), what, detail });
}

/// Viewers joined; `watching` is how many are on now.
pub fn viewers_joined(app: &AppHandle, names: Vec<String>, watching: usize) {
    for name in names {
        push(app, |n| n.viewers, Toast { kind: Kind::Viewer, who: Some(name), what: "joined".into(), detail: format!("{watching} watching") });
    }
}

/// The viewer list changed.
pub fn viewers_changed(app: &AppHandle) {
    panel::refresh(app);
}

/// Stop everything's state, every time the webview says it; toasts only on a change.
pub fn paused(app: &AppHandle, paused: bool) {
    let before = overlay(app).paused.lock().unwrap().replace(paused);
    panel::refresh(app);
    if before != Some(!paused) {
        return;
    }
    let toast = if paused {
        Toast { kind: Kind::Stop, who: None, what: "everything stopped".into(), detail: "Viewers can't use your controls until you resume.".into() }
    } else {
        Toast { kind: Kind::Stop, who: None, what: "controls resumed".into(), detail: "Viewers can use your controls again.".into() }
    };
    push(app, |n| n.stop, toast);
}

/// An automation ran.
pub fn automation_ran(app: &AppHandle, name: &str) {
    push(app, |n| n.automations, Toast { kind: Kind::Automation, who: None, what: format!("automation {name} ran"), detail: String::new() });
}

/// The profiles and the active one, from the webview.
pub fn set_context(app: &AppHandle, profiles: Vec<Profile>, profile: Option<String>, groups: Vec<Profile>) {
    panel::set_context(app, profiles, profile, groups);
}

/// The settings changed: the panel gets them, and may start again after crashing.
fn settings_changed(app: &AppHandle) {
    overlay(app).panel.lock().unwrap().gave_up = false;
    panel::refresh(app);
    crate::addons::changed(app);
}

/// The VR overlay as an add-on (see `crate::addons`).
pub struct Addon;

impl crate::addons::Addon for Addon {
    fn manifest(&self) -> crate::addons::Manifest {
        crate::addons::Manifest {
            id: "overlay",
            name: "VR overlay",
            group: "vr",
            pitch: "ToolKitOSC on your wrist in SteamVR, with Stop one poke away, and toasts in XSOverlay or OVR Toolkit",
            // Built and tried on Windows first; SteamVR on Linux follows.
            platforms: &["windows"],
            badges: &["beta"],
            needs: &[],
            needs_any: &[],
            control_types: &[],
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
        use crate::addons::{State, Status};
        let settings = store::get(app);
        let (running, gave_up) = {
            let panel = overlay(app).panel.lock().unwrap();
            (panel.running(), panel.gave_up)
        };
        if gave_up {
            return Status::new(State::Error, "Panel stopped after crashing 3 times");
        }
        let panel = match (settings.panel.enabled, running) {
            (false, _) => None,
            (true, true) => Some("Panel on your wrist"),
            (true, false) => Some("Panel waits for SteamVR")
        };
        let toasts = match (settings.notify.to, destination(app)) {
            (Target::Off, _) => None,
            (_, Some(Target::Xsoverlay)) => Some("toasts to XSOverlay"),
            (_, Some(Target::Ovrtoolkit)) => Some("toasts to OVR Toolkit"),
            _ => Some("waiting for XSOverlay or OVR Toolkit")
        };
        let text = match (panel, toasts) {
            (Some(panel), Some(toasts)) => format!("{panel} · {toasts}"),
            (Some(panel), None) => panel.to_owned(),
            (None, Some(toasts)) => notify::capitalize(toasts),
            (None, None) => "Panel and toasts off".to_owned()
        };
        let waiting = !running && toasts.map_or(true, |toasts| toasts.starts_with("waiting"));
        Status::new(if waiting { State::Warn } else { State::Ok }, text)
    }

    fn stop_everything(&self, _app: &AppHandle) -> crate::addons::StopFuture {
        Box::pin(async { Ok(()) })
    }

    fn was_in_use(&self, _app: &AppHandle) -> bool {
        false
    }
}
