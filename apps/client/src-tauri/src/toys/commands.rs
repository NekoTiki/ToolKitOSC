//! The toy and pattern commands the webview calls (see `super`).

use tauri::AppHandle;

/// Sets these actuators to `level` (0–1).
#[tauri::command]
pub fn toys_set(app: AppHandle, actuators: Vec<super::ActuatorRef>, level: f64) {
    super::set(&app, &actuators, level);
}

/// Stops every connected toy.
#[tauri::command]
pub fn toys_stop_all(app: AppHandle) {
    super::stop_all(&app);
}

/// Plays a pattern on these actuators for this control, after taking them from any other pattern.
/// Returns the run's number (`pattern-ended` carries it), or nothing for an unknown pattern.
#[tauri::command]
pub fn patterns_play(
    app: AppHandle,
    control_id: String,
    source: super::patterns::Source,
    actuators: Vec<super::ActuatorRef>,
    stop_after_ms: Option<u64>,
    by: Option<String>
) -> Option<u64> {
    super::patterns::play(&app, &control_id, source, actuators, stop_after_ms, by)
}

/// Stops this control's pattern and zeroes its toys.
#[tauri::command]
pub fn patterns_off(app: AppHandle, control_id: String, actuators: Vec<super::ActuatorRef>) {
    super::patterns::off(&app, &control_id, &actuators);
}

/// Stops this control's pattern without touching its toys.
#[tauri::command]
pub fn patterns_stop(app: AppHandle, control_id: String) {
    super::patterns::stop(&app, &control_id);
}

/// Stops every other pattern on these actuators, before something else drives them.
#[tauri::command]
pub fn patterns_release(app: AppHandle, actuators: Vec<super::ActuatorRef>, except: Option<String>) {
    super::patterns::release(&app, &actuators, except.as_deref());
}

#[tauri::command]
pub fn patterns_stop_all(app: AppHandle) {
    super::patterns::stop_all(&app);
}

#[tauri::command]
pub fn patterns_saved_list(app: AppHandle) -> Vec<super::saved::SavedPattern> {
    super::saved::list(&app)
}

/// Adds a recorded pattern, or replaces the one with its id.
#[tauri::command]
pub fn patterns_save(app: AppHandle, pattern: super::saved::SavedPattern) {
    super::saved::save(&app, pattern);
}

#[tauri::command]
pub fn patterns_delete(app: AppHandle, id: String) {
    super::saved::delete(&app, &id);
}

/// Once, from the recordings the webview used to keep in localStorage.
#[tauri::command]
pub fn patterns_migrate(app: AppHandle, patterns: Vec<super::saved::SavedPattern>) {
    super::saved::migrate(&app, patterns);
}
