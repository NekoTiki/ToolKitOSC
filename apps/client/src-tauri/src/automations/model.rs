//! What an automation is: *when* something happens, *do* something. Both sides are tagged unions
//! (`kind`) so new trigger and action kinds slot in without reshaping saved data. Mirrored by hand
//! in the frontend's `lib/tauri-bridge.ts` and `utils/automations.ts`.
use serde::{Deserialize, Deserializer, Serialize};

/// Most steps one automation can run.
pub const MAX_STEPS: usize = 8;
/// Longest "after" delay on a step.
pub const MAX_DELAY_MS: u32 = 10_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Automation {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub when: Trigger,
    /// Run together, each after its own delay. Older files have a single action here.
    #[serde(deserialize_with = "one_or_many")]
    pub then: Vec<Step>,
    /// Minimum gap between two runs, 0 = none.
    #[serde(default)]
    pub cooldown_ms: u32,
    /// What a moment trigger does while the previous run is still going.
    #[serde(default)]
    pub retrigger: Retrigger,
    /// All of these have to be true for it to start.
    #[serde(default)]
    pub only_if: Vec<Guard>
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
        condition: Condition,
        /// Runs on any avatar with a parameter of this name, not only `avatar_id`. Handy for
        /// standard names (VRCFury Haptics, OSC Goes Brrr, PhysBones).
        #[serde(default)]
        any_avatar: bool,
        /// A state only counts once it has held this long, 0 = at once.
        #[serde(default)]
        held_ms: u32
    },
    /// You start wearing an avatar: this one, or any (None). Also when the app first sees it.
    #[serde(rename_all = "camelCase")]
    AvatarChanged { avatar_id: Option<String>, avatar_name: Option<String> },
    /// Something happens in the app: viewers, Stop everything. The frontend reports these
    /// (`automation_event`).
    AppEvent { event: AppEvent }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum AppEvent {
    /// A viewer uses a control: this one, any of this type (e.g. a shocker), or any.
    #[serde(rename_all = "camelCase")]
    ViewerCommand {
        #[serde(default)]
        control_id: Option<String>,
        #[serde(default)]
        control_type: Option<String>
    },
    ViewerJoined,
    /// The share page goes from no viewers to some.
    FirstViewer,
    LastViewerLeft,
    /// Stop everything is turned off.
    Resumed
}

impl AppEvent {
    /// Whether `happened` (what the frontend reported) is this event.
    pub fn matches(&self, happened: &AppEvent) -> bool {
        match (self, happened) {
            (
                AppEvent::ViewerCommand { control_id, control_type },
                AppEvent::ViewerCommand { control_id: id, control_type: kind }
            ) => {
                control_id.as_ref().map_or(true, |c| Some(c) == id.as_ref())
                    && control_type.as_ref().map_or(true, |t| Some(t) == kind.as_ref())
            }
            (a, b) => a == b
        }
    }
}

/// Something that has to be true for an automation to start. Checked when it starts, not while
/// it runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Guard {
    /// Another parameter is in a state (`is-on`, `is`, `is-above`), or isn't (`not`).
    #[serde(rename_all = "camelCase")]
    Parameter {
        parameter: String,
        value_type: ValueType,
        condition: Condition,
        #[serde(default)]
        not: bool
    },
    /// This profile is active (None: No profile).
    #[serde(rename_all = "camelCase")]
    Profile { profile_id: Option<String> },
    /// Viewers are on the share page, or none are.
    Viewers { online: bool }
}

/// Most "only if" conditions on one automation.
pub const MAX_GUARDS: usize = 4;

impl Guard {
    /// `value` looks up a parameter's current value.
    pub fn holds(&self, context: &Context, value: &dyn Fn(&str) -> Option<f64>) -> bool {
        match self {
            Guard::Parameter { parameter, condition, not, .. } => value(parameter).is_some_and(|v| condition.holds(v)) != *not,
            Guard::Profile { profile_id } => context.profile_id == *profile_id,
            Guard::Viewers { online } => (context.viewers > 0) == *online
        }
    }
}

/// What the frontend knows that guards need.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    pub profile_id: Option<String>,
    pub viewers: u32
}

/// Longest "held for" on a state.
pub const MAX_HELD_MS: u32 = 60_000;

impl Trigger {
    /// What kind of steps it can run.
    pub fn runs(&self) -> Runs {
        match self {
            Trigger::AvatarParameter { condition, .. } => condition.runs(),
            Trigger::AvatarChanged { .. } | Trigger::AppEvent { .. } => Runs::Moment
        }
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

/// One thing an automation does: an action, plus how long after the trigger it starts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    #[serde(flatten)]
    pub action: Action,
    #[serde(default)]
    pub delay_ms: u32
}

fn one_or_many<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Step>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        Many(Vec<Step>),
        One(Step)
    }
    Ok(match OneOrMany::deserialize(deserializer)? {
        OneOrMany::Many(steps) => steps,
        OneOrMany::One(step) => vec![step]
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Action {
    #[serde(rename_all = "camelCase")]
    BoardOutput { board_id: String, pin: u8, output: OutputAction },
    /// Sets one of the worn avatar's own parameters.
    #[serde(rename_all = "camelCase")]
    AvatarParameter {
        /// Full OSC address, e.g. `/avatar/parameters/Ears`.
        parameter: String,
        value_type: ValueType,
        set: ParameterAction
    },
    /// Runs one of the avatar's controls, the way a viewer would. The frontend does it (see
    /// `useAutomationRunner.ts`); the engine only says when.
    #[serde(rename_all = "camelCase")]
    Control {
        group_id: String,
        control_id: String,
        /// The control's type when it was picked, e.g. `slider`, so this side can check `run`.
        control_type: String,
        run: ControlRun
    },
    /// Makes a profile active (None: No profile), or keeps it active while the condition holds.
    #[serde(rename_all = "camelCase")]
    Profile {
        profile_id: Option<String>,
        #[serde(default)]
        while_held: bool
    },
    /// Posts to the VRChat chatbox, when the chatbox is turned on in Settings.
    Chatbox { text: String },
    StopEverything
}

/// How a step runs a control. Values are what the control's command takes (see shared-ui
/// `CommandWithoutIds`): a bool for toggles, an option index for enums, 0–1 for sliders and toys,
/// a pattern id for patterns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ControlRun {
    Set { value: serde_json::Value },
    /// Flips a toggle.
    Toggle,
    /// A toy at `value` for `ms`, then off.
    Pulse { value: f64, ms: u32 },
    /// Presses a preset or a shocker.
    Press,
    /// `value` while the condition holds, then back to what it was.
    Hold { value: serde_json::Value },
    /// A slider or toy follows the trigger, mapped onto min..max (0–1).
    Follow { min: f64, max: f64 }
}

/// Which control types each way of running fits.
fn run_fits(run: &ControlRun, control_type: &str) -> bool {
    match run {
        ControlRun::Set { .. } => !matches!(control_type, "preset" | "open-shock-shocker"),
        ControlRun::Toggle => matches!(control_type, "boolean" | "boolean-group"),
        ControlRun::Pulse { .. } => control_type == "intiface-toy",
        ControlRun::Press => matches!(control_type, "preset" | "open-shock-shocker"),
        ControlRun::Hold { .. } => {
            matches!(control_type, "boolean" | "boolean-group" | "enum" | "step-enum" | "slider" | "intiface-toy" | "intiface-pattern")
        }
        ControlRun::Follow { .. } => matches!(control_type, "slider" | "intiface-toy")
    }
}

/// The chatbox takes up to 144 characters.
pub const CHATBOX_MAX: usize = 144;
/// Longest toy pulse.
pub const MAX_TOY_PULSE_MS: u32 = 60_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ParameterAction {
    /// Sets it to a value (Bools are 0/1).
    Set { value: f64 },
    /// Flips a Bool.
    Toggle,
    /// The value while the state condition holds, then back to what it was.
    Hold { value: f64 },
    /// A Float follows the trigger, mapped onto min..max (-1–1).
    Follow { min: f64, max: f64 }
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

    pub fn runs(&self) -> Runs {
        match self {
            c if c.is_moment() => Runs::Moment,
            Condition::Any => Runs::Follow,
            _ => Runs::State
        }
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

    // A step can't set the parameter that triggers it, or it would set itself off.
    let mut trigger = None;
    match &automation.when {
        Trigger::AvatarParameter { parameter, value_type, condition, avatar_id, held_ms, .. } => {
            if avatar_id.is_empty() || parameter.is_empty() {
                return Err("Pick the avatar parameter that triggers it.".into());
            }
            if !condition.fits(*value_type) {
                return Err("That condition doesn't fit this parameter's type.".into());
            }
            if *held_ms > 0 && condition.runs() != Runs::State {
                return Err("Only a state, like \"is on\", can wait to be held.".into());
            }
            if *held_ms > MAX_HELD_MS {
                return Err(format!("It can wait up to {} s to be held.", MAX_HELD_MS / 1000));
            }
            trigger = Some(parameter.as_str());
        }
        Trigger::AvatarChanged { avatar_id, .. } => {
            if avatar_id.as_deref() == Some("") {
                return Err("Pick the avatar, or any avatar.".into());
            }
        }
        Trigger::AppEvent { .. } => {}
    }

    if automation.only_if.len() > MAX_GUARDS {
        return Err(format!("An automation can have up to {MAX_GUARDS} \"only if\" conditions."));
    }
    for guard in &automation.only_if {
        if let Guard::Parameter { parameter, value_type, condition, .. } = guard {
            if parameter.is_empty() {
                return Err("Pick the parameter each \"only if\" checks.".into());
            }
            if condition.runs() != Runs::State || !condition.fits(*value_type) {
                return Err("An \"only if\" checks a state, like \"is on\".".into());
            }
        }
    }
    let runs = automation.when.runs();

    if automation.then.is_empty() {
        return Err("Add something for it to do.".into());
    }
    if automation.then.len() > MAX_STEPS {
        return Err(format!("An automation can do up to {MAX_STEPS} things."));
    }
    for step in &automation.then {
        validate_step(step, runs, trigger)?;
    }

    Ok(())
}

/// What kind of trigger edge an action runs on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Runs {
    Moment,
    State,
    Follow
}

impl Action {
    pub fn runs(&self) -> Runs {
        match self {
            Action::BoardOutput { output: OutputAction::Hold, .. }
            | Action::AvatarParameter { set: ParameterAction::Hold { .. }, .. }
            | Action::Control { run: ControlRun::Hold { .. }, .. }
            | Action::Profile { while_held: true, .. } => Runs::State,
            Action::BoardOutput { output: OutputAction::Follow { .. }, .. }
            | Action::AvatarParameter { set: ParameterAction::Follow { .. }, .. }
            | Action::Control { run: ControlRun::Follow { .. }, .. } => Runs::Follow,
            _ => Runs::Moment
        }
    }

    /// How long a moment keeps running after it starts, for "Running" and `retrigger: ignore`.
    pub fn busy_ms(&self) -> u32 {
        match self {
            Action::BoardOutput { output, .. } => output.busy_ms().unwrap_or(0),
            Action::Control { run: ControlRun::Pulse { ms, .. }, .. } => *ms,
            _ => 0
        }
    }

    /// Run by the frontend rather than here.
    pub fn in_app(&self) -> bool {
        matches!(self, Action::Control { .. } | Action::Profile { .. } | Action::Chatbox { .. } | Action::StopEverything)
    }
}

fn fits_type(value: f64, value_type: ValueType) -> bool {
    match value_type {
        ValueType::Bool => value == 0.0 || value == 1.0,
        ValueType::Int => value.fract() == 0.0 && (0.0..=255.0).contains(&value),
        ValueType::Float => (-1.0..=1.0).contains(&value)
    }
}

fn validate_step(step: &Step, runs: Runs, trigger: Option<&str>) -> Result<(), String> {
    if step.action.runs() != runs {
        return Err(match step.action.runs() {
            Runs::Moment => "This action needs a moment, like \"turns on\".",
            Runs::State => "Hold needs a state, like \"is on\".",
            Runs::Follow => "Follow needs \"any change\" of a Float."
        }
        .into());
    }
    if step.delay_ms > MAX_DELAY_MS {
        return Err(format!("A step can wait up to {} s.", MAX_DELAY_MS / 1000));
    }
    if step.delay_ms > 0 && step.action.runs() == Runs::Follow {
        return Err("Follow can't wait: it follows the value as it changes.".into());
    }

    match &step.action {
        Action::BoardOutput { output, .. } => match output {
            OutputAction::Pulse { on_ms, count, .. } => {
                if *on_ms == 0 || *count == 0 || *count > 100 {
                    return Err("A pulse needs a length, and 1 to 100 repeats.".into());
                }
            }
            OutputAction::Follow { min, max } => {
                if !(0.0..=1.0).contains(min) || !(0.0..=1.0).contains(max) || min > max {
                    return Err("Follow's range must be within 0–100 %, low to high.".into());
                }
            }
            OutputAction::Hold | OutputAction::Toggle | OutputAction::Off => {}
        },
        Action::AvatarParameter { parameter, value_type, set } => {
            if parameter.is_empty() {
                return Err("Pick the parameter it sets.".into());
            }
            // It would set off its own trigger.
            if Some(parameter.as_str()) == trigger {
                return Err("It can't set the parameter that triggers it.".into());
            }
            match set {
                ParameterAction::Set { value } | ParameterAction::Hold { value } => {
                    if !fits_type(*value, *value_type) {
                        return Err("That value doesn't fit the parameter's type.".into());
                    }
                }
                ParameterAction::Toggle => {
                    if *value_type != ValueType::Bool {
                        return Err("Only a Bool can be toggled.".into());
                    }
                }
                ParameterAction::Follow { min, max } => {
                    if *value_type != ValueType::Float {
                        return Err("Only a Float can follow a value.".into());
                    }
                    if !fits_type(*min, ValueType::Float) || !fits_type(*max, ValueType::Float) {
                        return Err("Follow's range must be within -1 and 1.".into());
                    }
                }
            }
        }
        Action::Control { group_id, control_id, control_type, run } => {
            if group_id.is_empty() || control_id.is_empty() {
                return Err("Pick the control it runs.".into());
            }
            if !run_fits(run, control_type) {
                return Err("That doesn't fit this kind of control.".into());
            }
            match run {
                ControlRun::Pulse { value, ms } => {
                    if !(0.0..=1.0).contains(value) || *ms == 0 || *ms > MAX_TOY_PULSE_MS {
                        return Err(format!("A toy pulse needs an intensity and up to {} s.", MAX_TOY_PULSE_MS / 1000));
                    }
                }
                ControlRun::Follow { min, max } => {
                    if !(0.0..=1.0).contains(min) || !(0.0..=1.0).contains(max) {
                        return Err("Follow's range must be within 0–100 %.".into());
                    }
                }
                _ => {}
            }
        }
        Action::Profile { .. } | Action::StopEverything => {}
        Action::Chatbox { text } => {
            if text.trim().is_empty() {
                return Err("Write the chatbox message.".into());
            }
            if text.chars().count() > CHATBOX_MAX {
                return Err(format!("The chatbox takes up to {CHATBOX_MAX} characters."));
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
                condition,
                any_avatar: false,
                held_ms: 0
            },
            then: vec![Step { action: Action::BoardOutput { board_id: "a1b2c3".into(), pin: 4, output }, delay_ms: 0 }],
            cooldown_ms: 0,
            retrigger: Retrigger::Restart,            only_if: Vec::new()
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
        assert_eq!(json["then"][0]["kind"], "board-output");
        assert_eq!(json["then"][0]["boardId"], "a1b2c3");
        assert_eq!(json["then"][0]["output"]["type"], "pulse");
        assert_eq!(json["then"][0]["output"]["onMs"], 2000);
        assert_eq!(json["then"][0]["delayMs"], 0);
        assert_eq!(json["retrigger"], "restart");
    }

    #[test]
    fn loads_a_single_action() {
        let json = serde_json::json!({
            "id": "a",
            "name": "Boop fan",
            "enabled": true,
            "when": {
                "kind": "avatar-parameter",
                "avatarId": "avtr_1",
                "avatarName": "Avatar",
                "parameter": "/avatar/parameters/Boop",
                "valueType": "Bool",
                "condition": { "type": "turns-on" }
            },
            "then": { "kind": "board-output", "boardId": "a1b2c3", "pin": 4, "output": { "type": "toggle" } }
        });
        let automation: Automation = serde_json::from_value(json).unwrap();
        assert_eq!(
            automation.then,
            vec![Step { action: Action::BoardOutput { board_id: "a1b2c3".into(), pin: 4, output: OutputAction::Toggle }, delay_ms: 0 }]
        );
    }

    fn parameter_step(parameter: &str, value_type: ValueType, set: ParameterAction) -> Step {
        Step { action: Action::AvatarParameter { parameter: parameter.into(), value_type, set }, delay_ms: 0 }
    }

    #[test]
    fn parameter_steps() {
        let mut a = sample(Condition::TurnsOn, ValueType::Bool, OutputAction::Toggle);
        a.then.push(parameter_step("/avatar/parameters/Ears", ValueType::Bool, ParameterAction::Toggle));
        assert!(validate(&a).is_ok());

        a.then[1] = parameter_step("/avatar/parameters/Boop", ValueType::Bool, ParameterAction::Toggle);
        assert!(validate(&a).is_err(), "sets its own trigger");

        a.then[1] = parameter_step("/avatar/parameters/Outfit", ValueType::Int, ParameterAction::Toggle);
        assert!(validate(&a).is_err(), "toggles an Int");

        a.then[1] = parameter_step("/avatar/parameters/Outfit", ValueType::Int, ParameterAction::Set { value: 3.0 });
        assert!(validate(&a).is_ok());

        a.then[1] = parameter_step("/avatar/parameters/Outfit", ValueType::Int, ParameterAction::Hold { value: 3.0 });
        assert!(validate(&a).is_err(), "hold on a moment");

        let mut follow = sample(Condition::Any, ValueType::Float, OutputAction::Follow { min: 0.0, max: 1.0 });
        follow.then.push(parameter_step("/avatar/parameters/Glow", ValueType::Float, ParameterAction::Follow { min: -1.0, max: 1.0 }));
        assert!(validate(&follow).is_ok());
        follow.then[1].delay_ms = 500;
        assert!(validate(&follow).is_err(), "a follow can't wait");
    }

    fn control_step(control_type: &str, run: ControlRun) -> Step {
        Step {
            action: Action::Control { group_id: "g".into(), control_id: "c".into(), control_type: control_type.into(), run },
            delay_ms: 0
        }
    }

    #[test]
    fn app_steps() {
        let mut a = sample(Condition::TurnsOn, ValueType::Bool, OutputAction::Toggle);
        a.then = vec![control_step("preset", ControlRun::Press)];
        assert!(validate(&a).is_ok());
        a.then = vec![control_step("slider", ControlRun::Press)];
        assert!(validate(&a).is_err(), "press a slider");
        a.then = vec![control_step("intiface-toy", ControlRun::Pulse { value: 0.5, ms: 0 })];
        assert!(validate(&a).is_err(), "pulse without a length");
        a.then = vec![control_step("boolean", ControlRun::Hold { value: serde_json::json!(true) })];
        assert!(validate(&a).is_err(), "hold on a moment");
        a.then = vec![Step { action: Action::Chatbox { text: "x".repeat(CHATBOX_MAX + 1) }, delay_ms: 0 }];
        assert!(validate(&a).is_err(), "chatbox too long");
        a.then = vec![Step { action: Action::Profile { profile_id: None, while_held: true }, delay_ms: 0 }];
        assert!(validate(&a).is_err(), "profile hold on a moment");

        let mut held = sample(Condition::IsOn, ValueType::Bool, OutputAction::Hold);
        held.then = vec![
            control_step("intiface-pattern", ControlRun::Hold { value: serde_json::json!("pulse") }),
            Step { action: Action::Profile { profile_id: Some("p".into()), while_held: true }, delay_ms: 0 }
        ];
        assert!(validate(&held).is_ok());
        held.then.push(Step { action: Action::StopEverything, delay_ms: 0 });
        assert!(validate(&held).is_err(), "stop everything on a state");
    }

    #[test]
    fn triggers() {
        let mut held = sample(Condition::IsOn, ValueType::Bool, OutputAction::Hold);
        if let Trigger::AvatarParameter { held_ms, .. } = &mut held.when {
            *held_ms = 2000;
        }
        assert!(validate(&held).is_ok());

        let mut moment = sample(Condition::TurnsOn, ValueType::Bool, OutputAction::Toggle);
        if let Trigger::AvatarParameter { held_ms, .. } = &mut moment.when {
            *held_ms = 2000;
        }
        assert!(validate(&moment).is_err(), "a moment can't be held");

        let mut changed = sample(Condition::TurnsOn, ValueType::Bool, OutputAction::Toggle);
        changed.when = Trigger::AvatarChanged { avatar_id: None, avatar_name: None };
        assert!(validate(&changed).is_ok());
        changed.then[0] = Step { action: Action::BoardOutput { board_id: "b".into(), pin: 4, output: OutputAction::Hold }, delay_ms: 0 };
        assert!(validate(&changed).is_err(), "an avatar change is a moment");
    }

    #[test]
    fn step_count_and_delay() {
        let mut a = sample(Condition::TurnsOn, ValueType::Bool, OutputAction::Toggle);
        a.then[0].delay_ms = MAX_DELAY_MS + 1;
        assert!(validate(&a).is_err());
        a.then[0].delay_ms = 0;
        a.then = vec![a.then[0].clone(); MAX_STEPS + 1];
        assert!(validate(&a).is_err());
        a.then.clear();
        assert!(validate(&a).is_err());
    }
}
