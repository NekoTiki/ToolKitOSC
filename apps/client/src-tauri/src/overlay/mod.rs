//! VR overlay: ToolKitOSC in the headset. For now, toasts through XSOverlay or OVR Toolkit when a
//! viewer shocks you, someone joins, Stop everything goes on or off (see `notify`). Built in Rust
//! so they keep coming while the window is in the tray. `overlay.json` (see `store`) says which
//! events send one and where to.
pub mod commands;
pub mod model;
mod notify;
mod store;

use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

use crate::state::AppState;
use model::{NotifySettings, Target};
use notify::{Kind, Toast};

/// How often it looks for XSOverlay and OVR Toolkit.
const DETECT_EVERY: Duration = Duration::from_secs(10);

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
    paused: Mutex<Option<bool>>
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

fn start(app: &AppHandle) {
    let (tx, rx) = mpsc::unbounded_channel();
    *overlay(app).toasts.lock().unwrap() = Some(tx);
    let sender = tauri::async_runtime::spawn(notify::run(app.clone(), rx));
    let handle = app.clone();
    let detector = tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(DETECT_EVERY);
        loop {
            ticker.tick().await;
            let found = tauri::async_runtime::spawn_blocking(notify::detect).await.unwrap_or(None);
            let changed = std::mem::replace(&mut *overlay(&handle).found.lock().unwrap(), found) != found;
            if changed {
                crate::addons::changed(&handle);
            }
        }
    });
    *overlay(app).tasks.lock().unwrap() = vec![sender, detector];
}

fn stop(app: &AppHandle) {
    overlay(app).toasts.lock().unwrap().take();
    for task in overlay(app).tasks.lock().unwrap().drain(..) {
        task.abort();
    }
    *overlay(app).found.lock().unwrap() = None;
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

/// Stop everything's state, every time the webview says it; toasts only on a change.
pub fn paused(app: &AppHandle, paused: bool) {
    let before = overlay(app).paused.lock().unwrap().replace(paused);
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

/// The VR overlay as an add-on (see `crate::addons`).
pub struct Addon;

impl crate::addons::Addon for Addon {
    fn manifest(&self) -> crate::addons::Manifest {
        crate::addons::Manifest {
            id: "overlay",
            name: "VR overlay",
            group: "vr",
            pitch: "See what viewers do while you're in VR, as toasts in XSOverlay or OVR Toolkit",
            // XSOverlay and OVR Toolkit are Windows only.
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
        if store::get(app).notify.to == Target::Off {
            return Status::new(State::Ok, "Toasts off");
        }
        match destination(app) {
            Some(Target::Xsoverlay) => Status::new(State::Ok, "Toasts to XSOverlay"),
            Some(Target::Ovrtoolkit) => Status::new(State::Ok, "Toasts to OVR Toolkit"),
            _ => Status::new(State::Warn, "Waiting for XSOverlay or OVR Toolkit")
        }
    }

    fn stop_everything(&self, _app: &AppHandle) -> crate::addons::StopFuture {
        Box::pin(async { Ok(()) })
    }

    fn was_in_use(&self, _app: &AppHandle) -> bool {
        false
    }
}
