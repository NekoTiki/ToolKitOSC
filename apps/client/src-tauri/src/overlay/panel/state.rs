//! What the panel shows, built from what the app knows now: one function per section of
//! `protocol::State`. A new page gets its section here.
use std::time::Duration;

use serde_json::Value;
use tauri::{AppHandle, Manager};

use super::{overlay, pinned, store};
use crate::wrist::protocol::{AddonRow, Home, Pinned, State, Viewer};

const RECENT: Duration = Duration::from_secs(60);

pub fn build(app: &AppHandle) -> State {
    let settings = store::get(app);
    let viewers = viewers(app);
    let pinned = pinned(app, &settings.pinned);
    let addons = addons(app);
    let (watching, signed_in) = crate::server::viewer_counts(app);
    let paused = crate::automations::is_paused(app);
    let mut panel = overlay(app).panel.lock().unwrap();
    while panel.recent.front().is_some_and(|at| at.elapsed() > RECENT) {
        panel.recent.pop_front();
    }
    State {
        paused,
        panel: settings.panel,
        theme: panel.theme.clone(),
        home: Home { watching, signed_in, recent: panel.recent.len(), profile: panel.profile.clone(), profiles: panel.profiles.clone() },
        activity: panel.activity.iter().cloned().collect(),
        viewers,
        pinned,
        addons
    }
}

/// Who's on the share page, with their groups' names and what they did last.
fn viewers(app: &AppHandle) -> Vec<Viewer> {
    let viewers = crate::server::viewers(app);
    let panel = overlay(app).panel.lock().unwrap();
    viewers
        .into_iter()
        .map(|(viewer, groups)| Viewer {
            last: panel.activity.iter().find(|entry| entry.who == viewer.display_name).cloned(),
            groups: groups.iter().filter_map(|id| panel.groups.iter().find(|group| &group.id == id).map(|group| group.name.clone())).collect(),
            signed_in: viewer.discord_id.is_some(),
            name: viewer.display_name
        })
        .collect()
}

fn pinned(app: &AppHandle, by_avatar: &std::collections::BTreeMap<String, Vec<String>>) -> Pinned {
    let avatar = app.state::<crate::state::AppState>().avatar_details.lock().unwrap().as_ref().map(|details| (details.id.clone(), details.name.clone()));
    let ids = avatar.as_ref().and_then(|(id, _)| by_avatar.get(id)).map_or(&[][..], Vec::as_slice);
    let (tiles, nothing_pinned) = pinned::tiles(app, ids);
    Pinned { avatar: avatar.map(|(_, name)| name), tiles, nothing_pinned }
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

/// The add-ons that are on, as their cards say, and a row per toy control with its level.
fn addons(app: &AppHandle) -> Vec<AddonRow> {
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
    // Only the toys are copied out of the controls.
    let toys: Vec<(String, String, bool)> = crate::server::with_control_groups(app, |groups| {
        groups
            .iter()
            .filter_map(|group| group.get("controls")?.as_array())
            .flatten()
            .filter(|control| text(control, "type") == Some("intiface-toy"))
            .map(|control| (text(control, "name").unwrap_or_default().to_owned(), text(control, "id").unwrap_or_default().to_owned(), control.get("paused").and_then(Value::as_bool).unwrap_or(false)))
            .collect()
    });
    rows.extend(toys.into_iter().map(|(name, id, touched)| AddonRow { name, status: "Toy".into(), level: Some(crate::controls::toy_level(&id)), touched }));
    rows
}
