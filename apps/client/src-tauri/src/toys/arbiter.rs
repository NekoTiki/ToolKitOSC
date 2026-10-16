//! Sends actuator levels to Intiface: one ScalarCmd per device, only for devices it has, and only
//! when a level actually changed since it was last sent.
use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use serde::Deserialize;
use tauri::{AppHandle, Manager};

use crate::intiface::{self, protocol};
use crate::state::AppState;

/// An actuator a control drives, as the webview stores it (see `IntifaceActuatorRef`). Its type
/// has to match what the device reports for that index, or Intiface rejects the whole command.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActuatorRef {
    pub device_index: u32,
    pub actuator_index: u32,
    pub actuator_type: String
}

#[derive(Default)]
pub struct Toys {
    /// Last level sent per (device, actuator).
    sent: Mutex<HashMap<(u32, u32), f64>>
}

fn toys(app: &AppHandle) -> &Toys {
    &app.state::<AppState>().inner().toys
}

pub(super) fn forget_sent(app: &AppHandle) {
    toys(app).sent.lock().unwrap().clear();
}

/// Sets these actuators to `level` (0–1). Nothing happens unless Intiface is connected. A toy that
/// isn't there is skipped: Intiface would refuse the whole command for it.
pub fn set(app: &AppHandle, actuators: &[ActuatorRef], level: f64) {
    if !intiface::connected(app) {
        return;
    }
    let level = level.clamp(0.0, 1.0);

    let mut by_device: BTreeMap<u32, Vec<(u32, f64, String)>> = BTreeMap::new();
    {
        let mut sent = toys(app).sent.lock().unwrap();
        for actuator in actuators {
            if !intiface::has_device(app, actuator.device_index) {
                continue;
            }
            let key = (actuator.device_index, actuator.actuator_index);
            if sent.get(&key) == Some(&level) {
                continue;
            }
            sent.insert(key, level);
            by_device.entry(actuator.device_index).or_default().push((
                actuator.actuator_index,
                level,
                actuator.actuator_type.clone()
            ));
        }
    }

    let messages: Vec<_> = by_device.iter().map(|(device, scalars)| protocol::scalar_cmd(*device, scalars)).collect();
    if !messages.is_empty() {
        intiface::send(app, messages);
    }
}

/// Stops every connected toy, not only the ones a control drives.
pub fn stop_all(app: &AppHandle) {
    toys(app).sent.lock().unwrap().clear();
    intiface::send(app, vec![protocol::message("StopAllDevices", serde_json::json!({}))]);
}
