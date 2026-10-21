//! The app's end of the wrist panel (see `crate::wrist`): starts `toolkitosc --overlay` while
//! SteamVR runs (here), hosts the localhost WebSocket it connects to and acts on what's pressed
//! there (see `link`), builds what it shows (see `state`, and `pinned` for the Pinned page).
//!
//! Anything that changes what the panel shows calls `refresh`, which only wakes the connection: the
//! state is built there, at most once per wake or live tick, and only while a panel is connected.
mod link;
pub mod pinned;
mod state;

use std::collections::{HashMap, VecDeque};
use std::process::{Child, Command};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};
use tokio::sync::watch;

use super::{overlay, store};
use crate::types::OscMessage;
use crate::wrist::protocol::{Entry, Profile, Theme, TileKind};
pub use link::listen;

/// The panel crashing this many times within `CRASH_WINDOW` stops it until you change a setting.
const MAX_CRASHES: usize = 3;
const CRASH_WINDOW: Duration = Duration::from_secs(60);
const ACTIVITY: usize = 20;

#[derive(Default)]
pub struct Panel {
    /// Wakes the connected panel to get the state again, while the add-on is on.
    wake: Option<watch::Sender<()>>,
    /// The localhost port and the token, while the add-on is on.
    listening: Option<(u16, String)>,
    child: Option<Child>,
    crashes: Vec<Instant>,
    /// Stopped after crashing too often.
    pub gave_up: bool,
    /// Profiles and the active one, the viewer groups and the theme, from the webview (see
    /// `set_context`).
    profiles: Vec<Profile>,
    profile: Option<String>,
    /// Viewer groups by id, for their names.
    groups: Vec<Profile>,
    theme: Theme,
    /// Newest first.
    activity: VecDeque<Entry>,
    recent: VecDeque<Instant>,
    /// Control id -> what a press on its tile made it, and its parameters' values from before the
    /// press (see `pinned`).
    expected: HashMap<String, (TileKind, Vec<Option<OscMessage>>)>
}

impl Panel {
    pub fn running(&self) -> bool {
        self.child.is_some()
    }
}

/// Has the connected panel get the state again; it's sent if it changed.
pub fn refresh(app: &AppHandle) {
    if let Some(wake) = &overlay(app).panel.lock().unwrap().wake {
        wake.send_replace(());
    }
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// A viewer's action, for Activity, Last and the count of the last minute.
pub fn note_action(app: &AppHandle, who: &str, what: &str) {
    {
        let mut panel = overlay(app).panel.lock().unwrap();
        panel.activity.push_front(Entry { at: now_ms(), who: who.to_owned(), what: what.to_owned() });
        panel.activity.truncate(ACTIVITY);
        panel.recent.push_back(Instant::now());
    }
    refresh(app);
}

pub fn set_context(app: &AppHandle, profiles: Vec<Profile>, profile: Option<String>, groups: Vec<Profile>, theme: Theme) {
    {
        let mut panel = overlay(app).panel.lock().unwrap();
        panel.profiles = profiles;
        panel.profile = profile;
        panel.groups = groups;
        panel.theme = theme;
    }
    refresh(app);
}

/// Starts, restarts or stops the panel process. Called every few seconds while the add-on is on,
/// with whether SteamVR runs.
pub fn supervise(app: &AppHandle, steamvr: bool) {
    let want = steamvr && store::get(app).panel.enabled;
    let mut changed = false;
    {
        let mut panel = overlay(app).panel.lock().unwrap();
        if let Some(status) = panel.child.as_mut().and_then(|child| child.try_wait().ok().flatten()) {
            tracing::info!("Panel exited: {status}");
            panel.child = None;
            changed = true;
            if !status.success() {
                panel.crashes.push(Instant::now());
            }
        }
        panel.crashes.retain(|at| at.elapsed() < CRASH_WINDOW);
        if want && panel.child.is_none() && !panel.gave_up {
            if panel.crashes.len() >= MAX_CRASHES {
                tracing::warn!("Panel crashed {MAX_CRASHES} times in a minute, stopped");
                panel.gave_up = true;
                changed = true;
            } else if let Some((port, token)) = panel.listening.clone() {
                // Its log goes to a file next to the settings, to look at after a session in VR.
                let log = app.path().app_data_dir().ok().and_then(|dir| std::fs::File::create(dir.join("overlay-panel.log")).ok());
                let mut command = std::env::current_exe().map(Command::new);
                if let (Ok(command), Some(log)) = (&mut command, log) {
                    if let Ok(copy) = log.try_clone() {
                        command.stdout(copy);
                    }
                    command.stderr(log);
                }
                match command.and_then(|mut command| command.args(["--overlay", &port.to_string(), &token]).spawn()) {
                    Ok(child) => {
                        panel.child = Some(child);
                        changed = true;
                    }
                    Err(err) => tracing::error!("Panel: couldn't start: {err}")
                }
            }
        }
        if !want {
            changed |= kill(&mut panel);
        }
    }
    if changed {
        crate::addons::changed(app);
    }
}

fn kill(panel: &mut Panel) -> bool {
    let Some(mut child) = panel.child.take() else {
        return false;
    };
    let _ = child.kill();
    let _ = child.wait();
    true
}

/// The add-on stopped.
pub fn stop(app: &AppHandle) {
    let mut panel = overlay(app).panel.lock().unwrap();
    kill(&mut panel);
    panel.wake = None;
    panel.listening = None;
    panel.gave_up = false;
    panel.crashes.clear();
}
