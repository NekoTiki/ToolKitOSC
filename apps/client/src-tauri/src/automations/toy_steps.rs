//! Toy, Play pattern and Stop toys steps: toys driven straight through the arbiter, without a
//! control, so they work on every avatar. Automations act as you, so profile limits don't apply;
//! avatar touch still comes first (see `toys::arbiter`), and Stop everything stops them.
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use tauri::async_runtime::JoinHandle;
use tauri::AppHandle;
use tokio::time::{interval, Instant, MissedTickBehavior};

use super::engine::{AppRun, Phase};
use super::model::{Action, ToyMotor, ToyRun};
use crate::toys::{self, patterns, ActuatorRef};

/// A ramp moves this often, like patterns.
const RAMP_TICK: Duration = Duration::from_millis(50);

/// Running pulses and ramps, by automation and step, so a new run replaces the old one.
static TASKS: Mutex<Option<HashMap<String, JoinHandle<()>>>> = Mutex::new(None);

fn replace_task(key: &str, task: Option<JoinHandle<()>>) {
    let mut tasks = TASKS.lock().unwrap();
    let tasks = tasks.get_or_insert_with(HashMap::new);
    if let Some(old) = tasks.remove(key) {
        old.abort();
    }
    if let Some(task) = task {
        tasks.insert(key.to_owned(), task);
    }
}

/// The step's motors, or every motor connected now when it has none.
fn actuators(app: &AppHandle, toys: &[ToyMotor]) -> Vec<ActuatorRef> {
    if toys.is_empty() {
        return crate::intiface::actuators(app);
    }
    toys.iter()
        .map(|toy| ActuatorRef { device_index: toy.device_index, actuator_index: toy.actuator_index, actuator_type: toy.actuator_type.clone() })
        .collect()
}

/// Driving these directly wins over a pattern on them, which stops first or its next tick would
/// land after this level.
fn set(app: &AppHandle, actuators: &[ActuatorRef], level: f64) {
    patterns::release(app, actuators, None);
    toys::set(app, actuators, level);
}

fn pulse(app: &AppHandle, actuators: Vec<ActuatorRef>, ms: u32) -> JoinHandle<()> {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(u64::from(ms))).await;
        if !super::is_paused(&app) {
            set(&app, &actuators, 0.0);
        }
    })
}

fn ramp(app: &AppHandle, actuators: Vec<ActuatorRef>, from: f64, to: f64, ms: u32) -> JoinHandle<()> {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let started = Instant::now();
        let mut ticker = interval(RAMP_TICK);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if super::is_paused(&app) {
                return;
            }
            let done = (started.elapsed().as_secs_f64() * 1000.0 / f64::from(ms.max(1))).min(1.0);
            set(&app, &actuators, from + (to - from) * done);
            if done >= 1.0 {
                return;
            }
        }
    })
}

/// Runs one toy step. Returns false for any other action.
pub fn run(app: &AppHandle, run: &AppRun) -> bool {
    let key = format!("automation:{}:{}", run.automation_id, run.step);
    // Nothing new starts while Stop everything is on, and it already switched every toy off.
    let paused = super::is_paused(app);

    match &run.action {
        Action::Toy { toys, run: toy } => {
            if paused {
                replace_task(&key, None);
                return true;
            }
            let actuators = actuators(app, toys);
            match (&run.phase, toy) {
                (Phase::Stop, _) => {
                    replace_task(&key, None);
                    set(app, &actuators, 0.0);
                }
                (Phase::Follow { value }, ToyRun::Follow { min, max }) => set(app, &actuators, min + f64::from(*value) * (max - min)),
                (Phase::Follow { .. }, _) | (_, ToyRun::Follow { .. }) => {}
                (_, ToyRun::Hold { level }) => set(app, &actuators, *level),
                (_, ToyRun::Pulse { level, ms }) => {
                    set(app, &actuators, *level);
                    replace_task(&key, Some(pulse(app, actuators, *ms)));
                }
                (_, ToyRun::Ramp { from, to, ms }) => replace_task(&key, Some(ramp(app, actuators, *from, *to, *ms)))
            }
        }
        Action::ToyPattern { toys, pattern_id, ms, while_held } => {
            if paused {
                return true;
            }
            let actuators = actuators(app, toys);
            match run.phase {
                Phase::Stop => patterns::off(app, &key, &actuators),
                Phase::Follow { .. } => {}
                Phase::Run | Phase::Start => {
                    let source = patterns::Source { id: pattern_id.clone(), samples: None };
                    let stop_after = (!while_held).then_some(u64::from(*ms));
                    if patterns::play(app, &key, source, actuators, stop_after, None).is_none() {
                        super::note_skip(app, run.automation_id.clone(), "Skipped Play pattern: that pattern was deleted".into());
                    }
                }
            }
        }
        Action::StopToys => {
            if run.phase == Phase::Run && !paused {
                TASKS.lock().unwrap().take().into_iter().flatten().for_each(|(_, task)| task.abort());
                patterns::stop_all(app);
                toys::stop_all(app);
            }
        }
        _ => return false
    }
    true
}
