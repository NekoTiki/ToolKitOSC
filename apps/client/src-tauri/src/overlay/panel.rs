//! The app's end of the wrist panel (see `crate::wrist`): starts `toolkitosc --overlay` while
//! SteamVR runs, hosts the localhost WebSocket it connects to, sends it the state whenever that
//! changes, and acts on what's pressed there the way the app's own window would.
//!
//! The socket only listens on 127.0.0.1, and only a connection that opens with the one-time token
//! passed on the panel's command line is answered.
use std::collections::VecDeque;
use std::process::{Child, Command};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use tauri::{AppHandle, Emitter, Manager};
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;

use super::{overlay, store};
use crate::wrist::protocol::{AddonRow, Entry, Home, Pinned, Profile, State, ToApp, Viewer};

/// The panel crashing this many times within `CRASH_WINDOW` stops it until you change a setting.
const MAX_CRASHES: usize = 3;
const CRASH_WINDOW: Duration = Duration::from_secs(60);
const RECENT: Duration = Duration::from_secs(60);
const ACTIVITY: usize = 20;
/// While the panel is connected, its state is checked this often: toy levels, the leash and touch
/// change without an event of their own.
const LIVE_EVERY: Duration = Duration::from_millis(250);

#[derive(Default)]
pub struct Panel {
    /// The state the panel shows, while the add-on is on.
    state: Option<watch::Sender<State>>,
    /// The localhost port and the token, while the add-on is on.
    listening: Option<(u16, String)>,
    child: Option<Child>,
    crashes: Vec<Instant>,
    /// Stopped after crashing too often.
    pub gave_up: bool,
    /// Profiles and the active one, from the webview (see `set_context`).
    profiles: Vec<Profile>,
    profile: Option<String>,
    /// Viewer groups by id, for their names.
    groups: Vec<Profile>,
    /// Newest first.
    activity: VecDeque<Entry>,
    recent: VecDeque<Instant>
}

impl Panel {
    pub fn running(&self) -> bool {
        self.child.is_some()
    }
}

/// Opens the socket. Called when the add-on starts.
pub async fn listen(app: AppHandle) {
    let listener = match TcpListener::bind("127.0.0.1:0").await {
        Ok(listener) => listener,
        Err(err) => {
            tracing::error!("Panel: couldn't open the local socket: {err}");
            return;
        }
    };
    let port = listener.local_addr().map(|addr| addr.port()).unwrap_or_default();
    let token = format!("{:032x}", rand::random::<u128>());
    {
        let mut panel = overlay(&app).panel.lock().unwrap();
        panel.listening = Some((port, token.clone()));
        panel.state = Some(watch::channel(State::default()).0);
    }
    refresh(&app);
    while let Ok((stream, _)) = listener.accept().await {
        let app = app.clone();
        let token = token.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(err) = serve(&app, stream, &token).await {
                tracing::debug!("Panel link closed: {err}");
            }
        });
    }
}

async fn serve(app: &AppHandle, stream: tokio::net::TcpStream, token: &str) -> Result<(), String> {
    let socket = tokio_tungstenite::accept_async(stream).await.map_err(|e| e.to_string())?;
    let (mut write, mut read) = socket.split();
    let hello = tokio::time::timeout(Duration::from_secs(5), read.next()).await.map_err(|_| "no hello")?;
    let Some(Ok(Message::Text(text))) = hello else {
        return Err("no hello".into());
    };
    if !matches!(serde_json::from_str(&text), Ok(ToApp::Hello { token: sent }) if sent == token) {
        return Err("wrong token".into());
    }
    let mut states = overlay(app).panel.lock().unwrap().state.as_ref().ok_or("the add-on is off")?.subscribe();
    states.mark_changed();
    let mut live = tokio::time::interval(LIVE_EVERY);
    loop {
        tokio::select! {
            _ = live.tick() => refresh(app),
            changed = states.changed() => {
                changed.map_err(|_| "the add-on stopped")?;
                let json = serde_json::to_string(&*states.borrow_and_update()).unwrap();
                write.send(Message::text(json)).await.map_err(|e| e.to_string())?;
            }
            message = read.next() => match message {
                Some(Ok(Message::Text(text))) => match serde_json::from_str::<ToApp>(&text) {
                    Ok(action) => act(app, action),
                    Err(err) => tracing::warn!("Panel: unknown message: {err}")
                },
                Some(Ok(_)) => {}
                _ => return Ok(())
            }
        }
    }
}

/// Something pressed on the panel. Pause and profiles live in the webview, which gets
/// `overlay-action`; Stop also stops here right away, in case the window is busy.
fn act(app: &AppHandle, action: ToApp) {
    tracing::info!("Panel: {action:?}");
    if action == ToApp::Stop {
        crate::automations::set_paused(app, true);
        let handle = app.clone();
        tauri::async_runtime::spawn(async move {
            crate::addons::stop_everything(&handle).await;
        });
    }
    match &action {
        ToApp::Control { group_id, control_id, level } => {
            super::tiles::press(app, group_id, control_id, *level);
            refresh(app);
        }
        ToApp::Stop | ToApp::Resume | ToApp::Profile { .. } => {
            let _ = app.emit("overlay-action", &action);
        }
        ToApp::Hello { .. } => {}
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

pub fn set_context(app: &AppHandle, profiles: Vec<Profile>, profile: Option<String>, groups: Vec<Profile>) {
    {
        let mut panel = overlay(app).panel.lock().unwrap();
        panel.profiles = profiles;
        panel.profile = profile;
        panel.groups = groups;
    }
    refresh(app);
}

fn text<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(serde_json::Value::as_str)
}

/// The add-ons that are on, as their cards say, and a row per toy control with its level.
fn addon_rows(app: &AppHandle) -> Vec<AddonRow> {
    let mut rows: Vec<AddonRow> = crate::addons::statuses(app)
        .into_iter()
        .filter(|(id, _, _)| *id != "overlay")
        .map(|(id, name, status)| {
            // The leash says who pulls and how hard.
            let leash = (id == "leash").then(|| crate::leash::view(app)).filter(|view| view.active.is_some());
            AddonRow {
                name: name.to_owned(),
                status: leash.as_ref().and_then(|view| view.active.clone()).map_or(status.text.clone(), |active| format!("{active} · {}", status.text)),
                level: leash.map(|view| view.stretch),
                touched: false
            }
        })
        .collect();
    let groups = crate::server::control_groups(app);
    let toys = groups.iter().filter_map(|group| group.get("controls")?.as_array()).flatten().filter(|control| text(control, "type") == Some("intiface-toy"));
    for control in toys {
        rows.push(AddonRow {
            name: text(control, "name").unwrap_or_default().to_owned(),
            status: "Toy".into(),
            level: Some(crate::controls::toy_level(text(control, "id").unwrap_or_default())),
            touched: control.get("paused").and_then(serde_json::Value::as_bool).unwrap_or(false)
        });
    }
    rows
}

/// Sends the panel the state again, if it changed.
pub fn refresh(app: &AppHandle) {
    if overlay(app).panel.lock().unwrap().state.is_none() {
        return;
    }
    let paused = crate::automations::is_paused(app);
    let (watching, signed_in) = crate::server::viewer_counts(app);
    let viewers = crate::server::viewers(app);
    let settings = store::get(app).panel;
    let (pinned, nothing_pinned) = super::tiles::pinned(app);
    let addons = addon_rows(app);
    let avatar = app.state::<crate::state::AppState>().avatar_details.lock().unwrap().as_ref().map(|details| details.name.clone());
    let mut panel = overlay(app).panel.lock().unwrap();
    let viewers = viewers
        .into_iter()
        .map(|(viewer, groups)| Viewer {
            last: panel.activity.iter().find(|entry| entry.who == viewer.display_name).cloned(),
            groups: groups.iter().filter_map(|id| panel.groups.iter().find(|group| &group.id == id).map(|group| group.name.clone())).collect(),
            signed_in: viewer.discord_id.is_some(),
            name: viewer.display_name
        })
        .collect();
    while panel.recent.front().is_some_and(|at| at.elapsed() > RECENT) {
        panel.recent.pop_front();
    }
    let state = State {
        paused,
        panel: settings,
        theme: Default::default(),
        home: Home { watching, signed_in, recent: panel.recent.len(), profile: panel.profile.clone(), profiles: panel.profiles.clone() },
        activity: panel.activity.iter().cloned().collect(),
        viewers,
        pinned: Pinned { avatar, tiles: pinned, nothing_pinned },
        addons
    };
    if let Some(tx) = &panel.state {
        tx.send_if_modified(|current| {
            let changed = *current != state;
            *current = state;
            changed
        });
    }
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
    // The count of the last minute goes down by itself.
    refresh(app);
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
    panel.state = None;
    panel.listening = None;
    panel.gave_up = false;
    panel.crashes.clear();
}
