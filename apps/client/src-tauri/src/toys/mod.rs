//! Everything that drives toys goes through here, so one place knows each actuator's level. For
//! now the last writer wins, as it always has; avatar haptics will add touch priority here.
mod arbiter;
pub mod patterns;
pub mod saved;
pub mod commands;

pub use arbiter::{set, stop_all, ActuatorRef, Toys};

use tauri::AppHandle;

/// Intiface's device list changed or it reconnected: forget what was sent, so the next value for a
/// toy that came back isn't skipped as a repeat.
pub fn devices_changed(app: &AppHandle) {
    arbiter::forget_sent(app);
}
