//! Automations: *when* something happens, *do* something (see `model`). For now the only trigger
//! is an avatar parameter changing and the only action an ESP32 board output; both are tagged
//! unions so app-specific kinds can be added later.
//!
//! The list is global. An avatar-parameter trigger carries its avatar, so it only runs while that
//! avatar is worn. Only live changes count: values read when an avatar loads never reach here
//! (see `osc::udp::handle_message`).
mod engine;
pub mod model;
mod store;

use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use engine::{Effect, State};
use model::{Action, Automation, OutputAction};

use crate::boards::{self, protocol::OutputMessage};
use crate::state::AppState;
use crate::types::{OscArg, OscMessage};

#[derive(Default)]
pub struct Automations {
    state: Mutex<State>
}

fn automations(app: &AppHandle) -> &Automations {
    &app.state::<AppState>().inner().automations
}

pub fn start(app: AppHandle) {
    let list = store::load(&app);
    tracing::info!("{} automation(s)", list.len());
    automations(&app).state.lock().unwrap().automations = list;

    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(100));
        loop {
            ticker.tick().await;
            let effects = automations(&app).state.lock().unwrap().tick();
            apply(&app, effects);
        }
    });
}

fn apply(app: &AppHandle, effects: Vec<Effect>) {
    for effect in effects {
        match effect {
            Effect::Send(board_id, message) => {
                if !boards::send_output(app, &board_id, message.clone()) {
                    tracing::debug!("ESP32 board {board_id} offline, dropped {message:?}");
                }
            }
            Effect::Activity(activity) => {
                let _ = app.emit("automation-activity", activity);
            }
        }
    }
}

pub fn list(app: &AppHandle) -> Vec<Automation> {
    automations(app).state.lock().unwrap().automations.clone()
}

fn changed(app: &AppHandle) {
    let list = list(app);
    store::save(app, &list);
    let _ = app.emit("automations-changed", list);
}

/// Creates or replaces an automation. Returns a message to show when it can't be saved.
pub fn save(app: &AppHandle, automation: Automation) -> Result<(), String> {
    model::validate(&automation)?;

    let Action::BoardOutput { board_id, pin, output } = &automation.then;
    let info = boards::output_info(app, board_id, *pin).ok_or("Pick an output saved on one of your boards.")?;
    if matches!(output, OutputAction::Follow { .. }) && !info.pwm {
        return Err(format!("{} can't dim, so it can't follow a value.", info.label));
    }
    if let OutputAction::Pulse { on_ms, .. } = output {
        if *on_ms > info.max_on_ms {
            return Err(format!(
                "The pulse is longer than {}'s max on-time ({} ms). Shorten it, or raise the max on-time.",
                info.label, info.max_on_ms
            ));
        }
    }

    let effects = {
        let mut state = automations(app).state.lock().unwrap();
        // Whatever the old version was doing stops: its trigger or output may have changed.
        let effects = state.release(&automation.id);
        match state.automations.iter_mut().find(|a| a.id == automation.id) {
            Some(existing) => *existing = automation,
            None => state.automations.push(automation)
        }
        effects
    };
    apply(app, effects);
    changed(app);
    Ok(())
}

pub fn delete(app: &AppHandle, id: &str) {
    let effects = {
        let mut state = automations(app).state.lock().unwrap();
        state.automations.retain(|a| a.id != id);
        state.release(id)
    };
    apply(app, effects);
    changed(app);
}

pub fn set_enabled(app: &AppHandle, id: &str, enabled: bool) {
    let effects = {
        let mut state = automations(app).state.lock().unwrap();
        let Some(automation) = state.automations.iter_mut().find(|a| a.id == id) else {
            return;
        };
        automation.enabled = enabled;
        if enabled {
            Vec::new()
        } else {
            state.release(id)
        }
    };
    apply(app, effects);
    changed(app);
}

/// Runs the action once, without its trigger, to check the wiring.
pub fn test(app: &AppHandle, id: &str) -> Result<(), String> {
    let automation = {
        let state = automations(app).state.lock().unwrap();
        if state.paused {
            return Err("Stop everything is on.".into());
        }
        state.automations.iter().find(|a| a.id == id).cloned().ok_or("That automation doesn't exist any more.")?
    };

    let Action::BoardOutput { board_id, pin, output } = &automation.then;
    let info = boards::output_info(app, board_id, *pin).ok_or("Its output isn't saved on the board any more.")?;
    let short = engine::TEST_MS.min(info.max_on_ms);
    let message = match output {
        OutputAction::Pulse { on_ms, off_ms, count } => OutputMessage::Pulse { pin: *pin, on_ms: *on_ms, off_ms: *off_ms, count: *count },
        OutputAction::Hold | OutputAction::Toggle => OutputMessage::Pulse { pin: *pin, on_ms: short, off_ms: 0, count: 1 },
        OutputAction::Follow { max, .. } => OutputMessage::Pwm { pin: *pin, duty: *max, ms: short },
        OutputAction::Off => OutputMessage::Set { pin: *pin, on: false, ms: 0 }
    };

    if boards::send_output(app, board_id, message) {
        Ok(())
    } else {
        Err("The board is offline.".into())
    }
}

pub fn set_paused(app: &AppHandle, paused: bool) {
    let effects = automations(app).state.lock().unwrap().set_paused(paused);
    apply(app, effects);
}

fn number(message: &OscMessage) -> Option<f64> {
    match message.args.first()? {
        OscArg::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        OscArg::Number(n) => Some(*n),
        OscArg::Str(_) => None
    }
}

/// A live change of an avatar parameter.
pub fn on_parameter(app: &AppHandle, message: &OscMessage, previous: Option<&OscMessage>) {
    let Some(current) = number(message) else {
        return;
    };
    let Some(avatar_id) = app.state::<AppState>().avatar_details.lock().unwrap().as_ref().map(|d| d.id.clone()) else {
        return;
    };

    let max_on = |board_id: &str, pin: u8| boards::output_info(app, board_id, pin).map(|o| o.max_on_ms);
    let effects = automations(app).state.lock().unwrap().on_parameter(
        &avatar_id,
        &message.address,
        previous.and_then(number),
        current,
        &max_on
    );
    apply(app, effects);
}

/// The avatar changed: everything the old one was holding lets go.
pub fn avatar_changed(app: &AppHandle) {
    let effects = automations(app).state.lock().unwrap().release_all();
    apply(app, effects);
}
