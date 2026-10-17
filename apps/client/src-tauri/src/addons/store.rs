//! `addons.json` under the app's data dir: which add-ons are on. Each add-on keeps its own settings
//! file; only the switch lives here. Writes are atomic (a tmp file, then a rename).
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const FILE: &str = "addons.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Switch {
    pub enabled: bool
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonsFile {
    #[serde(default)]
    pub addons: BTreeMap<String, Switch>,
    /// You've been through "What do you use?", or were already using an add-on.
    #[serde(default)]
    pub setup_done: bool,
    /// What the first launch with add-ons kept on, until its notice is dismissed.
    #[serde(default)]
    pub kept: Vec<String>
}

impl AddonsFile {
    /// The first launch with add-ons: what you already used stays on, everything else starts off.
    /// Someone who used none of them gets "What do you use?".
    pub fn migrated(kept: &[String]) -> Self {
        Self {
            addons: kept.iter().map(|id| (id.clone(), Switch { enabled: true })).collect(),
            setup_done: !kept.is_empty(),
            kept: kept.to_vec()
        }
    }

    pub fn is_enabled(&self, id: &str) -> bool {
        self.addons.get(id).is_some_and(|switch| switch.enabled)
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) {
        self.addons.entry(id.to_owned()).or_default().enabled = enabled;
    }

    pub fn enabled_ids(&self) -> HashSet<String> {
        self.addons.iter().filter(|(_, switch)| switch.enabled).map(|(id, _)| id.clone()).collect()
    }
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_data_dir().ok()?;
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join(FILE))
}

/// None when there's no file yet (the first launch with add-ons). A file that can't be read counts
/// as empty, everything off, rather than deciding the switches again.
pub fn load(app: &AppHandle) -> Option<AddonsFile> {
    let path = path(app)?;
    let data = std::fs::read_to_string(&path).ok()?;
    Some(serde_json::from_str(&data).unwrap_or_else(|err| {
        tracing::error!("Couldn't read {}: {err}", path.display());
        AddonsFile { setup_done: true, ..AddonsFile::default() }
    }))
}

pub fn save(app: &AppHandle, file: &AddonsFile) {
    let result = (|| -> Result<(), String> {
        let path = path(app).ok_or("no app data folder")?;
        let tmp_path = path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(file).map_err(|e| e.to_string())?;
        std::fs::write(&tmp_path, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())
    })();

    if let Err(err) = result {
        tracing::error!("Couldn't save {FILE}: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_keeps_what_was_in_use() {
        let file = AddonsFile::migrated(&["openshock".into(), "boards".into()]);
        assert!(file.is_enabled("openshock"));
        assert!(file.is_enabled("boards"));
        assert!(!file.is_enabled("intiface"));
        assert!(file.setup_done);
        assert_eq!(file.kept, ["openshock", "boards"]);
    }

    #[test]
    fn migration_with_nothing_in_use_asks_first() {
        let file = AddonsFile::migrated(&[]);
        assert!(file.enabled_ids().is_empty());
        assert!(!file.setup_done);
    }

    #[test]
    fn reads_an_older_or_partial_file() {
        let file: AddonsFile = serde_json::from_str(r#"{ "addons": { "intiface": { "enabled": true } } }"#).unwrap();
        assert!(file.is_enabled("intiface"));
        assert!(!file.setup_done);
        assert!(file.kept.is_empty());
    }
}
