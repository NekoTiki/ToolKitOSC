//! Your controls, their groups and the lock profiles of each avatar, in `controls-<avatarId>.json`.
//! Rust owns them, so viewer commands, automations, assistants and the wrist panel see the same
//! controls whether or not the window is open. The webview shows and edits them through
//! `commands`, and follows `setup-controls`.
//!
//! Controls stay JSON as the webview writes them (see `types/controls.ts`), like in `controls.rs`.
//! What everything reads is the resolved copy (see `resolve`), worked out again by `refresh`
//! whenever something it depends on changes.
pub mod commands;
mod resolve;

use std::cell::OnceCell;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;

/// A lock profile: what viewers may do with each control while it's on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub locked_controls: BTreeMap<String, bool>,
    /// Per control id.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub limits: BTreeMap<String, Value>,
    /// What each viewer group gets instead, by group id, then control id.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub group_overrides: BTreeMap<String, BTreeMap<String, Value>>,
    /// Set while it's an assistant's draft.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<crate::types::CreatedBy>
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlsFile {
    #[serde(default)]
    pub groups: Vec<Value>,
    #[serde(default)]
    pub profiles: Vec<Profile>,
    /// The profile that turns on the first time the avatar loads after a start.
    #[serde(default)]
    pub default_profile: Option<String>
}

/// What the webview shows: the worn avatar's controls, resolved, and its profiles.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub avatar_id: Option<String>,
    pub groups: Vec<Value>,
    pub profiles: Vec<Profile>,
    pub default_profile: Option<String>,
    pub active_profile: Option<String>
}

#[derive(Default)]
struct Inner {
    /// The avatar `file` belongs to.
    avatar_id: Option<String>,
    file: ControlsFile,
    /// The profile picked for each avatar since the app started, `None` for "No profile". An avatar
    /// with no entry gets its default.
    picked: HashMap<String, Option<String>>,
    /// What `refresh` last worked out.
    snapshot: Snapshot
}

#[derive(Default)]
pub struct Setup {
    inner: Mutex<Inner>,
    /// One `refresh` at a time, so an older one can't land after a newer one.
    refreshing: Mutex<()>
}

fn setup(app: &AppHandle) -> &Setup {
    &app.state::<AppState>().inner().setup
}

fn worn_avatar(app: &AppHandle) -> Option<String> {
    app.state::<AppState>().avatar_details.lock().unwrap().as_ref().map(|details| details.id.clone())
}

/// VRChat's `avtr_<uuid>`, or `local:<uuid>` for an avatar built and tested from Unity. Anything
/// else never reaches a file name.
pub fn valid_avatar_id(id: &str) -> bool {
    let rest = id.strip_prefix("avtr_").or_else(|| id.strip_prefix("local:"));
    rest.is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
}

fn path(app: &AppHandle, avatar_id: &str) -> Result<PathBuf, String> {
    if !valid_avatar_id(avatar_id) {
        return Err(format!("Not an avatar id: {avatar_id}"));
    }
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // Windows doesn't take `:` in a file name.
    Ok(dir.join(format!("controls-{}.json", avatar_id.replace(':', "_"))))
}

fn read_file(app: &AppHandle, avatar_id: &str) -> ControlsFile {
    let Ok(data) = path(app, avatar_id).and_then(|path| std::fs::read_to_string(path).map_err(|e| e.to_string())) else {
        return ControlsFile::default();
    };
    serde_json::from_str(&data).unwrap_or_else(|err| {
        // Left on disk as it is: the next save would overwrite it, so this is loud.
        tracing::error!("Couldn't read the controls of {avatar_id}: {err}");
        ControlsFile::default()
    })
}

fn write_file(app: &AppHandle, avatar_id: &str, file: &ControlsFile) -> Result<(), String> {
    let path = path(app, avatar_id)?;
    let tmp_path = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(file).map_err(|e| e.to_string())?;
    std::fs::write(&tmp_path, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())
}

/// Brings `inner` to the worn avatar, reading its file the first time.
fn load_worn(app: &AppHandle, inner: &mut Inner) {
    let worn = worn_avatar(app);
    if inner.avatar_id != worn {
        inner.file = worn.as_deref().map(|id| read_file(app, id)).unwrap_or_default();
        inner.avatar_id = worn;
    }
}

fn active_profile(inner: &Inner) -> Option<String> {
    let exists = |id: &Option<String>| id.as_ref().filter(|id| inner.file.profiles.iter().any(|profile| &profile.id == *id)).cloned();
    match inner.avatar_id.as_ref().and_then(|avatar| inner.picked.get(avatar)) {
        Some(None) => None,
        Some(picked @ Some(_)) => exists(picked).or_else(|| exists(&inner.file.default_profile)),
        None => exists(&inner.file.default_profile)
    }
}

/// Changes the worn avatar's file and saves it, then refreshes.
fn change(app: &AppHandle, avatar_id: &str, f: impl FnOnce(&mut Inner)) -> Result<(), String> {
    {
        let mut inner = setup(app).inner.lock().unwrap();
        load_worn(app, &mut inner);
        if inner.avatar_id.as_deref() != Some(avatar_id) {
            return Err("That avatar isn't worn anymore".to_owned());
        }
        f(&mut inner);
        write_file(app, avatar_id, &inner.file)?;
    }
    refresh(app);
    Ok(())
}

pub fn snapshot(app: &AppHandle) -> Snapshot {
    setup(app).inner.lock().unwrap().snapshot.clone()
}

/// Saves the worn avatar's groups (the order they come in), without anything `resolve` added.
pub fn save_groups(app: &AppHandle, avatar_id: &str, mut groups: Vec<Value>) -> Result<(), String> {
    for control in groups.iter_mut().filter_map(|group| group.get_mut("controls")?.as_array_mut()).flatten() {
        resolve::strip(control);
    }
    change(app, avatar_id, |inner| inner.file.groups = groups)
}

pub fn save_profiles(app: &AppHandle, avatar_id: &str, profiles: Vec<Profile>, default_profile: Option<String>) -> Result<(), String> {
    change(app, avatar_id, |inner| {
        let exists = |id: &String| profiles.iter().any(|profile| &profile.id == id);
        // Deleting the profile that's on leaves none on, not the default.
        if let Some(picked) = inner.picked.get_mut(avatar_id) {
            if picked.as_ref().is_some_and(|id| !exists(id)) {
                *picked = None;
            }
        } else if active_profile(inner).is_some_and(|id| !exists(&id)) {
            inner.picked.insert(avatar_id.to_owned(), None);
        }
        inner.file.default_profile = default_profile.filter(exists);
        inner.file.profiles = profiles;
    })
}

/// Turns a profile on for the worn avatar, or none, until the app closes.
pub fn switch_profile(app: &AppHandle, profile_id: Option<String>) -> Result<(), String> {
    {
        let mut inner = setup(app).inner.lock().unwrap();
        load_worn(app, &mut inner);
        let avatar = inner.avatar_id.clone().ok_or("No avatar yet")?;
        if profile_id.as_ref().is_some_and(|id| !inner.file.profiles.iter().any(|profile| &profile.id == id)) {
            return Err("No such profile".to_owned());
        }
        inner.picked.insert(avatar, profile_id);
    }
    refresh(app);
    Ok(())
}

/// One avatar's setup as the webview kept it in localStorage.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Legacy {
    avatar_id: String,
    #[serde(default)]
    groups: Vec<Value>,
    /// Loose, so one damaged profile doesn't stop the rest.
    #[serde(default)]
    profiles: Vec<Value>,
    #[serde(default)]
    default_profile: Option<String>
}

/// The first launch after this moved out of the webview: takes over what it kept. An avatar that
/// already has a file keeps it.
pub fn migrate(app: &AppHandle, avatars: Vec<Legacy>) {
    let mut moved = 0;
    for Legacy { avatar_id, groups, profiles, default_profile } in avatars {
        if path(app, &avatar_id).map_or(true, |path| path.exists()) {
            continue;
        }
        let profiles: Vec<Profile> = profiles.into_iter().filter_map(|profile| serde_json::from_value(profile).ok()).collect();
        let default_profile = default_profile.filter(|id| profiles.iter().any(|profile| &profile.id == id));
        let mut file = ControlsFile { groups, profiles, default_profile };
        for control in file.groups.iter_mut().filter_map(|group| group.get_mut("controls")?.as_array_mut()).flatten() {
            resolve::strip(control);
            resolve::upgrade(control);
        }
        match write_file(app, &avatar_id, &file) {
            Ok(()) => moved += 1,
            Err(err) => tracing::error!("Couldn't move the controls of {avatar_id}: {err}")
        }
    }
    tracing::info!("Moved the controls of {moved} avatar(s) out of the webview");
    // The worn avatar's file may be new: read it again.
    setup(app).inner.lock().unwrap().avatar_id = None;
    refresh(app);
}

/// The type of every control saved for any avatar, for "3 controls use OpenShock" when turning
/// an add-on off.
pub fn saved_control_types(app: &AppHandle) -> Vec<String> {
    let Ok(dir) = app.path().app_data_dir() else {
        return Vec::new();
    };
    let files = std::fs::read_dir(dir).into_iter().flatten().flatten();
    files
        .filter_map(|entry| entry.file_name().to_str()?.strip_prefix("controls-")?.strip_suffix(".json").map(|id| id.replacen("local_", "local:", 1)))
        .flat_map(|avatar_id| read_file(app, &avatar_id).groups)
        .flat_map(|group| group.get("controls").and_then(Value::as_array).cloned().unwrap_or_default())
        .filter_map(|control| control.get("type")?.as_str().map(str::to_owned))
        .collect()
}

/// The app as `resolve` sees it right now.
struct AppLive<'a> {
    app: &'a AppHandle,
    avatar_id: Option<String>,
    touched: Vec<crate::toys::Touched>,
    presets: OnceCell<Vec<String>>,
    patterns: OnceCell<Vec<crate::toys::saved::SavedPattern>>
}

impl resolve::Live for AppLive<'_> {
    fn addon_off(&self, control_type: &str) -> Option<String> {
        crate::addons::owner_of_control_type(control_type).filter(|addon| !crate::addons::is_on(self.app, addon.id)).map(|addon| addon.name.to_owned())
    }
    fn openshock_available(&self) -> bool {
        crate::openshock::token_valid(self.app)
    }
    fn hubs_offline(&self, shocker_ids: &[String]) -> bool {
        crate::openshock::hubs_offline(self.app, shocker_ids)
    }
    fn intiface_connected(&self) -> bool {
        crate::intiface::connected(self.app)
    }
    fn has_device(&self, index: u64) -> bool {
        u32::try_from(index).is_ok_and(|index| crate::intiface::has_device(self.app, index))
    }
    fn touched(&self, device_index: u64, actuator_index: u64) -> bool {
        self.touched.iter().any(|touched| u64::from(touched.device_index) == device_index && u64::from(touched.actuator_index) == actuator_index)
    }
    fn preset_exists(&self, id: &str) -> bool {
        self.presets
            .get_or_init(|| {
                let store = self.avatar_id.as_deref().and_then(|avatar| crate::presets::load(self.app, avatar).ok());
                store.map(|store| store.presets.into_iter().map(|preset| preset.id).collect()).unwrap_or_default()
            })
            .iter()
            .any(|preset| preset == id)
    }
    fn bhaptics_connected(&self) -> bool {
        crate::bhaptics::connected(self.app)
    }
    fn owo_connected(&self) -> bool {
        crate::owo::connected(self.app)
    }
    fn saved_pattern(&self, id: &str) -> Option<(String, Option<String>)> {
        let patterns = self.patterns.get_or_init(|| crate::toys::saved::list(self.app));
        patterns.iter().find(|pattern| pattern.id == id).map(|pattern| (pattern.name.clone(), pattern.icon.clone()))
    }
    fn builtin_pattern(&self, id: &str) -> bool {
        crate::toys::patterns::builtin(id).is_some()
    }
}

/// Works the controls out again (the avatar, a profile, a device or an add-on changed) and tells
/// the webview when anything is different. Running toys are brought within tighter limits (see
/// `server::policy::clamp_running`).
pub fn refresh(app: &AppHandle) {
    let (previous, snapshot) = {
        let _one_at_a_time = setup(app).refreshing.lock().unwrap();
        let (avatar_id, file, active) = {
            let mut inner = setup(app).inner.lock().unwrap();
            load_worn(app, &mut inner);
            (inner.avatar_id.clone(), inner.file.clone(), active_profile(&inner))
        };

        // Without the lock: this asks the device modules.
        let live = AppLive { app, avatar_id: avatar_id.clone(), touched: crate::toys::touched(app), presets: OnceCell::new(), patterns: OnceCell::new() };
        let profile = file.profiles.iter().find(|profile| Some(&profile.id) == active.as_ref());
        let snapshot = Snapshot {
            groups: resolve::resolve(&file.groups, profile, &live),
            avatar_id,
            profiles: file.profiles.clone(),
            default_profile: file.default_profile.clone(),
            active_profile: active
        };

        let mut inner = setup(app).inner.lock().unwrap();
        if inner.snapshot == snapshot {
            return;
        }
        (std::mem::replace(&mut inner.snapshot, snapshot.clone()), snapshot)
    };

    let _ = app.emit("setup-controls", &snapshot);
    let same_avatar = previous.avatar_id == snapshot.avatar_id;
    crate::server::policy::clamp_running(app, same_avatar.then_some(previous.groups.as_slice()), &snapshot.groups);
}

/// Runs `f` on the worn avatar's resolved groups without copying them. It holds the lock: copy
/// out what you need and work on it after.
pub fn with_groups<T>(app: &AppHandle, f: impl FnOnce(&[Value]) -> T) -> T {
    f(&setup(app).inner.lock().unwrap().snapshot.groups)
}

fn find_group<'a>(groups: &'a [Value], group_id: &str) -> Option<&'a Value> {
    groups.iter().find(|group| group.get("id").and_then(Value::as_str) == Some(group_id))
}

/// A control of the worn avatar, resolved.
pub fn find_control(app: &AppHandle, group_id: &str, control_id: &str) -> Option<Value> {
    with_groups(app, |groups| {
        find_group(groups, group_id)?
            .get("controls")?
            .as_array()?
            .iter()
            .find(|control| control.get("id").and_then(Value::as_str) == Some(control_id))
            .cloned()
    })
}

/// Whether viewers can't see this group.
pub fn is_group_hidden(app: &AppHandle, group_id: &str) -> bool {
    with_groups(app, |groups| find_group(groups, group_id).and_then(|group| group.get("hidden")).and_then(Value::as_bool).unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avatar_ids() {
        assert!(valid_avatar_id("avtr_b89f691a-d31d-4a84-83fe-5cff212b3761"));
        assert!(valid_avatar_id("local:b89f691a-d31d-4a84-83fe-5cff212b3761"));
        for bad in ["", "avtr_", "avtr_../x", "avtr_a/b", "usr_123", "avtr_a b", "local:..", "local:a:b"] {
            assert!(!valid_avatar_id(bad), "{bad}");
        }
    }

    fn inner(picked: Option<Option<&str>>, default: Option<&str>) -> Inner {
        let profile = |id: &str| Profile { id: id.into(), name: id.into(), locked_controls: BTreeMap::new(), limits: BTreeMap::new(), group_overrides: BTreeMap::new(), created_by: None };
        let mut inner = Inner {
            avatar_id: Some("avtr_a".into()),
            file: ControlsFile { groups: vec![], profiles: vec![profile("p"), profile("d")], default_profile: default.map(Into::into) },
            ..Default::default()
        };
        if let Some(picked) = picked {
            inner.picked.insert("avtr_a".into(), picked.map(Into::into));
        }
        inner
    }

    #[test]
    fn active_profile_rules() {
        // Not picked since the start: the default.
        assert_eq!(active_profile(&inner(None, Some("d"))), Some("d".into()));
        // "No profile" picked wins over the default.
        assert_eq!(active_profile(&inner(Some(None), Some("d"))), None);
        assert_eq!(active_profile(&inner(Some(Some("p")), Some("d"))), Some("p".into()));
        // A picked profile that was deleted falls back to the default.
        assert_eq!(active_profile(&inner(Some(Some("gone")), Some("d"))), Some("d".into()));
        assert_eq!(active_profile(&inner(None, Some("gone"))), None);
    }
}
