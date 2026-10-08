//! Runs automations. Parameter changes come in from `osc::udp::handle_message`; what to send is
//! worked out under the lock and sent after it, so a slow board never holds anything up.
//!
//! Boards keep time themselves (see the firmware's PROTOCOL.md): a pulse is one message. Holds and
//! follows are leases instead: a claim on the output that `tick` renews every 500 ms, so the
//! output goes off by itself within 1.5 s if the app freezes or quits.
use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::Serialize;

use super::model::{Action, Automation, Edge, OutputAction, Retrigger, Trigger};
use crate::boards::protocol::OutputMessage;

/// How long each hold/follow message keeps the output on, and how often it's renewed.
pub const LEASE_MS: u32 = 1500;
const RENEW_EVERY: Duration = Duration::from_millis(500);
/// Follow sends at most this often; the latest value goes out on the next tick.
const FOLLOW_MIN_GAP: Duration = Duration::from_millis(50);
/// A Test of an open-ended action switches the output on for this long.
pub const TEST_MS: u32 = 1000;

#[derive(Default)]
struct Runtime {
    last_run: Option<Instant>,
    busy_until: Option<Instant>
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ClaimKind {
    Hold,
    Follow
}

struct Claim {
    automation_id: String,
    board_id: String,
    pin: u8,
    kind: ClaimKind,
    duty: f32,
    sent_at: Option<Instant>,
    sent_duty: Option<f32>
}

impl Claim {
    fn message(&self) -> OutputMessage {
        match self.kind {
            ClaimKind::Hold => OutputMessage::Set { pin: self.pin, on: true, ms: LEASE_MS },
            ClaimKind::Follow => OutputMessage::Pwm { pin: self.pin, duty: self.duty, ms: LEASE_MS }
        }
    }
}

/// Things the board times itself, so a release doesn't cut them short.
#[derive(Default)]
struct PinState {
    pulse_until: Option<Instant>,
    toggled_until: Option<Instant>
}

/// What the UI shows as "Running".
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: String,
    pub running: bool,
    /// For pulses and toggles: when it ends on its own.
    pub for_ms: Option<u32>
}

pub enum Effect {
    Send(String, OutputMessage),
    Activity(Activity)
}

#[derive(Default)]
pub struct State {
    pub automations: Vec<Automation>,
    pub paused: bool,
    runtime: HashMap<String, Runtime>,
    claims: Vec<Claim>,
    pins: HashMap<(String, u8), PinState>
}

fn running(id: &str, running: bool, for_ms: Option<u32>) -> Effect {
    Effect::Activity(Activity { id: id.to_string(), running, for_ms })
}

fn is_future(at: Option<Instant>, now: Instant) -> bool {
    at.is_some_and(|at| at > now)
}

impl State {
    /// A live change of an avatar parameter. `max_on_ms` looks up an output's max on-time (for
    /// toggles); it's passed in so this stays free of app state and testable.
    pub fn on_parameter(
        &mut self,
        avatar_id: &str,
        address: &str,
        previous: Option<f64>,
        current: f64,
        max_on_ms: &dyn Fn(&str, u8) -> Option<u32>
    ) -> Vec<Effect> {
        let mut effects = Vec::new();
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
                Edge::Fire => self.fire(&automation, now, max_on_ms, &mut effects),
                Edge::Enter => self.claim(&automation, ClaimKind::Hold, 1.0, now, &mut effects),
                Edge::Exit => self.unclaim(&automation.id, now, &mut effects),
                Edge::Follow(value) => {
                    let Action::BoardOutput { output: OutputAction::Follow { min, max }, .. } = &automation.then else {
                        continue;
                    };
                    let duty = min + value * (max - min);
                    if duty <= 0.001 {
                        self.unclaim(&automation.id, now, &mut effects);
                    } else {
                        self.claim(&automation, ClaimKind::Follow, duty, now, &mut effects);
                    }
                }
                Edge::None => {}
            }
        }

        effects
    }

    fn fire(&mut self, automation: &Automation, now: Instant, max_on_ms: &dyn Fn(&str, u8) -> Option<u32>, effects: &mut Vec<Effect>) {
        let runtime = self.runtime.entry(automation.id.clone()).or_default();

        if automation.cooldown_ms > 0
            && runtime.last_run.is_some_and(|at| now < at + Duration::from_millis(u64::from(automation.cooldown_ms)))
        {
            return;
        }
        if automation.retrigger == Retrigger::Ignore && is_future(runtime.busy_until, now) {
            return;
        }

        let Action::BoardOutput { board_id, pin, output } = &automation.then;
        let key = (board_id.clone(), *pin);

        match output {
            OutputAction::Pulse { on_ms, off_ms, count } => {
                let busy = output.busy_ms().unwrap_or(*on_ms);
                runtime.last_run = Some(now);
                runtime.busy_until = Some(now + Duration::from_millis(u64::from(busy)));
                let pin_state = self.pins.entry(key).or_default();
                pin_state.pulse_until = runtime.busy_until.max(pin_state.pulse_until);

                effects.push(Effect::Send(
                    board_id.clone(),
                    OutputMessage::Pulse { pin: *pin, on_ms: *on_ms, off_ms: *off_ms, count: *count }
                ));
                effects.push(running(&automation.id, true, Some(busy)));
            }
            OutputAction::Toggle => {
                runtime.last_run = Some(now);
                let pin_state = self.pins.entry(key).or_default();

                if is_future(pin_state.toggled_until, now) {
                    pin_state.toggled_until = None;
                    runtime.busy_until = None;
                    effects.push(Effect::Send(board_id.clone(), OutputMessage::Set { pin: *pin, on: false, ms: 0 }));
                    effects.push(running(&automation.id, false, None));
                } else {
                    // On until toggled again, never past the output's max on-time. A pulse, so
                    // reaching that limit doesn't trip the output.
                    let ms = max_on_ms(board_id, *pin).unwrap_or(10_000);
                    pin_state.toggled_until = Some(now + Duration::from_millis(u64::from(ms)));
                    runtime.busy_until = pin_state.toggled_until;
                    effects.push(Effect::Send(
                        board_id.clone(),
                        OutputMessage::Pulse { pin: *pin, on_ms: ms, off_ms: 0, count: 1 }
                    ));
                    effects.push(running(&automation.id, true, Some(ms)));
                }
            }
            OutputAction::Off => {
                runtime.last_run = Some(now);
                // Everything else driving this output lets go too, or the next renewal would
                // switch it straight back on.
                let released: Vec<String> = self
                    .claims
                    .iter()
                    .filter(|c| c.board_id == *board_id && c.pin == *pin)
                    .map(|c| c.automation_id.clone())
                    .collect();
                self.claims.retain(|c| !(c.board_id == *board_id && c.pin == *pin));
                self.pins.remove(&key);
                for id in released {
                    effects.push(running(&id, false, None));
                }
                effects.push(Effect::Send(board_id.clone(), OutputMessage::Set { pin: *pin, on: false, ms: 0 }));
            }
            // State and follow actions don't fire on moments (validation rules that out).
            OutputAction::Hold | OutputAction::Follow { .. } => {}
        }
    }

    fn claim(&mut self, automation: &Automation, kind: ClaimKind, duty: f32, now: Instant, effects: &mut Vec<Effect>) {
        let Action::BoardOutput { board_id, pin, .. } = &automation.then;

        if let Some(claim) = self.claims.iter_mut().find(|c| c.automation_id == automation.id) {
            claim.duty = duty;
            // Follow: send now unless it was just sent; the tick catches up otherwise.
            if claim.sent_at.is_none_or_elapsed(now, FOLLOW_MIN_GAP) {
                claim.sent_at = Some(now);
                claim.sent_duty = Some(duty);
                effects.push(Effect::Send(board_id.clone(), claim.message()));
            }
            return;
        }

        let claim = Claim {
            automation_id: automation.id.clone(),
            board_id: board_id.clone(),
            pin: *pin,
            kind,
            duty,
            sent_at: Some(now),
            sent_duty: Some(duty)
        };
        effects.push(Effect::Send(board_id.clone(), claim.message()));
        effects.push(running(&automation.id, true, None));
        self.claims.push(claim);
    }

    fn unclaim(&mut self, automation_id: &str, now: Instant, effects: &mut Vec<Effect>) {
        let Some(index) = self.claims.iter().position(|c| c.automation_id == automation_id) else {
            return;
        };
        let claim = self.claims.remove(index);
        effects.push(running(automation_id, false, None));
        self.release_pin(&claim.board_id, claim.pin, now, effects);
    }

    /// Switches the output off unless something else still has it on.
    fn release_pin(&mut self, board_id: &str, pin: u8, now: Instant, effects: &mut Vec<Effect>) {
        if self.claims.iter().any(|c| c.board_id == board_id && c.pin == pin) {
            return;
        }
        if let Some(state) = self.pins.get(&(board_id.to_string(), pin)) {
            if is_future(state.pulse_until, now) || is_future(state.toggled_until, now) {
                return;
            }
        }
        effects.push(Effect::Send(board_id.to_string(), OutputMessage::Set { pin, on: false, ms: 0 }));
    }

    /// Every 100 ms: renews leases, and sends follow values that were held back.
    pub fn tick(&mut self) -> Vec<Effect> {
        let now = Instant::now();
        let mut effects = Vec::new();
        if self.paused {
            return effects;
        }

        for claim in &mut self.claims {
            let renew = claim.sent_at.is_none_or_elapsed(now, RENEW_EVERY);
            let catch_up = claim.sent_duty != Some(claim.duty) && claim.sent_at.is_none_or_elapsed(now, FOLLOW_MIN_GAP);
            if renew || catch_up {
                claim.sent_at = Some(now);
                claim.sent_duty = Some(claim.duty);
                effects.push(Effect::Send(claim.board_id.clone(), claim.message()));
            }
        }
        effects
    }

    /// Lets go of every output (avatar change, an automation edited or removed...).
    pub fn release_all(&mut self) -> Vec<Effect> {
        let now = Instant::now();
        let mut effects = Vec::new();
        let ids: Vec<String> = self.claims.iter().map(|c| c.automation_id.clone()).collect();
        for id in ids {
            self.unclaim(&id, now, &mut effects);
        }
        self.runtime.clear();
        self.pins.retain(|_, state| is_future(state.pulse_until, now));
        for state in self.pins.values_mut() {
            state.toggled_until = None;
        }
        effects
    }

    /// Lets go of one automation's output, e.g. when it's edited, disabled or deleted.
    pub fn release(&mut self, automation_id: &str) -> Vec<Effect> {
        let mut effects = Vec::new();
        self.unclaim(automation_id, Instant::now(), &mut effects);
        self.runtime.remove(automation_id);
        effects
    }

    /// Stop everything: the boards get `stop` separately, so this only forgets what was running.
    pub fn set_paused(&mut self, paused: bool) -> Vec<Effect> {
        self.paused = paused;
        if !paused {
            return Vec::new();
        }
        let effects = self.claims.iter().map(|c| running(&c.automation_id, false, None)).collect();
        self.claims.clear();
        self.runtime.clear();
        self.pins.clear();
        effects
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
    use crate::automations::model::{Condition, ValueType};

    fn automation(id: &str, condition: Condition, value_type: ValueType, output: OutputAction) -> Automation {
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
            then: Action::BoardOutput { board_id: "b".into(), pin: 4, output },
            cooldown_ms: 0,
            retrigger: Retrigger::Restart
        }
    }

    fn sends(effects: &[Effect]) -> Vec<OutputMessage> {
        effects
            .iter()
            .filter_map(|e| match e {
                Effect::Send(_, m) => Some(m.clone()),
                Effect::Activity(_) => None
            })
            .collect()
    }

    const NO_MAX: &dyn Fn(&str, u8) -> Option<u32> = &|_, _| Some(10_000);

    fn change(state: &mut State, previous: f64, current: f64) -> Vec<OutputMessage> {
        sends(&state.on_parameter("avtr_1", "/avatar/parameters/P", Some(previous), current, NO_MAX))
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

        assert_eq!(change(&mut state, 0.0, 1.0), vec![OutputMessage::Pulse { pin: 4, on_ms: 2000, off_ms: 0, count: 1 }]);
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
        let effects = state.on_parameter("avtr_2", "/avatar/parameters/P", Some(0.0), 1.0, NO_MAX);
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
        assert_eq!(change(&mut state, 1.0, 0.0), vec![OutputMessage::Set { pin: 4, on: false, ms: 0 }]);
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
        state.on_parameter("avtr_1", "/avatar/parameters/Q", Some(0.0), 1.0, NO_MAX);
        // "a" lets go, but "b" still holds the same output: no off.
        assert!(change(&mut state, 1.0, 0.0).is_empty());
    }

    #[test]
    fn toggle_flips() {
        let mut state = State::default();
        state.automations.push(automation("a", Condition::TurnsOn, ValueType::Bool, OutputAction::Toggle));

        assert_eq!(change(&mut state, 0.0, 1.0), vec![OutputMessage::Pulse { pin: 4, on_ms: 10_000, off_ms: 0, count: 1 }]);
        assert_eq!(change(&mut state, 0.0, 1.0), vec![OutputMessage::Set { pin: 4, on: false, ms: 0 }]);
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
        assert!(state.tick().is_empty());
    }
}
