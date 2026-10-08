//! What an automation is: *when* something happens, *do* something. Both sides are tagged unions
//! (`kind`) so new trigger and action kinds slot in without reshaping saved data. Mirrored by hand
//! in the frontend's `lib/tauri-bridge.ts` and `utils/automations.ts`.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Automation {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub when: Trigger,
    pub then: Action,
    /// Minimum gap between two runs, 0 = none.
    #[serde(default)]
    pub cooldown_ms: u32,
    /// What a moment trigger does while the previous run is still going.
    #[serde(default)]
    pub retrigger: Retrigger
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Retrigger {
    #[default]
    Restart,
    Ignore
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Trigger {
    #[serde(rename_all = "camelCase")]
    AvatarParameter {
        avatar_id: String,
        avatar_name: String,
        /// Full OSC address, e.g. `/avatar/parameters/Boop`.
        parameter: String,
        value_type: ValueType,
        condition: Condition
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueType {
    Bool,
    Int,
    Float
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Condition {
    TurnsOn,
    TurnsOff,
    IsOn,
    Becomes { value: f64 },
    Is { value: f64 },
    RisesAbove { value: f64 },
    IsAbove { value: f64 },
    /// Every change of a Float. Only with `follow`.
    Any
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Action {
    #[serde(rename_all = "camelCase")]
    BoardOutput { board_id: String, pin: u8, output: OutputAction }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum OutputAction {
    #[serde(rename_all = "camelCase")]
    Pulse { on_ms: u32, off_ms: u32, count: u32 },
    /// On while the state condition holds.
    Hold,
    Toggle,
    Off,
    /// PWM duty from a Float, mapped onto min..max (0–1).
    Follow { min: f32, max: f32 }
}

/// What a parameter change means for one condition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Edge {
    /// A moment condition happened.
    Fire,
    /// A state condition became true / stopped being true.
    Enter,
    Exit,
    /// `any`: the new value, clamped to 0–1.
    Follow(f32),
    None
}

/// Bools are 0/1 on the way in.
const ON: f64 = 0.5;

fn same_int(a: f64, b: f64) -> bool {
    a.round() == b.round()
}

impl Condition {
    /// A moment (an edge) rather than a state (a level).
    pub fn is_moment(&self) -> bool {
        matches!(self, Condition::TurnsOn | Condition::TurnsOff | Condition::Becomes { .. } | Condition::RisesAbove { .. })
    }

    pub fn fits(&self, value_type: ValueType) -> bool {
        match self {
            Condition::TurnsOn | Condition::TurnsOff | Condition::IsOn => value_type == ValueType::Bool,
            Condition::Becomes { .. } | Condition::Is { .. } => value_type == ValueType::Int,
            Condition::RisesAbove { .. } | Condition::IsAbove { .. } | Condition::Any => value_type == ValueType::Float
        }
    }

    /// For state conditions: whether `value` satisfies it.
    fn holds(&self, value: f64) -> bool {
        match self {
            Condition::IsOn => value >= ON,
            Condition::Is { value: target } => same_int(value, *target),
            Condition::IsAbove { value: threshold } => value > *threshold,
            _ => false
        }
    }

    /// `previous` is None the first time the parameter is seen this session.
    pub fn evaluate(&self, previous: Option<f64>, current: f64) -> Edge {
        match self {
            Condition::TurnsOn => edge(current >= ON && previous.map_or(true, |p| p < ON)),
            Condition::TurnsOff => edge(current < ON && previous.is_some_and(|p| p >= ON)),
            Condition::Becomes { value } => edge(same_int(current, *value) && previous.map_or(true, |p| !same_int(p, *value))),
            Condition::RisesAbove { value } => edge(current > *value && previous.map_or(true, |p| p <= *value)),
            Condition::Any => Edge::Follow(current.clamp(0.0, 1.0) as f32),
            state => match (previous.is_some_and(|p| state.holds(p)), state.holds(current)) {
                (false, true) => Edge::Enter,
                (true, false) => Edge::Exit,
                _ => Edge::None
            }
        }
    }
}

fn edge(fired: bool) -> Edge {
    if fired {
        Edge::Fire
    } else {
        Edge::None
    }
}

impl OutputAction {
    /// How long a run keeps the output busy, for `retrigger: ignore`. None for open-ended ones.
    pub fn busy_ms(&self) -> Option<u32> {
        match self {
            OutputAction::Pulse { on_ms, off_ms, count } => {
                let count = (*count).max(1);
                Some(on_ms.saturating_mul(count).saturating_add(off_ms.saturating_mul(count - 1)))
            }
            _ => None
        }
    }
}

/// Checks what can be checked without knowing the boards. Returns a message to show.
pub fn validate(automation: &Automation) -> Result<(), String> {
    if automation.name.trim().is_empty() {
        return Err("Give the automation a name.".into());
    }

    let Trigger::AvatarParameter { parameter, value_type, condition, avatar_id, .. } = &automation.when;
    if avatar_id.is_empty() || parameter.is_empty() {
        return Err("Pick the avatar parameter that triggers it.".into());
    }
    if !condition.fits(*value_type) {
        return Err("That condition doesn't fit this parameter's type.".into());
    }

    let Action::BoardOutput { output, .. } = &automation.then;
    match output {
        OutputAction::Pulse { on_ms, count, .. } => {
            if !condition.is_moment() {
                return Err("A pulse needs a moment, like \"turns on\".".into());
            }
            if *on_ms == 0 || *count == 0 || *count > 100 {
                return Err("A pulse needs a length, and 1 to 100 repeats.".into());
            }
        }
        OutputAction::Toggle | OutputAction::Off => {
            if !condition.is_moment() {
                return Err("This action needs a moment, like \"turns on\".".into());
            }
        }
        OutputAction::Hold => {
            if condition.is_moment() || *condition == Condition::Any {
                return Err("Hold needs a state, like \"is on\".".into());
            }
        }
        OutputAction::Follow { min, max } => {
            if *condition != Condition::Any {
                return Err("Follow needs \"any change\" of a Float.".into());
            }
            if !(0.0..=1.0).contains(min) || !(0.0..=1.0).contains(max) || min > max {
                return Err("Follow's range must be within 0–100 %, low to high.".into());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(condition: Condition, value_type: ValueType, output: OutputAction) -> Automation {
        Automation {
            id: "a".into(),
            name: "Boop fan".into(),
            enabled: true,
            when: Trigger::AvatarParameter {
                avatar_id: "avtr_1".into(),
                avatar_name: "Avatar".into(),
                parameter: "/avatar/parameters/Boop".into(),
                value_type,
                condition
            },
            then: Action::BoardOutput { board_id: "a1b2c3".into(), pin: 4, output },
            cooldown_ms: 0,
            retrigger: Retrigger::Restart
        }
    }

    #[test]
    fn bool_moments() {
        assert_eq!(Condition::TurnsOn.evaluate(Some(0.0), 1.0), Edge::Fire);
        assert_eq!(Condition::TurnsOn.evaluate(None, 1.0), Edge::Fire);
        assert_eq!(Condition::TurnsOn.evaluate(Some(1.0), 1.0), Edge::None);
        assert_eq!(Condition::TurnsOff.evaluate(Some(1.0), 0.0), Edge::Fire);
        assert_eq!(Condition::TurnsOff.evaluate(None, 0.0), Edge::None);
    }

    #[test]
    fn states() {
        assert_eq!(Condition::IsOn.evaluate(Some(0.0), 1.0), Edge::Enter);
        assert_eq!(Condition::IsOn.evaluate(Some(1.0), 0.0), Edge::Exit);
        assert_eq!(Condition::IsOn.evaluate(Some(1.0), 1.0), Edge::None);
        assert_eq!(Condition::Is { value: 3.0 }.evaluate(Some(2.0), 3.0), Edge::Enter);
        assert_eq!(Condition::IsAbove { value: 0.8 }.evaluate(Some(0.9), 0.5), Edge::Exit);
    }

    #[test]
    fn int_and_float_moments() {
        assert_eq!(Condition::Becomes { value: 2.0 }.evaluate(Some(1.0), 2.0), Edge::Fire);
        assert_eq!(Condition::Becomes { value: 2.0 }.evaluate(Some(2.0), 2.0), Edge::None);
        assert_eq!(Condition::RisesAbove { value: 0.5 }.evaluate(Some(0.4), 0.6), Edge::Fire);
        assert_eq!(Condition::RisesAbove { value: 0.5 }.evaluate(Some(0.6), 0.7), Edge::None);
        assert_eq!(Condition::Any.evaluate(Some(0.1), 1.4), Edge::Follow(1.0));
    }

    #[test]
    fn pulse_busy_time() {
        let pulse = OutputAction::Pulse { on_ms: 200, off_ms: 300, count: 3 };
        assert_eq!(pulse.busy_ms(), Some(1200));
        assert_eq!(OutputAction::Hold.busy_ms(), None);
    }

    #[test]
    fn validation() {
        let pulse = OutputAction::Pulse { on_ms: 2000, off_ms: 0, count: 1 };
        assert!(validate(&sample(Condition::TurnsOn, ValueType::Bool, pulse.clone())).is_ok());
        assert!(validate(&sample(Condition::IsOn, ValueType::Bool, pulse)).is_err());
        assert!(validate(&sample(Condition::IsOn, ValueType::Bool, OutputAction::Hold)).is_ok());
        assert!(validate(&sample(Condition::TurnsOn, ValueType::Float, OutputAction::Toggle)).is_err());
        assert!(validate(&sample(Condition::Any, ValueType::Float, OutputAction::Follow { min: 0.0, max: 0.8 })).is_ok());
        assert!(validate(&sample(Condition::Any, ValueType::Float, OutputAction::Hold)).is_err());
    }

    #[test]
    fn serde_shape() {
        let json = serde_json::to_value(sample(
            Condition::TurnsOn,
            ValueType::Bool,
            OutputAction::Pulse { on_ms: 2000, off_ms: 0, count: 1 }
        ))
        .unwrap();
        assert_eq!(json["when"]["kind"], "avatar-parameter");
        assert_eq!(json["when"]["valueType"], "Bool");
        assert_eq!(json["when"]["condition"]["type"], "turns-on");
        assert_eq!(json["then"]["kind"], "board-output");
        assert_eq!(json["then"]["boardId"], "a1b2c3");
        assert_eq!(json["then"]["output"]["type"], "pulse");
        assert_eq!(json["then"]["output"]["onMs"], 2000);
        assert_eq!(json["retrigger"], "restart");
    }
}
