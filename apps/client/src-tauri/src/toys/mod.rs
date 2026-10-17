//! Everything that drives toys goes through here, so one place knows each actuator's level: the last
//! writer wins, except that avatar touch comes first (see `arbiter`).
mod arbiter;
pub mod patterns;
pub mod saved;
pub mod commands;

pub use arbiter::{set, set_touch, stop_all, touched, ActuatorRef, Priority, Touch, Touched, Toys};

use tauri::AppHandle;

/// Intiface's device list changed or it reconnected: forget what was sent, so the next value for a
/// toy that came back isn't skipped as a repeat.
pub fn devices_changed(app: &AppHandle) {
    arbiter::forget_sent(app);
}
