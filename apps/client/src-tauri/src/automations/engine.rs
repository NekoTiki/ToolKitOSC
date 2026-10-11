//! Runs automations. Parameter changes come in from `osc::udp::handle_message`; what to send is
//! worked out under the lock and sent after it, so a slow board never holds anything up.
//!
//! Boards keep time themselves (see the firmware's PROTOCOL.md): a pulse is one message. Holds and
//! follows are leases instead: a claim on the output that `tick` renews every 500 ms, so the
//! output goes off by itself within 1.5 s if the app freezes or quits.
//!
//! An automation runs its steps together, each after its own delay. Delayed steps wait in
//! `pending` until `tick` (every 100 ms) runs them.
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use serde::Serialize;

use super::model::{Action, Automation, Edge, OutputAction, ParameterAction, Retrigger, Runs, Trigger, ValueType};
use crate::boards::protocol::OutputMessage;

/// How long each hold/follow message keeps the output on, and how often it's renewed.
pub const LEASE_MS: u32 = 1500;
const RENEW_EVERY: Duration = Duration::from_millis(500);
/// Follow sends at most this often; the latest value goes out on the next tick.
const FOLLOW_MIN_GAP: Duration = Duration::from_millis(50);
/// A Test of an open-ended action switches the output on for this long.
pub const TEST_MS: u32 = 1000;

/// What the engine needs to know about the rest of the app, passed in so it stays testable.
pub trait Env {
    /// An output's max on-time, for toggles.
    fn max_on_ms(&self, board_id: &str, pin: u8) -> Option<u32>;
    /// A parameter's value as last read from VRChat, e.g. when the avatar loaded.
    fn value(&self, address: &str) -> Option<f64>;
}

#[derive(Default)]
struct Runtime {
    last_run: Option<Instant>,
    /// Until the last run's pulses and delayed steps are done.
    busy_until: Option<Instant>
}

enum Hold {
    /// On (or dimmed, for a follow) while claimed, renewed by `tick`.
    Pin { board_id: String, pin: u8, follow: bool, duty: f32, sent_at: Option<Instant>, sent_duty: Option<f32> },
    /// Set while claimed; `restore` has what goes back afterwards.
    Parameter { address: String }
}

struct Claim {
    automation_id: String,
    step: usize,
    hold: Hold
}

fn pin_message(pin: u8, follow: bool, duty: f32) -> OutputMessage {
    if follow {
        OutputMessage::Pwm { pin, duty, ms: LEASE_MS }
    } else {
        OutputMessage::Set { pin, on: true, ms: LEASE_MS }
    }
}

/// Things the board times itself, so a release doesn't cut them short.
#[derive(Default)]
struct PinState {
    pulse_until: Option<Instant>,
    toggled_until: Option<Instant>,
    toggled_by: Option<String>
}

/// A step waiting for its delay.
struct Pending {
    due: Instant,
    automation_id: String,
    step: usize
}

/// What the UI shows as "Running".
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: String,
    pub running: bool
}

pub enum Effect {
    Send(String, OutputMessage),
    /// Sets one of the worn avatar's parameters.
    Parameter { address: String, value_type: ValueType, value: f64 },
    Activity(Activity)
}

#[derive(Default)]
pub struct State {
    pub automations: Vec<Automation>,
    pub paused: bool,
    runtime: HashMap<String, Runtime>,
    claims: Vec<Claim>,
    pins: HashMap<(String, u8), PinState>,
    pending: Vec<Pending>,
    /// Parameters a hold changed: what to set back once nothing holds them.
    restore: HashMap<String, (ValueType, f64)>,
    /// The latest value of each parameter, as seen live or set here.
    values: HashMap<String, f64>,
    /// Ids last reported as running.
    reported: HashSet<String>
}

fn is_future(at: Option<Instant>, now: Instant) -> bool {
    at.is_some_and(|at| at > now)
}

fn after(now: Instant, ms: u32) -> Instant {
    now + Duration::from_millis(u64::from(ms))
}

impl State {
    /// A live change of an avatar parameter.
    pub fn on_parameter(&mut self, avatar_id: &str, address: &str, previous: Option<f64>, current: f64, env: &dyn Env) -> Vec<Effect> {
        let mut effects = Vec::new();
        self.values.insert(address.to_string(), current);
        if self.paused {
            return effects;
        }

        let now = Instant::now();
        let matching: Vec<Automation> = self
            .automations
            .iter()
            .filter(|a| a.enabled)
            .filter(|a| {
                let Trigger::AvatarParameter { avatar_id: id, parameter, .. } = &a.when;
                id == avatar_id && parameter == address
            })
            .cloned()
            .collect();

        for automation in matching {
            let Trigger::AvatarParameter { condition, .. } = &automation.when;
            match condition.evaluate(previous, current) {
                Edge::Fire => self.fire(&automation, now, env, &mut effects),
                Edge::Enter => self.start(&automation, Runs::State, now, env, &mut effects),
                Edge::Exit => {
                    self.pending.retain(|p| p.automation_id != automation.id);
                    self.unclaim(&automation.id, None, now, &mut effects);
                }
                Edge::Follow(value) => self.follow(&automation, value, now, &mut effects),
                Edge::None => {}
            }
        }

        self.report(now, &mut effects);
        effects
    }

    fn fire(&mut self, automation: &Automation, now: Instant, env: &dyn Env, effects: &mut Vec<Effect>) {
        if let Some(runtime) = self.runtime.get(&automation.id) {
            if automation.cooldown_ms > 0 && runtime.last_run.is_some_and(|at| now < after(at, automation.cooldown_ms)) {
                return;
            }
            if automation.retrigger == Retrigger::Ignore && is_future(runtime.busy_until, now) {
                return;
            }
        }

        // Start over: what the last run still had waiting is dropped.
        self.pending.retain(|p| p.automation_id != automation.id);
        let busy = automation
            .then
            .iter()
            .map(|step| {
                let length = match &step.action {
                    Action::BoardOutput { output, .. } => output.busy_ms().unwrap_or(0),
                    Action::AvatarParameter { .. } => 0
                };
                step.delay_ms.saturating_add(length)
            })
            .max()
            .unwrap_or(0);
        let runtime = self.runtime.entry(automation.id.clone()).or_default();
        runtime.last_run = Some(now);
        runtime.busy_until = (busy > 0).then(|| after(now, busy));

        self.start(automation, Runs::Moment, now, env, effects);
    }

    /// Runs (or schedules) the steps that run on this kind of edge.
    fn start(&mut self, automation: &Automation, runs: Runs, now: Instant, env: &dyn Env, effects: &mut Vec<Effect>) {
        for (index, step) in automation.then.iter().enumerate() {
            if step.action.runs() != runs {
                continue;
            }
            if step.delay_ms == 0 {
                self.run_step(automation, index, now, env, effects);
            } else {
                self.pending.push(Pending { due: after(now, step.delay_ms), automation_id: automation.id.clone(), step: index });
            }
        }
    }

    fn run_step(&mut self, automation: &Automation, index: usize, now: Instant, env: &dyn Env, effects: &mut Vec<Effect>) {
        let Some(step) = automation.then.get(index) else {
            return;
        };
        match step.action.runs() {
            Runs::Moment => self.moment(automation, &step.action, now, env, effects),
            Runs::State => self.claim(automation, index, &step.action, 1.0, now, env, effects),
            // Follows run on every change, never from a delay (validation rules that out).
            Runs::Follow => {}
        }
    }

    fn moment(&mut self, automation: &Automation, action: &Action, now: Instant, env: &dyn Env, effects: &mut Vec<Effect>) {
        match action {
            Action::BoardOutput { board_id, pin, output } => {
                let key = (board_id.clone(), *pin);
                match output {
                    OutputAction::Pulse { on_ms, off_ms, count } => {
                        let pin_state = self.pins.entry(key).or_default();
                        pin_state.pulse_until = Some(after(now, output.busy_ms().unwrap_or(*on_ms))).max(pin_state.pulse_until);
                        effects.push(Effect::Send(
                            board_id.clone(),
                            OutputMessage::Pulse { pin: *pin, on_ms: *on_ms, off_ms: *off_ms, count: *count }
                        ));
                    }
                    OutputAction::Toggle => {
                        let pin_state = self.pins.entry(key).or_default();
                        if is_future(pin_state.toggled_until, now) {
                            pin_state.toggled_until = None;
                            pin_state.toggled_by = None;
                            effects.push(Effect::Send(board_id.clone(), OutputMessage::Set { pin: *pin, on: false, ms: 0 }));
                        } else {
                            // On until toggled again, never past the output's max on-time. A
                            // pulse, so reaching that limit doesn't trip the output.
                            let ms = env.max_on_ms(board_id, *pin).unwrap_or(10_000);
                            pin_state.toggled_until = Some(after(now, ms));
                            pin_state.toggled_by = Some(automation.id.clone());
                            effects.push(Effect::Send(
                                board_id.clone(),
                                OutputMessage::Pulse { pin: *pin, on_ms: ms, off_ms: 0, count: 1 }
                            ));
                        }
                    }
                    OutputAction::Off => {
                        // Everything else driving this output lets go too, or the next renewal
                        // would switch it straight back on.
                        self.claims.retain(|c| !matches!(&c.hold, Hold::Pin { board_id: b, pin: p, .. } if b == board_id && p == pin));
                        self.pins.remove(&key);
                        effects.push(Effect::Send(board_id.clone(), OutputMessage::Set { pin: *pin, on: false, ms: 0 }));
                    }
                    // State and follow actions don't run on moments (validation rules that out).
                    OutputAction::Hold | OutputAction::Follow { .. } => {}
                }
            }
            Action::AvatarParameter { parameter, value_type, set } => match set {
                ParameterAction::Set { value } => self.set_parameter(parameter, *value_type, *value, effects),
                ParameterAction::Toggle => {
                    let on = self.value(parameter, env).is_some_and(|v| v >= 0.5);
                    self.set_parameter(parameter, *value_type, if on { 0.0 } else { 1.0 }, effects);
                }
                ParameterAction::Hold { .. } | ParameterAction::Follow { .. } => {}
            }
        }
    }

    fn value(&self, address: &str, env: &dyn Env) -> Option<f64> {
        self.values.get(address).copied().or_else(|| env.value(address))
    }

    fn set_parameter(&mut self, address: &str, value_type: ValueType, value: f64, effects: &mut Vec<Effect>) {
        self.values.insert(address.to_string(), value);
        effects.push(Effect::Parameter { address: address.to_string(), value_type, value });
    }

    fn follow(&mut self, automation: &Automation, value: f32, now: Instant, effects: &mut Vec<Effect>) {
        for (index, step) in automation.then.iter().enumerate() {
            match &step.action {
                Action::BoardOutput { output: OutputAction::Follow { min, max }, .. } => {
                    let duty = min + value * (max - min);
                    if duty <= 0.001 {
                        self.unclaim(&automation.id, Some(index), now, effects);
                    } else {
                        self.claim_pin(automation, index, &step.action, true, duty, now, effects);
                    }
                }
                Action::AvatarParameter { parameter, value_type, set: ParameterAction::Follow { min, max } } => {
                    let target = min + f64::from(value) * (max - min);
                    self.set_parameter(parameter, *value_type, target, effects);
                }
                _ => {}
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn claim(&mut self, automation: &Automation, index: usize, action: &Action, duty: f32, now: Instant, env: &dyn Env, effects: &mut Vec<Effect>) {
        match action {
            Action::BoardOutput { .. } => self.claim_pin(automation, index, action, false, duty, now, effects),
            Action::AvatarParameter { parameter, value_type, set: ParameterAction::Hold { value } } => {
                if self.claims.iter().any(|c| c.automation_id == automation.id && c.step == index) {
                    return;
                }
                // The first hold remembers what was there before any of them.
                if !self.restore.contains_key(parameter) {
                    let before = self.value(parameter, env).unwrap_or(0.0);
                    self.restore.insert(parameter.clone(), (*value_type, before));
                }
                self.claims.push(Claim {
                    automation_id: automation.id.clone(),
                    step: index,
                    hold: Hold::Parameter { address: parameter.clone() }
                });
                self.set_parameter(parameter, *value_type, *value, effects);
            }
            Action::AvatarParameter { .. } => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn claim_pin(&mut self, automation: &Automation, index: usize, action: &Action, follow: bool, duty: f32, now: Instant, effects: &mut Vec<Effect>) {
        let Action::BoardOutput { board_id, pin, .. } = action else {
            return;
        };

        if let Some(claim) = self.claims.iter_mut().find(|c| c.automation_id == automation.id && c.step == index) {
            if let Hold::Pin { duty: claimed, sent_at, sent_duty, .. } = &mut claim.hold {
                *claimed = duty;
                // Follow: send now unless it was just sent; the tick catches up otherwise.
                if sent_at.is_none_or_elapsed(now, FOLLOW_MIN_GAP) {
                    *sent_at = Some(now);
                    *sent_duty = Some(duty);
                    effects.push(Effect::Send(board_id.clone(), pin_message(*pin, follow, duty)));
                }
            }
            return;
        }

        effects.push(Effect::Send(board_id.clone(), pin_message(*pin, follow, duty)));
        self.claims.push(Claim {
            automation_id: automation.id.clone(),
            step: index,
            hold: Hold::Pin { board_id: board_id.clone(), pin: *pin, follow, duty, sent_at: Some(now), sent_duty: Some(duty) }
        });
    }

    /// Lets go of an automation's claims: one step's, or all of them.
    fn unclaim(&mut self, automation_id: &str, step: Option<usize>, now: Instant, effects: &mut Vec<Effect>) {
        let mut released = Vec::new();
        let mut index = 0;
        while index < self.claims.len() {
            let claim = &self.claims[index];
            if claim.automation_id == automation_id && step.map_or(true, |s| s == claim.step) {
                released.push(self.claims.remove(index).hold);
            } else {
                index += 1;
            }
        }

        for hold in released {
            match hold {
                Hold::Pin { board_id, pin, .. } => self.release_pin(&board_id, pin, now, effects),
                Hold::Parameter { address } => self.release_parameter(&address, effects)
            }
        }
    }

    /// Switches the output off unless something else still has it on.
    fn release_pin(&mut self, board_id: &str, pin: u8, now: Instant, effects: &mut Vec<Effect>) {
        if self.claims.iter().any(|c| matches!(&c.hold, Hold::Pin { board_id: b, pin: p, .. } if b == board_id && *p == pin)) {
            return;
        }
        if let Some(state) = self.pins.get(&(board_id.to_string(), pin)) {
            if is_future(state.pulse_until, now) || is_future(state.toggled_until, now) {
                return;
            }
        }
        effects.push(Effect::Send(board_id.to_string(), OutputMessage::Set { pin, on: false, ms: 0 }));
    }

    /// Sets the parameter back unless another hold still has it.
    fn release_parameter(&mut self, address: &str, effects: &mut Vec<Effect>) {
        if self.claims.iter().any(|c| matches!(&c.hold, Hold::Parameter { address: a } if a == address)) {
            return;
        }
        if let Some((value_type, value)) = self.restore.remove(address) {
            self.set_parameter(address, value_type, value, effects);
        }
    }

    /// Every 100 ms: runs delayed steps, renews leases and sends follow values that were held back.
    pub fn tick(&mut self, env: &dyn Env) -> Vec<Effect> {
        let now = Instant::now();
        let mut effects = Vec::new();
        if self.paused {
            return effects;
        }

        let (due, waiting): (Vec<Pending>, Vec<Pending>) = std::mem::take(&mut self.pending).into_iter().partition(|p| p.due <= now);
        self.pending = waiting;
        for pending in due {
            let Some(automation) = self.automations.iter().find(|a| a.id == pending.automation_id && a.enabled).cloned() else {
                continue;
            };
            self.run_step(&automation, pending.step, now, env, &mut effects);
        }

        for claim in &mut self.claims {
            let Hold::Pin { board_id, pin, follow, duty, sent_at, sent_duty } = &mut claim.hold else {
                continue;
            };
            let renew = sent_at.is_none_or_elapsed(now, RENEW_EVERY);
            let catch_up = *sent_duty != Some(*duty) && sent_at.is_none_or_elapsed(now, FOLLOW_MIN_GAP);
            if renew || catch_up {
                *sent_at = Some(now);
                *sent_duty = Some(*duty);
                effects.push(Effect::Send(board_id.clone(), pin_message(*pin, *follow, *duty)));
            }
        }

        self.report(now, &mut effects);
        effects
    }

    /// The avatar changed: every output lets go. Parameter holds are dropped without setting
    /// anything back, since those parameters belonged to the old avatar.
    pub fn release_all(&mut self) -> Vec<Effect> {
        let now = Instant::now();
        let mut effects = Vec::new();
        self.claims.retain(|c| matches!(c.hold, Hold::Pin { .. }));
        self.restore.clear();
        self.values.clear();
        self.pending.clear();

        let ids: Vec<String> = self.claims.iter().map(|c| c.automation_id.clone()).collect();
        for id in ids {
            self.unclaim(&id, None, now, &mut effects);
        }
        self.runtime.clear();
        self.pins.retain(|_, state| is_future(state.pulse_until, now));
        for state in self.pins.values_mut() {
            state.toggled_until = None;
            state.toggled_by = None;
        }
        self.report(now, &mut effects);
        effects
    }

    /// Lets go of one automation's outputs and parameters, e.g. when it's edited, disabled or
    /// deleted.
    pub fn release(&mut self, automation_id: &str) -> Vec<Effect> {
        let now = Instant::now();
        let mut effects = Vec::new();
        self.pending.retain(|p| p.automation_id != automation_id);
        self.unclaim(automation_id, None, now, &mut effects);
        self.runtime.remove(automation_id);
        self.report(now, &mut effects);
        effects
    }

    /// Stop everything: the boards get `stop` separately, so this only forgets what was running.
    /// Held parameters go back to what they were.
    pub fn set_paused(&mut self, paused: bool) -> Vec<Effect> {
        self.paused = paused;
        let mut effects = Vec::new();
        if !paused {
            return effects;
        }
        let parameters: Vec<String> = self.restore.keys().cloned().collect();
        self.claims.clear();
        for address in parameters {
            self.release_parameter(&address, &mut effects);
        }
        self.pending.clear();
        self.runtime.clear();
        self.pins.clear();
        self.report(Instant::now(), &mut effects);
        effects
    }

    fn is_running(&self, id: &str, now: Instant) -> bool {
        self.claims.iter().any(|c| c.automation_id == id)
            || self.pending.iter().any(|p| p.automation_id == id)
            || self.runtime.get(id).is_some_and(|r| is_future(r.busy_until, now))
            || self.pins.values().any(|p| p.toggled_by.as_deref() == Some(id) && is_future(p.toggled_until, now))
    }

    /// Tells the UI which automations started or stopped running.
    fn report(&mut self, now: Instant, effects: &mut Vec<Effect>) {
        let running: HashSet<String> =
            self.automations.iter().filter(|a| self.is_running(&a.id, now)).map(|a| a.id.clone()).collect();
        for id in running.difference(&self.reported) {
            effects.push(Effect::Activity(Activity { id: id.clone(), running: true }));
        }
        for id in self.reported.difference(&running) {
            effects.push(Effect::Activity(Activity { id: id.clone(), running: false }));
        }
        self.reported = running;
    }
}

trait ElapsedExt {
    fn is_none_or_elapsed(&self, now: Instant, gap: Duration) -> bool;
}

impl ElapsedExt for Option<Instant> {
    fn is_none_or_elapsed(&self, now: Instant, gap: Duration) -> bool {
        self.map_or(true, |at| now.duration_since(at) >= gap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automations::model::{Condition, Step};

    struct TestEnv;

    impl Env for TestEnv {
        fn max_on_ms(&self, _: &str, _: u8) -> Option<u32> {
            Some(10_000)
        }
        fn value(&self, _: &str) -> Option<f64> {
            None
        }
    }

    fn board(output: OutputAction) -> Step {
        Step { action: Action::BoardOutput { board_id: "b".into(), pin: 4, output }, delay_ms: 0 }
    }

    fn parameter(address: &str, value_type: ValueType, set: ParameterAction) -> Step {
        Step { action: Action::AvatarParameter { parameter: address.into(), value_type, set }, delay_ms: 0 }
    }

    fn automation_with(id: &str, condition: Condition, value_type: ValueType, then: Vec<Step>) -> Automation {
        Automation {
            id: id.into(),
            name: id.into(),
            enabled: true,
            when: Trigger::AvatarParameter {
                avatar_id: "avtr_1".into(),
                avatar_name: "Avatar".into(),
                parameter: "/avatar/parameters/P".into(),
                value_type,
                condition
            },
            then,
            cooldown_ms: 0,
            retrigger: Retrigger::Restart
        }
    }

    fn automation(id: &str, condition: Condition, value_type: ValueType, output: OutputAction) -> Automation {
        automation_with(id, condition, value_type, vec![board(output)])
    }

    fn sends(effects: &[Effect]) -> Vec<OutputMessage> {
        effects
            .iter()
            .filter_map(|e| match e {
                Effect::Send(_, m) => Some(m.clone()),
                _ => None
            })
            .collect()
    }

    fn parameters(effects: &[Effect]) -> Vec<(String, f64)> {
        effects
            .iter()
            .filter_map(|e| match e {
                Effect::Parameter { address, value, .. } => Some((address.clone(), *value)),
                _ => None
            })
            .collect()
    }

    fn activity(effects: &[Effect]) -> Vec<(String, bool)> {
        effects
            .iter()
            .filter_map(|e| match e {
                Effect::Activity(a) => Some((a.id.clone(), a.running)),
                _ => None
            })
            .collect()
    }

    fn change_effects(state: &mut State, previous: f64, current: f64) -> Vec<Effect> {
        state.on_parameter("avtr_1", "/avatar/parameters/P", Some(previous), current, &TestEnv)
    }

    fn change(state: &mut State, previous: f64, current: f64) -> Vec<OutputMessage> {
        sends(&change_effects(state, previous, current))
    }

    #[test]
    fn pulse_on_turns_on() {
        let mut state = State::default();
        state.automations.push(automation(
            "a",
            Condition::TurnsOn,
            ValueType::Bool,
            OutputAction::Pulse { on_ms: 2000, off_ms: 0, count: 1 }
        ));

        let effects = change_effects(&mut state, 0.0, 1.0);
        assert_eq!(sends(&effects), vec![OutputMessage::Pulse { pin: 4, on_ms: 2000, off_ms: 0, count: 1 }]);
        assert_eq!(activity(&effects), vec![("a".to_string(), true)]);
        assert!(change(&mut state, 1.0, 0.0).is_empty());
    }

    #[test]
    fn other_avatars_and_disabled_are_ignored() {
        let mut state = State::default();
        let mut disabled = automation("a", Condition::TurnsOn, ValueType::Bool, OutputAction::Toggle);
        disabled.enabled = false;
        state.automations.push(disabled);
        assert!(change(&mut state, 0.0, 1.0).is_empty());

        state.automations[0].enabled = true;
        let effects = state.on_parameter("avtr_2", "/avatar/parameters/P", Some(0.0), 1.0, &TestEnv);
        assert!(effects.is_empty());
    }

    #[test]
    fn ignore_while_busy_and_cooldown() {
        let mut state = State::default();
        let mut a = automation("a", Condition::TurnsOn, ValueType::Bool, OutputAction::Pulse { on_ms: 5000, off_ms: 0, count: 1 });
        a.retrigger = Retrigger::Ignore;
        state.automations.push(a);
        assert_eq!(change(&mut state, 0.0, 1.0).len(), 1);
        assert!(change(&mut state, 0.0, 1.0).is_empty());

        let mut state = State::default();
        let mut b = automation("b", Condition::TurnsOn, ValueType::Bool, OutputAction::Pulse { on_ms: 10, off_ms: 0, count: 1 });
        b.cooldown_ms = 60_000;
        state.automations.push(b);
        assert_eq!(change(&mut state, 0.0, 1.0).len(), 1);
        assert!(change(&mut state, 0.0, 1.0).is_empty());
    }

    #[test]
    fn hold_claims_and_releases() {
        let mut state = State::default();
        state.automations.push(automation("a", Condition::IsOn, ValueType::Bool, OutputAction::Hold));

        assert_eq!(change(&mut state, 0.0, 1.0), vec![OutputMessage::Set { pin: 4, on: true, ms: LEASE_MS }]);
        let effects = change_effects(&mut state, 1.0, 0.0);
        assert_eq!(sends(&effects), vec![OutputMessage::Set { pin: 4, on: false, ms: 0 }]);
        assert_eq!(activity(&effects), vec![("a".to_string(), false)]);
    }

    #[test]
    fn release_keeps_other_claims() {
        let mut state = State::default();
        state.automations.push(automation("a", Condition::IsOn, ValueType::Bool, OutputAction::Hold));
        let mut b = automation("b", Condition::IsOn, ValueType::Bool, OutputAction::Hold);
        let Trigger::AvatarParameter { parameter, .. } = &mut b.when;
        *parameter = "/avatar/parameters/Q".into();
        state.automations.push(b);

        change(&mut state, 0.0, 1.0);
        state.on_parameter("avtr_1", "/avatar/parameters/Q", Some(0.0), 1.0, &TestEnv);
        // "a" lets go, but "b" still holds the same output: no off.
        assert!(change(&mut state, 1.0, 0.0).is_empty());
    }

    #[test]
    fn toggle_flips() {
        let mut state = State::default();
        state.automations.push(automation("a", Condition::TurnsOn, ValueType::Bool, OutputAction::Toggle));

        assert_eq!(change(&mut state, 0.0, 1.0), vec![OutputMessage::Pulse { pin: 4, on_ms: 10_000, off_ms: 0, count: 1 }]);
        assert!(state.is_running("a", Instant::now()));
        assert_eq!(change(&mut state, 0.0, 1.0), vec![OutputMessage::Set { pin: 4, on: false, ms: 0 }]);
        assert!(!state.is_running("a", Instant::now()));
    }

    #[test]
    fn follow_maps_duty_and_stops_at_zero() {
        let mut state = State::default();
        state.automations.push(automation("a", Condition::Any, ValueType::Float, OutputAction::Follow { min: 0.0, max: 0.5 }));

        assert_eq!(change(&mut state, 0.0, 1.0), vec![OutputMessage::Pwm { pin: 4, duty: 0.5, ms: LEASE_MS }]);
        assert_eq!(change(&mut state, 1.0, 0.0), vec![OutputMessage::Set { pin: 4, on: false, ms: 0 }]);
    }

    #[test]
    fn paused_does_nothing() {
        let mut state = State::default();
        state.automations.push(automation("a", Condition::IsOn, ValueType::Bool, OutputAction::Hold));
        change(&mut state, 0.0, 1.0);
        state.set_paused(true);
        assert!(change(&mut state, 0.0, 1.0).is_empty());
        assert!(state.tick(&TestEnv).is_empty());
    }

    #[test]
    fn runs_every_step() {
        let mut state = State::default();
        state.automations.push(automation_with(
            "a",
            Condition::TurnsOn,
            ValueType::Bool,
            vec![
                board(OutputAction::Pulse { on_ms: 200, off_ms: 0, count: 1 }),
                parameter("/avatar/parameters/Ears", ValueType::Bool, ParameterAction::Set { value: 1.0 })
            ]
        ));

        let effects = change_effects(&mut state, 0.0, 1.0);
        assert_eq!(sends(&effects).len(), 1);
        assert_eq!(parameters(&effects), vec![("/avatar/parameters/Ears".to_string(), 1.0)]);
    }

    #[test]
    fn delayed_steps_wait_for_tick() {
        let mut state = State::default();
        let mut step = parameter("/avatar/parameters/Ears", ValueType::Bool, ParameterAction::Set { value: 1.0 });
        step.delay_ms = 1;
        state.automations.push(automation_with("a", Condition::TurnsOn, ValueType::Bool, vec![step]));

        let effects = change_effects(&mut state, 0.0, 1.0);
        assert!(parameters(&effects).is_empty());
        assert_eq!(activity(&effects), vec![("a".to_string(), true)]);

        std::thread::sleep(Duration::from_millis(5));
        let effects = state.tick(&TestEnv);
        assert_eq!(parameters(&effects), vec![("/avatar/parameters/Ears".to_string(), 1.0)]);
        assert_eq!(activity(&effects), vec![("a".to_string(), false)]);
    }

    #[test]
    fn exit_cancels_a_delayed_hold() {
        let mut state = State::default();
        let mut step = board(OutputAction::Hold);
        step.delay_ms = 1;
        state.automations.push(automation_with("a", Condition::IsOn, ValueType::Bool, vec![step]));

        assert!(change(&mut state, 0.0, 1.0).is_empty());
        change(&mut state, 1.0, 0.0);
        std::thread::sleep(Duration::from_millis(5));
        assert!(sends(&state.tick(&TestEnv)).is_empty());
    }

    #[test]
    fn parameter_toggle_flips_the_known_value() {
        let mut state = State::default();
        state.automations.push(automation_with(
            "a",
            Condition::TurnsOn,
            ValueType::Bool,
            vec![parameter("/avatar/parameters/Ears", ValueType::Bool, ParameterAction::Toggle)]
        ));

        assert_eq!(parameters(&change_effects(&mut state, 0.0, 1.0)), vec![("/avatar/parameters/Ears".to_string(), 1.0)]);
        assert_eq!(parameters(&change_effects(&mut state, 0.0, 1.0)), vec![("/avatar/parameters/Ears".to_string(), 0.0)]);
    }

    #[test]
    fn parameter_hold_sets_back_what_was_there() {
        let mut state = State::default();
        state.automations.push(automation_with(
            "a",
            Condition::IsOn,
            ValueType::Bool,
            vec![parameter("/avatar/parameters/Outfit", ValueType::Int, ParameterAction::Hold { value: 3.0 })]
        ));
        state.on_parameter("avtr_1", "/avatar/parameters/Outfit", Some(0.0), 2.0, &TestEnv);

        assert_eq!(parameters(&change_effects(&mut state, 0.0, 1.0)), vec![("/avatar/parameters/Outfit".to_string(), 3.0)]);
        assert_eq!(parameters(&change_effects(&mut state, 1.0, 0.0)), vec![("/avatar/parameters/Outfit".to_string(), 2.0)]);
    }

    #[test]
    fn avatar_change_drops_parameter_holds_silently() {
        let mut state = State::default();
        state.automations.push(automation_with(
            "a",
            Condition::IsOn,
            ValueType::Bool,
            vec![parameter("/avatar/parameters/Ears", ValueType::Bool, ParameterAction::Hold { value: 1.0 })]
        ));
        change(&mut state, 0.0, 1.0);

        let effects = state.release_all();
        assert!(parameters(&effects).is_empty());
        assert_eq!(activity(&effects), vec![("a".to_string(), false)]);
    }

    #[test]
    fn stop_everything_sets_held_parameters_back() {
        let mut state = State::default();
        state.automations.push(automation_with(
            "a",
            Condition::IsOn,
            ValueType::Bool,
            vec![parameter("/avatar/parameters/Ears", ValueType::Bool, ParameterAction::Hold { value: 1.0 })]
        ));
        change(&mut state, 0.0, 1.0);

        assert_eq!(parameters(&state.set_paused(true)), vec![("/avatar/parameters/Ears".to_string(), 0.0)]);
    }

    #[test]
    fn parameter_follow_maps_the_range() {
        let mut state = State::default();
        state.automations.push(automation_with(
            "a",
            Condition::Any,
            ValueType::Float,
            vec![parameter("/avatar/parameters/Glow", ValueType::Float, ParameterAction::Follow { min: -1.0, max: 1.0 })]
        ));

        assert_eq!(parameters(&change_effects(&mut state, 0.0, 0.5)), vec![("/avatar/parameters/Glow".to_string(), 0.0)]);
    }
}
