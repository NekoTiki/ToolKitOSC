//! Profile, Stop everything, automation on/off and VR message steps, run here so they work with
//! the window closed. Chatbox steps
//! still go to the webview, which has the chatbox settings.
use std::collections::HashMap;
use std::sync::Mutex;

use tauri::AppHandle;

use super::engine::{AppRun, Phase};
use super::model::Action;

/// The profile each running hold replaced, by automation and step, to put back on `stop`.
static HELD: Mutex<Option<HashMap<String, Option<String>>>> = Mutex::new(None);

/// Runs a Profile or Stop everything step. Returns false for any other action.
pub fn run(app: &AppHandle, run: &AppRun) -> bool {
    // Nothing new starts while Stop everything is on; holds may still let go.
    let blocked = super::is_paused(app) && run.phase != Phase::Stop;
    match &run.action {
        Action::StopEverything => {
            if run.phase == Phase::Run && !blocked {
                crate::setup::stop::stop_now(app);
            }
            true
        }
        Action::Profile { profile_id, .. } => {
            if !blocked {
                profile(app, format!("{}:{}", run.automation_id, run.step), profile_id, &run.phase);
            }
            true
        }
        Action::AutomationEnabled { automation_id, enabled } => {
            if run.phase == Phase::Run && !blocked {
                super::set_enabled(app, automation_id, *enabled);
            }
            true
        }
        Action::VrToast { text } => {
            if run.phase == Phase::Run && !blocked {
                crate::overlay::automation_message(app, &super::fill(app, text));
            }
            true
        }
        _ => false
    }
}

fn profile(app: &AppHandle, key: String, profile_id: &Option<String>, phase: &Phase) {
    let snapshot = crate::setup::snapshot(app);
    // A profile that was deleted since is skipped.
    if profile_id.as_ref().is_some_and(|id| !snapshot.profiles.iter().any(|profile| &profile.id == id)) {
        return;
    }
    let current = snapshot.active_profile;
    let target = {
        let mut held = HELD.lock().unwrap();
        let held = held.get_or_insert_with(HashMap::new);
        match phase {
            Phase::Stop => {
                let Some(before) = held.remove(&key) else {
                    return;
                };
                // Only if nothing else switched it in the meantime.
                if current != *profile_id {
                    return;
                }
                before
            }
            Phase::Start => {
                held.entry(key).or_insert(current);
                profile_id.clone()
            }
            _ => profile_id.clone()
        }
    };
    if let Err(err) = crate::setup::switch_profile(app, target) {
        tracing::debug!("Automation couldn't switch the profile: {err}");
    }
}

/// Holds belong to the old avatar's profiles: forget them.
pub fn avatar_changed() {
    HELD.lock().unwrap().take();
}
