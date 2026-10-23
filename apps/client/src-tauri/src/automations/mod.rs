//! Automations: *when* something happens, *do* something (see `model`). For now the only trigger
//! is an avatar parameter changing; an automation does one or more things. ESP32 outputs and the
//! avatar's own parameters are driven from here; controls, profiles, the chatbox and Stop
//! everything live in the frontend, which gets an `automation-run` event for each (see
//! `useAutomationRunner.ts`).
//!
//! The list is global. An avatar-parameter trigger carries its avatar, so it only runs while that
//! avatar is worn. Only live changes count: values read when an avatar loads never reach here
//! (see `osc::udp::handle_message`).
mod app_steps;
mod control_runs;
mod engine;
mod toy_steps;
pub mod model;
mod store;
pub mod commands;

use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use engine::{AppRun, Effect, Env, Phase, State};
use model::{Action, Automation, OutputAction, ParameterAction, ValueType};

use crate::boards::{self, protocol::OutputMessage};
use crate::state::AppState;
use crate::types::{OscArg, OscCommand, OscMessage};

#[derive(Default)]
pub struct Automations {
    state: Mutex<State>
}

fn automations(app: &AppHandle) -> &Automations {
    &app.state::<AppState>().inner().automations
}

struct AppEnv<'a>(&'a AppHandle);

impl Env for AppEnv<'_> {
    fn max_on_ms(&self, board_id: &str, pin: u8) -> Option<u32> {
        boards::output_info(self.0, board_id, pin).map(|o| o.max_on_ms)
    }

    fn value(&self, address: &str) -> Option<f64> {
        self.0.state::<AppState>().messages.lock().unwrap().get(address).and_then(number)
    }

    fn live(&self, action: &Action) -> bool {
        addon_off(self.0, action).is_none()
    }

    fn now(&self) -> model::Now {
        use chrono::{Datelike, Timelike};
        let now = chrono::Local::now();
        model::Now {
            weekday: now.weekday().num_days_from_monday() as u8,
            minute: (now.hour() * 60 + now.minute()) as u16,
            roll: rand::random::<f64>()
        }
    }
}

/// The name of the add-on this step needs, when it's off.
fn addon_off(app: &AppHandle, action: &Action) -> Option<&'static str> {
    let owner = match action {
        Action::BoardOutput { .. } => crate::addons::owner_of_automation_kind("board-output"),
        Action::LeashEnabled { .. } => crate::addons::owner_of_automation_kind("leash-enabled"),
        Action::BhapticsPulse { .. } => crate::addons::owner_of_automation_kind("bhaptics-pulse"),
        Action::BhapticsEffect { .. } => crate::addons::owner_of_automation_kind("bhaptics-effect"),
        Action::OwoSensation { .. } => crate::addons::owner_of_automation_kind("owo-sensation"),
        Action::Control { control_type, .. } => crate::addons::owner_of_control_type(control_type),
        Action::Toy { .. } => crate::addons::owner_of_automation_kind("toy"),
        Action::ToyPattern { .. } => crate::addons::owner_of_automation_kind("toy-pattern"),
        Action::StopToys => crate::addons::owner_of_automation_kind("stop-toys"),
        Action::VrToast { .. } => crate::addons::owner_of_automation_kind("vr-toast"),
        _ => None
    };
    owner.filter(|owner| !crate::addons::is_on(app, owner.id)).map(|owner| owner.name)
}

/// Automations that already said they skipped a step, so each says it once per session.
static SKIP_NOTED: Mutex<Option<HashSet<String>>> = Mutex::new(None);

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Skipped {
    automation_id: String,
    message: String
}

/// The last viewer command that went through: who, which control, and its value.
// shortcut: the latest viewer command, not necessarily the one that started this automation; pass
// it along with the event if two viewers ever race.
static LAST_VIEWER: Mutex<Option<(String, String, String)>> = Mutex::new(None);

pub fn note_viewer(who: &str, control: &str, value: Option<&serde_json::Value>) {
    let value = match value {
        Some(serde_json::Value::Bool(on)) => if *on { "on" } else { "off" }.to_owned(),
        Some(serde_json::Value::Number(n)) => match n.as_f64() {
            Some(n) if n.fract() != 0.0 && (0.0..=1.0).contains(&n) => format!("{}%", (n * 100.0).round()),
            _ => n.to_string()
        },
        Some(serde_json::Value::String(text)) => text.clone(),
        _ => String::new()
    };
    *LAST_VIEWER.lock().unwrap() = Some((who.to_owned(), control.to_owned(), value));
}

/// Chatbox and VR message text with {viewer}, {control}, {value} and {avatar} filled in; the
/// ones that don't apply stay empty.
fn fill(app: &AppHandle, text: &str) -> String {
    if !model::PLACEHOLDERS.iter().any(|p| text.contains(p)) {
        return text.to_owned();
    }
    let (viewer, control, value) = LAST_VIEWER.lock().unwrap().clone().unwrap_or_default();
    let avatar = app.state::<AppState>().avatar_details.lock().unwrap().as_ref().map(|d| d.name.clone()).unwrap_or_default();
    let filled = text.replace("{viewer}", &viewer).replace("{control}", &control).replace("{value}", &value).replace("{avatar}", &avatar);
    filled.chars().take(model::CHATBOX_MAX).collect()
}

/// Says once per session that an automation skipped a step, and why.
fn note_skip(app: &AppHandle, automation_id: String, message: String) {
    if !SKIP_NOTED.lock().unwrap().get_or_insert_with(HashSet::new).insert(automation_id.clone()) {
        return;
    }
    tracing::info!("Automation {automation_id}: {message}");
    let _ = app.emit("automation-skipped", Skipped { automation_id, message });
}

pub fn start(app: AppHandle) {
    let list = store::load(&app);
    tracing::info!("{} automation(s)", list.len());
    automations(&app).state.lock().unwrap().automations = list;

    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(100));
        loop {
            ticker.tick().await;
            let effects = automations(&app).state.lock().unwrap().tick(&AppEnv(&app));
            apply(&app, effects);
        }
    });
}

fn parameter_command(address: &str, value_type: ValueType, value: f64) -> OscCommand {
    let arg = match value_type {
        ValueType::Bool => OscArg::Bool(value >= 0.5),
        ValueType::Int => OscArg::Number(value.round()),
        ValueType::Float => OscArg::Number(value)
    };
    OscCommand { address: address.to_string(), args: vec![arg] }
}

fn apply(app: &AppHandle, effects: Vec<Effect>) {
    for effect in effects {
        match effect {
            // The add-on may have been turned off while a hold was being renewed.
            Effect::Send(_, _) if !crate::addons::is_on(app, "boards") => {}
            Effect::Send(board_id, message) => {
                if !boards::send_output(app, &board_id, message.clone()) {
                    tracing::debug!("ESP32 board {board_id} offline, dropped {message:?}");
                }
            }
            // The add-on may have been turned off since.
            Effect::Leash(_) if !crate::addons::is_on(app, "leash") => {}
            Effect::Leash(enabled) => crate::leash::set_enabled(app, enabled),
            Effect::Bhaptics(_) if !crate::addons::is_on(app, "bhaptics") => {}
            Effect::Bhaptics(action) => crate::bhaptics::run_step(app, &action),
            Effect::Owo(_) if !crate::addons::is_on(app, "owo") => {}
            Effect::Owo(action) => crate::owo::run_step(app, &action),
            Effect::Osc { address, value } => {
                if let Err(err) = crate::osc::send(&app.state::<AppState>(), &OscCommand { address: address.clone(), args: vec![value] }) {
                    tracing::debug!("Couldn't send {address}: {err}");
                }
            }
            // The chatbox posts from the webview, with what started it filled in.
            Effect::App(AppRun { action: Action::Chatbox { text }, automation_id, step, phase }) => {
                let action = Action::Chatbox { text: fill(app, &text) };
                let _ = app.emit("automation-run", AppRun { automation_id, step, action, phase });
            }
            Effect::Parameter { address, value_type, value } => {
                if let Err(err) = crate::osc::send(&app.state::<AppState>(), &parameter_command(&address, value_type, value)) {
                    tracing::debug!("Couldn't set {address}: {err}");
                }
            }
            Effect::App(run) => {
                // Control, toy, profile and Stop everything steps run here; the chatbox in the webview.
                if !control_runs::run(app, &run) && !toy_steps::run(app, &run) && !app_steps::run(app, &run) {
                    let _ = app.emit("automation-run", run);
                }
            }
            Effect::Activity(activity) => {
                let _ = app.emit("automation-activity", activity);
            }
            Effect::Ran(id) => {
                if let Some(automation) = list(app).into_iter().find(|a| a.id == id) {
                    crate::overlay::automation_ran(app, &automation.name);
                }
                let _ = app.emit("automation-ran", id);
            }
            Effect::Skipped { automation_id, action } => {
                let Some(addon) = addon_off(app, &action) else {
                    continue;
                };
                let step = match action {
                    Action::BoardOutput { .. } => "ESP32 output",
                    Action::LeashEnabled { .. } => "Leash on or off",
                    Action::BhapticsPulse { .. } => "bHaptics pulse",
                    Action::BhapticsEffect { .. } => "bHaptics effect",
                    Action::OwoSensation { .. } => "OWO sensation",
                    Action::VrToast { .. } => "VR message",
                    Action::Toy { .. } => "Toy",
                    Action::ToyPattern { .. } => "Play pattern",
                    Action::StopToys => "Stop toys",
                    _ => "Control"
                };
                note_skip(app, automation_id, format!("Skipped {step}: {addon} is off"));
            }
            Effect::Chatbox { board_id, pin, note } => {
                if let Some(info) = boards::output_info(app, &board_id, pin) {
                    let _ = app.emit("automation-chatbox", model::chatbox_text(&info.label, note));
                }
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

    for step in &automation.then {
        let Action::BoardOutput { board_id, pin, output, .. } = &step.action else {
            continue;
        };
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
    }

    let effects = {
        let mut state = automations(app).state.lock().unwrap();
        // Whatever the old version was doing stops: its trigger or steps may have changed.
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

/// What a Test of one step sends, and what it sends `TEST_MS` later to undo it (holds and
/// follows of a parameter).
fn test_step(app: &AppHandle, id: &str, index: usize, action: &Action) -> Result<(Effect, Option<Effect>), String> {
    if let Some(addon) = addon_off(app, action) {
        return Err(format!("{addon} is off."));
    }
    let run = |phase: Phase| Effect::App(AppRun { automation_id: id.to_string(), step: index, action: action.clone(), phase });
    match action {
        action if action.in_app() => Ok(match action.runs() {
            model::Runs::Moment => (run(Phase::Run), None),
            model::Runs::State => (run(Phase::Start), Some(run(Phase::Stop))),
            model::Runs::Follow => (run(Phase::Follow { value: 1.0 }), Some(run(Phase::Follow { value: 0.0 })))
        }),
        Action::BoardOutput { board_id, pin, output, .. } => {
            let info = boards::output_info(app, board_id, *pin).ok_or("Its output isn't saved on the board any more.")?;
            let short = engine::TEST_MS.min(info.max_on_ms);
            let message = match output {
                OutputAction::Pulse { on_ms, off_ms, count } => OutputMessage::Pulse { pin: *pin, on_ms: *on_ms, off_ms: *off_ms, count: *count },
                OutputAction::Hold | OutputAction::Toggle => OutputMessage::Pulse { pin: *pin, on_ms: short, off_ms: 0, count: 1 },
                OutputAction::Follow { max, .. } => OutputMessage::Pwm { pin: *pin, duty: *max, ms: short },
                OutputAction::Off => OutputMessage::Set { pin: *pin, on: false, ms: 0 }
            };
            if !boards::is_online(app, board_id) {
                return Err("The board is offline.".into());
            }
            Ok((Effect::Send(board_id.clone(), message), None))
        }
        Action::AvatarParameter { parameter, value_type, set } => {
            let current = AppEnv(app).value(parameter).unwrap_or(0.0);
            let to = |value: f64| Effect::Parameter { address: parameter.clone(), value_type: *value_type, value };
            Ok(match set {
                ParameterAction::Set { value } => (to(*value), None),
                ParameterAction::Toggle => (to(if current >= 0.5 { 0.0 } else { 1.0 }), None),
                ParameterAction::Hold { value } => (to(*value), Some(to(current))),
                ParameterAction::Follow { max, .. } => (to(*max), Some(to(current))),
                ParameterAction::Add { amount } => (to(value_type.clamp(current + amount)), None)
            })
        }
        Action::OscMessage { address, value } => Ok((Effect::Osc { address: address.clone(), value: value.clone() }, None)),
        // A test tries one of them, at random like a real run.
        Action::OneOf { options } => {
            let pick = options.get(rand::random_range(0..options.len().max(1))).ok_or("It has nothing to pick from.")?;
            test_step(app, id, index, pick)
        }
        Action::LeashEnabled { enabled } => Ok((Effect::Leash(*enabled), None)),
        Action::BhapticsPulse { .. } | Action::BhapticsEffect { .. } => Ok((Effect::Bhaptics(action.clone()), None)),
        Action::OwoSensation { .. } => Ok((Effect::Owo(action.clone()), None)),
        // App actions are handled above.
        Action::Control { .. }
        | Action::Profile { .. }
        | Action::Chatbox { .. }
        | Action::StopEverything
        | Action::Toy { .. }
        | Action::ToyPattern { .. }
        | Action::StopToys
        | Action::AutomationEnabled { .. }
        | Action::VrToast { .. } => Ok((run(Phase::Run), None))
    }
}

/// Runs the steps once, with their delays but without the trigger, to check the wiring.
pub fn test(app: &AppHandle, id: &str) -> Result<(), String> {
    let automation = {
        let state = automations(app).state.lock().unwrap();
        if state.paused {
            return Err("Stop everything is on.".into());
        }
        state.automations.iter().find(|a| a.id == id).cloned().ok_or("That automation doesn't exist any more.")?
    };

    // Everything is checked before anything is sent.
    let mut steps = Vec::new();
    for (index, step) in automation.then.iter().enumerate() {
        steps.push((step.delay_ms, test_step(app, id, index, &step.action)?));
    }

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let started = tokio::time::Instant::now();
        steps.sort_by_key(|(delay, _)| *delay);
        let mut undo = Vec::new();
        for (delay, (effect, after)) in steps {
            tokio::time::sleep_until(started + Duration::from_millis(u64::from(delay))).await;
            if automations(&app).state.lock().unwrap().paused {
                return;
            }
            apply(&app, vec![effect]);
            if let Some(after) = after {
                undo.push((delay + engine::TEST_MS, after));
            }
        }
        for (at, effect) in undo {
            tokio::time::sleep_until(started + Duration::from_millis(u64::from(at))).await;
            apply(&app, vec![effect]);
        }
    });
    Ok(())
}

/// Stop everything is on (see `setup::stop`, which keeps it).
pub fn is_paused(app: &AppHandle) -> bool {
    automations(app).state.lock().unwrap().paused
}

pub fn set_paused(app: &AppHandle, paused: bool) {
    let effects = automations(app).state.lock().unwrap().set_paused(paused);
    apply(app, effects);
    crate::overlay::paused(app, paused);
}

pub fn leash_event(app: &AppHandle, event: model::LeashEvent) {
    let effects = automations(app).state.lock().unwrap().on_leash(event, &AppEnv(app));
    apply(app, effects);
}

pub fn event(app: &AppHandle, event: &model::AppEvent) {
    let effects = automations(app).state.lock().unwrap().on_event(event, &AppEnv(app));
    apply(app, effects);
}

/// How many viewers are on, from the webview. The profile comes from `set_profile`.
pub fn set_context(app: &AppHandle, context: model::Context) {
    let mut state = automations(app).state.lock().unwrap();
    state.context = model::Context { profile_id: state.context.profile_id.take(), avatar_id: state.context.avatar_id.take(), ..context };
}

/// The worn avatar's active profile, for "only if".
pub fn set_profile(app: &AppHandle, profile_id: Option<String>) {
    automations(app).state.lock().unwrap().context.profile_id = profile_id;
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

    let effects = automations(app).state.lock().unwrap().on_parameter(
        &avatar_id,
        &message.address,
        previous.and_then(number),
        current,
        &AppEnv(app)
    );
    apply(app, effects);
}

/// The new avatar is loaded (details and saved values): its "avatar changed" automations run.
pub fn avatar_worn(app: &AppHandle, avatar_id: &str) {
    let effects = {
        let mut state = automations(app).state.lock().unwrap();
        state.context.avatar_id = Some(avatar_id.to_owned());
        state.on_avatar_worn(avatar_id, &AppEnv(app))
    };
    apply(app, effects);
}

/// The avatar changed: everything the old one was holding lets go.
pub fn avatar_changed(app: &AppHandle) {
    // Control holds and pulses belonged to the old avatar's controls: forgotten, not put back.
    control_runs::forget();
    app_steps::avatar_changed();
    let effects = automations(app).state.lock().unwrap().release_all();
    apply(app, effects);
}
