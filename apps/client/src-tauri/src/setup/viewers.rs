//! Viewer groups (Friends, Mods…), who is in them, and bans, in `viewers.json`. Global, not per
//! avatar: they're about people. Viewers themselves (names, activity) stay in the webview's
//! database. Changes go to the webview as `setup-viewers`.
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

const FILE: &str = "viewers.json";

/// Built in while the AI assistants add-on is on: every assistant is in it (see `mcp`).
pub const ASSISTANTS: &str = "assistants";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewerGroup {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub color: String
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Membership {
    /// The group's id.
    pub id: String,
    /// When it ends, in Unix ms. None: until the host removes them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<u64>
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ban {
    /// `ip` (a stable IP, see `server::viewer::stable_ip`) or `discord`. Never both: banning by IP
    /// doesn't spare that IP under another Discord account.
    pub scope: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Unix ms.
    #[serde(default)]
    pub created_at: u64
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ViewersFile {
    /// The groups you made, without Assistants.
    #[serde(default)]
    pub groups: Vec<ViewerGroup>,
    /// Viewer key (`discord:<id>` or `guest:<peerId>`) -> their groups, expired ones included until
    /// the next change to that viewer.
    #[serde(default)]
    pub memberships: BTreeMap<String, Vec<Membership>>,
    #[serde(default)]
    pub bans: Vec<Ban>
}

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn active(membership: &Membership, now: u64) -> bool {
    membership.until.is_none_or(|until| until > now)
}

fn path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(FILE))
}

fn read_file(app: &AppHandle) -> ViewersFile {
    let Ok(data) = path(app).and_then(|path| std::fs::read_to_string(path).map_err(|e| e.to_string())) else {
        return ViewersFile::default();
    };
    serde_json::from_str(&data).unwrap_or_else(|err| {
        tracing::error!("Couldn't read {FILE}: {err}");
        ViewersFile::default()
    })
}

fn write_file(app: &AppHandle, file: &ViewersFile) -> Result<(), String> {
    let path = path(app)?;
    let tmp_path = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(file).map_err(|e| e.to_string())?;
    std::fs::write(&tmp_path, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())
}

/// Runs `f` on the file (read from disk the first time). When it says something changed, saves
/// it, tells the webview, and works the controls out again (profiles' rules for a deleted group
/// stop counting).
fn with<T>(app: &AppHandle, f: impl FnOnce(&mut ViewersFile) -> (T, bool)) -> Result<T, String> {
    let (result, changed) = {
        let mut guard = super::setup(app).viewers.lock().unwrap();
        let file = guard.get_or_insert_with(|| read_file(app));
        let (result, changed) = f(file);
        if changed {
            write_file(app, file)?;
        }
        (result, changed.then(|| file.clone()))
    };
    if let Some(file) = changed {
        let _ = app.emit("setup-viewers", &file);
        super::refresh(app);
        super::sync(app, false);
        crate::overlay::viewers_changed(app);
    }
    Ok(result)
}

fn read<T>(app: &AppHandle, f: impl FnOnce(&ViewersFile) -> T) -> T {
    with(app, |file| (f(file), false)).expect("reading never writes")
}

pub fn snapshot(app: &AppHandle) -> ViewersFile {
    read(app, Clone::clone)
}

/// The groups you made, plus Assistants while that add-on is on.
pub fn group_ids(app: &AppHandle) -> Vec<String> {
    let mut ids = read(app, |file| file.groups.iter().map(|group| group.id.clone()).collect::<Vec<_>>());
    if crate::addons::is_on(app, "mcp") {
        ids.push(ASSISTANTS.to_owned());
    }
    ids
}

/// The groups you made with their names, plus Assistants while that add-on is on.
pub fn groups(app: &AppHandle) -> Vec<ViewerGroup> {
    let mut groups = read(app, |file| file.groups.clone());
    if crate::addons::is_on(app, "mcp") {
        groups.push(ViewerGroup { id: ASSISTANTS.to_owned(), name: "Assistants".to_owned(), color: "#a78bfa".to_owned() });
    }
    groups
}

fn active_ids(file: &ViewersFile, key: &str, now: u64) -> Vec<String> {
    let exists = |id: &str| file.groups.iter().any(|group| group.id == id);
    file.memberships.get(key).into_iter().flatten().filter(|m| active(m, now) && exists(&m.id)).map(|m| m.id.clone()).collect()
}

/// The groups this viewer is in right now.
pub fn groups_of(app: &AppHandle, key: &str) -> Vec<String> {
    read(app, |file| active_ids(file, key, now_ms()))
}

/// When the next temporary membership ends.
pub fn next_expiry(app: &AppHandle) -> Option<u64> {
    let now = now_ms();
    read(app, |file| file.memberships.values().flatten().filter_map(|m| m.until).filter(|until| *until > now).min())
}

pub fn banned(app: &AppHandle, ip: &str, discord_id: Option<&str>) -> Option<Ban> {
    read(app, |file| file.bans.iter().find(|ban| (ban.scope == "ip" && ban.value == ip) || (ban.scope == "discord" && Some(ban.value.as_str()) == discord_id)).cloned())
}

/// Saves the groups you made, in this order. Anyone in a group that's gone is taken out of it.
pub fn save_groups(app: &AppHandle, groups: Vec<ViewerGroup>) -> Result<(), String> {
    with(app, |file| {
        let groups: Vec<ViewerGroup> = groups.into_iter().filter(|group| group.id != ASSISTANTS).collect();
        for list in file.memberships.values_mut() {
            list.retain(|m| groups.iter().any(|group| group.id == m.id));
        }
        file.memberships.retain(|_, list| !list.is_empty());
        file.groups = groups;
        ((), true)
    })
}

/// Changes one viewer's groups, dropping their expired ones.
fn set_memberships(app: &AppHandle, key: &str, f: impl FnOnce(&mut Vec<Membership>)) -> Result<(), String> {
    let now = now_ms();
    with(app, |file| {
        let list = file.memberships.entry(key.to_owned()).or_default();
        list.retain(|m| active(m, now));
        f(list);
        if list.is_empty() {
            file.memberships.remove(key);
        }
        ((), true)
    })
}

pub fn add_to_group(app: &AppHandle, key: &str, group_id: &str, duration_ms: Option<u64>) -> Result<(), String> {
    if group_id == ASSISTANTS || !read(app, |file| file.groups.iter().any(|group| group.id == group_id)) {
        return Err("No such viewer group".to_owned());
    }
    set_memberships(app, key, |list| {
        list.retain(|m| m.id != group_id);
        list.push(Membership { id: group_id.to_owned(), until: duration_ms.map(|ms| now_ms() + ms) });
    })
}

pub fn remove_from_group(app: &AppHandle, key: &str, group_id: &str) -> Result<(), String> {
    set_memberships(app, key, |list| list.retain(|m| m.id != group_id))
}

/// Two lists of memberships as one, keeping the longer of two for the same group.
fn merge(into: &mut Vec<Membership>, from: Vec<Membership>) {
    for membership in from {
        match into.iter_mut().find(|m| m.id == membership.id) {
            Some(seen) => {
                if membership.until.is_none() || seen.until.is_some_and(|until| membership.until > Some(until)) {
                    *seen = membership;
                }
            }
            None => into.push(membership)
        }
    }
}

/// A guest signed in with Discord: their guest groups become their account's.
pub fn fold_guest(app: &AppHandle, guest_key: &str, discord_key: &str) {
    if !read(app, |file| file.memberships.contains_key(guest_key)) {
        return;
    }
    let result = with(app, |file| {
        let from = file.memberships.remove(guest_key).unwrap_or_default();
        merge(file.memberships.entry(discord_key.to_owned()).or_default(), from);
        ((), true)
    });
    if let Err(err) = result {
        tracing::error!("Couldn't move a guest's viewer groups: {err}");
    }
}

/// Bans by IP or Discord id, or updates the reason of an existing ban.
pub fn ban(app: &AppHandle, scope: String, value: String, reason: Option<String>) -> Result<(), String> {
    if scope != "ip" && scope != "discord" {
        return Err(format!("Can't ban by {scope}"));
    }
    with(app, |file| {
        match file.bans.iter_mut().find(|ban| ban.scope == scope && ban.value == value) {
            Some(existing) => existing.reason = reason.or(existing.reason.take()),
            None => file.bans.push(Ban { scope, value, reason, created_at: now_ms() })
        }
        ((), true)
    })
}

pub fn unban(app: &AppHandle, scope: &str, value: &str) -> Result<(), String> {
    with(app, |file| {
        let before = file.bans.len();
        file.bans.retain(|ban| !(ban.scope == scope && ban.value == value));
        ((), file.bans.len() != before)
    })
}

/// The first launch after this moved out of the webview: takes over its groups, memberships and
/// bans. A no-op once `viewers.json` exists.
pub fn migrate(app: &AppHandle, legacy: ViewersFile) -> Result<(), String> {
    if path(app)?.exists() {
        return Ok(());
    }
    tracing::info!("Moved {} viewer group(s), {} member(s) and {} ban(s) out of the webview", legacy.groups.len(), legacy.memberships.len(), legacy.bans.len());
    with(app, |file| {
        *file = legacy;
        ((), true)
    })
}

/// Tells the webview, the share page and the panel when a temporary membership ends.
pub async fn watch_expiry(app: AppHandle) {
    loop {
        // Checked again every minute, so a membership added meanwhile is never waited past by long.
        let next = next_expiry(&app);
        let wait = next.map_or(60_000, |until| until.saturating_sub(now_ms()) + 50).min(60_000);
        tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
        if next.is_none_or(|until| until > now_ms()) {
            continue;
        }
        let _ = app.emit("setup-viewers", snapshot(&app));
        super::refresh(&app);
        super::sync(&app, false);
        crate::overlay::viewers_changed(&app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(id: &str, until: Option<u64>) -> Membership {
        Membership { id: id.into(), until }
    }

    #[test]
    fn merge_keeps_the_longer() {
        let mut into = vec![m("a", Some(10)), m("b", None)];
        merge(&mut into, vec![m("a", Some(20)), m("b", Some(5)), m("c", Some(1))]);
        assert_eq!(into, vec![m("a", Some(20)), m("b", None), m("c", Some(1))]);
        let mut forever = vec![m("a", Some(10))];
        merge(&mut forever, vec![m("a", None)]);
        assert_eq!(forever, vec![m("a", None)]);
    }

    #[test]
    fn active_ids_drop_expired_and_deleted() {
        let file = ViewersFile {
            groups: vec![ViewerGroup { id: "a".into(), name: "A".into(), color: String::new() }],
            memberships: BTreeMap::from([("guest:1".to_owned(), vec![m("a", Some(100)), m("gone", None)])]),
            bans: vec![]
        };
        assert_eq!(active_ids(&file, "guest:1", 50), vec!["a".to_owned()]);
        assert!(active_ids(&file, "guest:1", 100).is_empty());
    }
}
