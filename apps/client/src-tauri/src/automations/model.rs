//! What an automation is: *when* something happens, *do* something. Both sides are tagged unions
//! (`kind`) so new trigger and action kinds slot in without reshaping saved data. Mirrored by hand
//! in the frontend's `lib/tauri-bridge.ts` and `utils/automations.ts`.
use serde::{Deserialize, Deserializer, Serialize};

/// Most steps one automation can run.
pub const MAX_STEPS: usize = 8;
/// Longest "after" delay on a step.
pub const MAX_DELAY_MS: u32 = 300_000;

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
    /// All of these have to be true for it to start, or with `only_if_any` one of them.
    #[serde(default)]
    pub only_if: Vec<Guard>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub only_if_any: bool,
    /// Set while it's an assistant's draft.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<crate::types::CreatedBy>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Retrigger {
    #[default]
    Restart,
    Ignore
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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
        held_ms: u32,
        /// More parameters of the same type that start it too. Moments only.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        also: Vec<String>
    },
    /// You start wearing an avatar: this one, or any (None). Also when the app first sees it.
    #[serde(rename_all = "camelCase")]
    AvatarChanged { avatar_id: Option<String>, avatar_name: Option<String> },
    /// Something happens in the app: viewers, Stop everything. The frontend reports these
    /// (`automation_event`).
    AppEvent { event: AppEvent },
    /// Something happens to the avatar leash. Its add-on reports these.
    Leash { event: LeashEvent },
    /// Every `minutes` while the app is open, counted from when it starts or is saved.
    Every { minutes: u32 },
    /// A toy connects, disconnects or runs low (the Intiface add-on).
    ToyEvent { event: ToyEvent },
    /// A board comes online or goes offline: this one, or any (the ESP32 boards add-on).
    #[serde(rename_all = "camelCase")]
    BoardEvent {
        event: BoardEvent,
        #[serde(default)]
        board_id: Option<String>
    },
    /// A shock actually goes out (the OpenShock add-on): not when a cooldown or a failed check drops
    /// it.
    ShockSent,
    /// Avatar touch starts or stops driving your toys or gear (the avatar haptics add-on).
    Touch { event: TouchEvent },
    /// Another automation starts, or with `when_stops`, stops running. Chains stop after
    /// `MAX_CHAIN` links so two automations can't keep starting each other.
    #[serde(rename_all = "camelCase")]
    AfterAutomation {
        automation_id: String,
        #[serde(default)]
        when_stops: bool
    }
}

/// How many automations in a row can start each other from one event.
pub const MAX_CHAIN: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ToyEvent {
    Connected,
    Disconnected,
    /// Its battery drops under `LOW_BATTERY`.
    BatteryLow
}

/// A toy's battery counts as low under this (0–1).
pub const LOW_BATTERY: f64 = 0.15;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum BoardEvent {
    Online,
    Offline
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TouchEvent {
    Starts,
    Stops
}

/// Longest gap of an `every` trigger: a day.
pub const MAX_EVERY_MINUTES: u32 = 24 * 60;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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
    Resumed,
    /// Stop everything is turned on. Only chatbox steps run on it, right away.
    Stopped,
    /// A viewer's command was held back by a lock, a limit or a cooldown.
    ViewerHeldBack
}

/// What happens to the avatar leash (see `crate::leash`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum LeashEvent {
    /// Someone grabs a leash that moves you.
    Grabbed,
    /// The leash is pulled past the run point.
    Running,
    /// The leash is let go.
    Released
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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
    /// At least `at_least` viewers (1 when 0) are on the share page, or, with `online` false, none.
    #[serde(rename_all = "camelCase")]
    Viewers {
        online: bool,
        #[serde(default, skip_serializing_if = "is_zero")]
        at_least: u32
    },
    /// The local time is within from..to (minutes of the day, across midnight when to < from), on
    /// these days (0 = Monday; empty = every day).
    Time {
        from: u16,
        to: u16,
        #[serde(default)]
        days: Vec<u8>
    },
    /// You're wearing this avatar, or with `not`, any other.
    #[serde(rename_all = "camelCase")]
    Wearing {
        avatar_id: String,
        avatar_name: String,
        #[serde(default)]
        not: bool
    },
    /// A dice roll: true `percent` times out of 100.
    Chance { percent: u8 },
    /// The viewer whose command came last is in this viewer group, or with `not`, isn't.
    #[serde(rename_all = "camelCase")]
    ViewerGroup {
        group_id: String,
        #[serde(default)]
        not: bool
    },
    /// A toy is connected to Intiface, or none is. Checked by the engine (see `Env`).
    ToysConnected { connected: bool },
    /// This board is online, or isn't. Checked by the engine (see `Env`).
    #[serde(rename_all = "camelCase")]
    BoardOnline { board_id: String, online: bool },
    /// Another automation is running, or isn't. Checked by the engine, which knows.
    #[serde(rename_all = "camelCase")]
    AutomationRunning { automation_id: String, running: bool }
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// Most "only if" conditions on one automation.
pub const MAX_GUARDS: usize = 4;

/// The clock and a dice roll, for time and chance guards.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Now {
    /// 0 = Monday.
    pub weekday: u8,
    /// Minutes since local midnight.
    pub minute: u16,
    /// 0–1.
    pub roll: f64
}

impl Guard {
    /// `value` looks up a parameter's current value.
    pub fn holds(&self, context: &Context, now: Now, value: &dyn Fn(&str) -> Option<f64>) -> bool {
        match self {
            Guard::Parameter { parameter, condition, not, .. } => value(parameter).is_some_and(|v| condition.holds(v)) != *not,
            Guard::Profile { profile_id } => context.profile_id == *profile_id,
            Guard::Viewers { online: true, at_least } => context.viewers >= (*at_least).max(1),
            Guard::Viewers { online: false, .. } => context.viewers == 0,
            Guard::Time { from, to, days } => {
                let within = if from <= to { (*from..*to).contains(&now.minute) } else { now.minute >= *from || now.minute < *to };
                within && (days.is_empty() || days.contains(&now.weekday))
            }
            Guard::Wearing { avatar_id, not, .. } => (context.avatar_id.as_deref() == Some(avatar_id.as_str())) != *not,
            Guard::Chance { percent } => now.roll * 100.0 < f64::from(*percent),
            Guard::ViewerGroup { group_id, not } => context.viewer_groups.contains(group_id) != *not,
            // The engine asks the app about devices before it gets here.
            Guard::ToysConnected { .. } | Guard::BoardOnline { .. } | Guard::AutomationRunning { .. } => true
        }
    }
}

/// What guards need: the active profile (see `setup`), how many viewers are on (from the webview)
/// and the avatar you're wearing.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    pub profile_id: Option<String>,
    pub viewers: u32,
    #[serde(default)]
    pub avatar_id: Option<String>,
    /// The viewer groups of whoever sent the last viewer command.
    #[serde(default)]
    pub viewer_groups: Vec<String>
}

/// Longest "held for" on a state.
pub const MAX_HELD_MS: u32 = 60_000;

impl Trigger {
    /// What kind of steps it can run.
    pub fn runs(&self) -> Runs {
        match self {
            Trigger::AvatarParameter { condition, .. } => condition.runs(),
            _ => Runs::Moment
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum ValueType {
    Bool,
    Int,
    Float
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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
    Any,
    /// A Float drops under `value`.
    FallsBelow { value: f64 },
    /// A Float is within min..=max.
    IsBetween { min: f64, max: f64 },
    /// A Bool turns on `count` times within `within_ms` (see `engine`, which keeps the taps).
    #[serde(rename_all = "camelCase")]
    Taps { count: u32, within_ms: u32 },
    /// A Float has stopped changing for `ms` (see `engine`, which keeps the clock).
    StopsChanging { ms: u32 }
}

/// Most taps, and the longest window or quiet time.
pub const MAX_TAPS: u32 = 10;
pub const MAX_TAP_WINDOW_MS: u32 = 10_000;
/// Most extra parameters an avatar parameter trigger also listens to.
pub const MAX_ALSO: usize = 8;

/// One thing an automation does: an action, plus how long after the trigger it starts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    #[serde(flatten)]
    pub action: Action,
    #[serde(default)]
    pub delay_ms: u32,
    /// The delay counts from when the step before it is done, not from the trigger.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub after_previous: bool
}

/// When each step starts, in ms after the trigger: `after_previous` ones wait for the step before
/// to be done (its own delay plus how long it runs).
pub fn step_starts(steps: &[Step]) -> Vec<u32> {
    let mut previous_end = 0u32;
    steps
        .iter()
        .map(|step| {
            let start = if step.after_previous { previous_end.saturating_add(step.delay_ms) } else { step.delay_ms };
            previous_end = start.saturating_add(step.action.busy_ms());
            start
        })
        .collect()
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Action {
    #[serde(rename_all = "camelCase")]
    BoardOutput {
        board_id: String,
        pin: u8,
        output: OutputAction,
        /// Also says so in the chatbox (see `chatbox_text`).
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        chatbox: bool
    },
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
    StopEverything,
    /// Turns "Leash moves me" on or off, like `TKOSC/Leash`.
    LeashEnabled { enabled: bool },
    /// Plays motors of one bHaptics position (all of them when `motors` is empty) for `ms`.
    #[serde(rename_all = "camelCase")]
    BhapticsPulse {
        position: crate::bhaptics::model::Position,
        #[serde(default)]
        motors: Vec<usize>,
        /// 0–1.
        intensity: f64,
        ms: u32
    },
    /// Plays one of the built-in bHaptics effects across `positions`.
    #[serde(rename_all = "camelCase")]
    BhapticsEffect {
        effect: String,
        positions: Vec<crate::bhaptics::model::Position>,
        /// 0–1.
        intensity: f64
    },
    /// Plays one of the built-in OWO sensations on `muscles`.
    #[serde(rename_all = "camelCase")]
    OwoSensation {
        sensation: String,
        muscles: Vec<crate::owo::model::Muscle>,
        /// 0–1.
        intensity: f64
    },
    /// Drives toys straight from Intiface, without a control: these motors, or every toy
    /// connected when `toys` is empty. Acts as you, so profile limits don't apply.
    Toy {
        #[serde(default)]
        toys: Vec<ToyMotor>,
        run: ToyRun
    },
    /// Plays a built-in or saved pattern on toys (all when `toys` is empty), for `ms` or, with
    /// `while_held`, while the condition holds.
    #[serde(rename_all = "camelCase")]
    ToyPattern {
        #[serde(default)]
        toys: Vec<ToyMotor>,
        pattern_id: String,
        #[serde(default)]
        ms: u32,
        #[serde(default)]
        while_held: bool
    },
    /// Stops every toy and pattern, without pausing anything else.
    StopToys,
    /// Turns another automation on or off.
    #[serde(rename_all = "camelCase")]
    AutomationEnabled { automation_id: String, enabled: bool },
    /// Shows a message in VR (the overlay add-on). Takes the chatbox's placeholders.
    VrToast { text: String },
    /// Runs one of these moments, picked at random each time.
    OneOf { options: Vec<Action> },
    /// Sends any OSC message to VRChat, e.g. `/input/Jump`. Nothing checks what it does, so
    /// assistants aren't offered it.
    #[schemars(skip)]
    OscMessage { address: String, value: crate::types::OscArg }
}

/// Most options of a "one of these" step.
pub const MAX_ONE_OF: usize = 6;
/// What chatbox and VR toast text can say about what started it (see `mod.rs` `fill`).
pub const PLACEHOLDERS: [&str; 4] = ["{viewer}", "{control}", "{value}", "{avatar}"];

/// One toy motor, saved the way toy controls save it (see shared-ui `IntifaceActuatorRef`): the
/// names are only shown while the toy is off.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToyMotor {
    pub device_index: u32,
    #[serde(default)]
    pub device_name: String,
    pub actuator_index: u32,
    #[serde(default)]
    pub actuator_description: String,
    /// Buttplug's `Vibrate`, `Rotate`, ... sent back as it is.
    pub actuator_type: String
}

/// How a Toy step drives its toys. Levels are 0–1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ToyRun {
    /// `level` for `ms`, then off.
    Pulse { level: f64, ms: u32 },
    /// `level` while the condition holds, then off.
    Hold { level: f64 },
    /// Follows the trigger, mapped onto min..max.
    Follow { min: f64, max: f64 },
    /// Fades from `from` to `to` over `ms`, and stays at `to`.
    Ramp { from: f64, to: f64, ms: u32 }
}

/// How a step runs a control. Values are what the control's command takes (see shared-ui
/// `CommandWithoutIds`): a bool for toggles, an option index for enums, 0–1 for sliders and toys,
/// a pattern id for patterns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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
/// Longest a pattern step plays, when it isn't held.
pub const MAX_TOY_PATTERN_MS: u32 = 600_000;
/// The longest a bHaptics pulse step plays.
pub const MAX_BHAPTICS_PULSE_MS: u32 = 30_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ParameterAction {
    /// Sets it to a value (Bools are 0/1).
    Set { value: f64 },
    /// Flips a Bool.
    Toggle,
    /// The value while the state condition holds, then back to what it was.
    Hold { value: f64 },
    /// A Float follows the trigger, mapped onto min..max (-1–1).
    Follow { min: f64, max: f64 },
    /// Adds `amount` (negative to take away) to an Int or a Float, kept within its range.
    Add { amount: f64 }
}

impl ValueType {
    /// What a parameter of this type can hold.
    pub fn clamp(self, value: f64) -> f64 {
        match self {
            ValueType::Bool => value.clamp(0.0, 1.0).round(),
            ValueType::Int => value.round().clamp(0.0, 255.0),
            ValueType::Float => value.clamp(-1.0, 1.0)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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
        matches!(
            self,
            Condition::TurnsOn
                | Condition::TurnsOff
                | Condition::Becomes { .. }
                | Condition::RisesAbove { .. }
                | Condition::FallsBelow { .. }
                | Condition::Taps { .. }
                | Condition::StopsChanging { .. }
        )
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
            Condition::TurnsOn | Condition::TurnsOff | Condition::IsOn | Condition::Taps { .. } => value_type == ValueType::Bool,
            Condition::Becomes { .. } | Condition::Is { .. } => value_type == ValueType::Int,
            Condition::RisesAbove { .. }
            | Condition::IsAbove { .. }
            | Condition::Any
            | Condition::FallsBelow { .. }
            | Condition::IsBetween { .. }
            | Condition::StopsChanging { .. } => value_type == ValueType::Float
        }
    }

    /// For state conditions: whether `value` satisfies it.
    fn holds(&self, value: f64) -> bool {
        match self {
            Condition::IsOn => value >= ON,
            Condition::Is { value: target } => same_int(value, *target),
            Condition::IsAbove { value: threshold } => value > *threshold,
            Condition::IsBetween { min, max } => (*min..=*max).contains(&value),
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
            Condition::FallsBelow { value } => edge(current < *value && previous.is_some_and(|p| p >= *value)),
            // Kept by the engine, which counts taps and quiet time.
            Condition::Taps { .. } | Condition::StopsChanging { .. } => Edge::None,
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

/// What a `chatbox` output step says.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoardNote {
    Pulse { on_ms: u32, count: u32 },
    On,
    Off
}

/// E.g. `0.5s`, `2s`, `1m 30s`.
pub fn format_duration(ms: u32) -> String {
    if ms >= 60_000 {
        let seconds = (ms + 500) / 1000;
        return match seconds % 60 {
            0 => format!("{}m", seconds / 60),
            rest => format!("{}m {rest}s", seconds / 60)
        };
    }
    let tenths = (ms + 50) / 100;
    if tenths % 10 == 0 {
        format!("{}s", tenths / 10)
    } else {
        format!("{}.{}s", tenths / 10, tenths % 10)
    }
}

/// The chatbox message for an output, named by its label in Settings › ESP32 boards.
pub fn chatbox_text(label: &str, note: BoardNote) -> String {
    let text = match note {
        BoardNote::Pulse { on_ms, count: 0 | 1 } => format!("{label} activated for {}", format_duration(on_ms)),
        BoardNote::Pulse { on_ms, count } => format!("{label} activated for {}, {count} times", format_duration(on_ms)),
        BoardNote::On => format!("{label} activated"),
        BoardNote::Off => format!("{label} turned off")
    };
    text.chars().take(CHATBOX_MAX).collect()
}

/// Checks what can be checked without knowing the boards. Returns a message to show.
pub fn validate(automation: &Automation) -> Result<(), String> {
    if automation.name.trim().is_empty() {
        return Err("Give the automation a name.".into());
    }

    // A step can't set the parameter that triggers it, or it would set itself off.
    let mut trigger = None;
    match &automation.when {
        Trigger::AvatarParameter { parameter, value_type, condition, avatar_id, held_ms, also, .. } => {
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
            if !also.is_empty() && (!condition.is_moment() || also.len() > MAX_ALSO || also.iter().any(|a| a.is_empty() || a == parameter)) {
                return Err(format!("Up to {MAX_ALSO} more parameters, each different, and only for a moment like \"turns on\"."));
            }
            match condition {
                Condition::Taps { count, within_ms } if !(2..=MAX_TAPS).contains(count) || !(1..=MAX_TAP_WINDOW_MS).contains(within_ms) => {
                    return Err(format!("Taps count 2 to {MAX_TAPS} times, within up to {} s.", MAX_TAP_WINDOW_MS / 1000));
                }
                Condition::StopsChanging { ms } if !(100..=MAX_HELD_MS).contains(ms) => {
                    return Err(format!("It can wait from 0.1 to {} s for it to stop changing.", MAX_HELD_MS / 1000));
                }
                Condition::IsBetween { min, max } if min >= max => return Err("The low end has to be under the high end.".into()),
                _ => {}
            }
            trigger = Some(parameter.as_str());
        }
        Trigger::AvatarChanged { avatar_id, .. } => {
            if avatar_id.as_deref() == Some("") {
                return Err("Pick the avatar, or any avatar.".into());
            }
        }
        Trigger::Every { minutes } => {
            if !(1..=MAX_EVERY_MINUTES).contains(minutes) {
                return Err("It can run every minute up to once a day.".into());
            }
        }
        // Everything else is stopped: only the chatbox can still say so.
        Trigger::AppEvent { event: AppEvent::Stopped } => {
            if automation.then.iter().any(|step| !matches!(step.action, Action::Chatbox { .. }) || step.delay_ms > 0) {
                return Err("When Stop everything turns on, only the chatbox can say something, right away.".into());
            }
        }
        // A shock step would set itself off again.
        Trigger::ShockSent => {
            if automation.then.iter().any(|step| matches!(&step.action, Action::Control { control_type, .. } if control_type == "open-shock-shocker")) {
                return Err("When a shock goes out, it can't send another one.".into());
            }
        }
        Trigger::AfterAutomation { automation_id, .. } => {
            if automation_id.is_empty() || *automation_id == automation.id {
                return Err("Pick another automation for it to follow.".into());
            }
        }
        Trigger::AppEvent { .. } | Trigger::Leash { .. } | Trigger::ToyEvent { .. } | Trigger::BoardEvent { .. } | Trigger::Touch { .. } => {}
    }

    if automation.only_if.len() > MAX_GUARDS {
        return Err(format!("An automation can have up to {MAX_GUARDS} \"only if\" conditions."));
    }
    for guard in &automation.only_if {
        match guard {
            Guard::Parameter { parameter, value_type, condition, .. } => {
                if parameter.is_empty() {
                    return Err("Pick the parameter each \"only if\" checks.".into());
                }
                if condition.runs() != Runs::State || !condition.fits(*value_type) {
                    return Err("An \"only if\" checks a state, like \"is on\".".into());
                }
            }
            Guard::Time { from, to, days } => {
                if *from >= 24 * 60 || *to >= 24 * 60 || from == to || days.iter().any(|day| *day > 6) {
                    return Err("Pick a start and an end time that aren't the same.".into());
                }
            }
            Guard::Wearing { avatar_id, .. } => {
                if avatar_id.is_empty() {
                    return Err("Pick the avatar.".into());
                }
            }
            Guard::Chance { percent } => {
                if !(1..=99).contains(percent) {
                    return Err("A chance goes from 1 to 99 %.".into());
                }
            }
            Guard::ViewerGroup { group_id: id, .. } | Guard::BoardOnline { board_id: id, .. } | Guard::AutomationRunning { automation_id: id, .. } => {
                if id.is_empty() {
                    return Err("Pick what each \"only if\" checks.".into());
                }
            }
            Guard::Profile { .. } | Guard::Viewers { .. } | Guard::ToysConnected { .. } => {}
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
        if matches!(&step.action, Action::AutomationEnabled { automation_id, .. } if *automation_id == automation.id) {
            return Err("It can't turn itself on or off.".into());
        }
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
            | Action::Profile { while_held: true, .. }
            | Action::Toy { run: ToyRun::Hold { .. }, .. }
            | Action::ToyPattern { while_held: true, .. } => Runs::State,
            Action::BoardOutput { output: OutputAction::Follow { .. }, .. }
            | Action::AvatarParameter { set: ParameterAction::Follow { .. }, .. }
            | Action::Control { run: ControlRun::Follow { .. }, .. }
            | Action::Toy { run: ToyRun::Follow { .. }, .. } => Runs::Follow,
            _ => Runs::Moment
        }
    }

    /// How long a moment keeps running after it starts, for "Running" and `retrigger: ignore`.
    pub fn busy_ms(&self) -> u32 {
        match self {
            Action::BoardOutput { output, .. } => output.busy_ms().unwrap_or(0),
            Action::Control { run: ControlRun::Pulse { ms, .. }, .. } => *ms,
            Action::BhapticsPulse { ms, .. } => *ms,
            Action::Toy { run: ToyRun::Pulse { ms, .. } | ToyRun::Ramp { ms, .. }, .. } => *ms,
            Action::ToyPattern { ms, while_held: false, .. } => *ms,
            Action::OneOf { options } => options.iter().map(Action::busy_ms).max().unwrap_or(0),
            _ => 0
        }
    }

    /// Run outside the engine, as an `AppRun` (see `mod.rs` `apply`), with a `stop` for holds.
    pub fn in_app(&self) -> bool {
        matches!(
            self,
            Action::Control { .. }
                | Action::Profile { .. }
                | Action::Chatbox { .. }
                | Action::StopEverything
                | Action::Toy { .. }
                | Action::ToyPattern { .. }
                | Action::StopToys
                | Action::AutomationEnabled { .. }
                | Action::VrToast { .. }
        )
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
        return Err(format!("A step can wait up to {} minutes.", MAX_DELAY_MS / 60_000));
    }
    if (step.delay_ms > 0 || step.after_previous) && step.action.runs() == Runs::Follow {
        return Err("Follow can't wait: it follows the value as it changes.".into());
    }

    match &step.action {
        Action::BoardOutput { output, chatbox, .. } => match output {
            OutputAction::Follow { .. } if *chatbox => {
                return Err("Follow can't post to the chatbox: it changes all the time.".into());
            }
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
                ParameterAction::Add { amount } => {
                    let fits = match value_type {
                        ValueType::Bool => false,
                        ValueType::Int => amount.fract() == 0.0 && (-255.0..=255.0).contains(amount),
                        ValueType::Float => (-2.0..=2.0).contains(amount)
                    };
                    if *amount == 0.0 || !fits {
                        return Err("Add a whole number to an Int, or up to ±2 to a Float. A Bool can only be set or toggled.".into());
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
        Action::Profile { .. } | Action::StopEverything | Action::LeashEnabled { .. } => {}
        Action::BhapticsPulse { position, motors, intensity, ms } => {
            if !(0.0..=1.0).contains(intensity) || *ms == 0 || *ms > MAX_BHAPTICS_PULSE_MS {
                return Err(format!("A bHaptics pulse needs an intensity and up to {} s.", MAX_BHAPTICS_PULSE_MS / 1000));
            }
            if motors.iter().any(|motor| *motor >= position.motors()) {
                return Err("That motor isn't on this device.".into());
            }
        }
        Action::BhapticsEffect { effect, positions, intensity } => {
            if !crate::bhaptics::effects::exists(effect) {
                return Err("Pick an effect.".into());
            }
            if positions.is_empty() {
                return Err("Pick where it plays.".into());
            }
            if !(0.0..=1.0).contains(intensity) {
                return Err("The intensity must be within 0–100 %.".into());
            }
        }
        Action::OwoSensation { sensation, muscles, intensity } => {
            if !crate::owo::sensations::exists(sensation) {
                return Err("Pick a sensation.".into());
            }
            if muscles.is_empty() {
                return Err("Pick where it plays.".into());
            }
            if !(0.0..=1.0).contains(intensity) {
                return Err("The intensity must be within 0–100 %.".into());
            }
        }
        Action::Toy { run, .. } => {
            let level = |l: &f64| (0.0..=1.0).contains(l);
            let ok = match run {
                ToyRun::Pulse { level: l, ms } => level(l) && (1..=MAX_TOY_PULSE_MS).contains(ms),
                ToyRun::Hold { level: l } => level(l),
                ToyRun::Follow { min, max } => level(min) && level(max),
                ToyRun::Ramp { from, to, ms } => level(from) && level(to) && (1..=MAX_TOY_PULSE_MS).contains(ms)
            };
            if !ok {
                return Err(format!("Toy levels go from 0 to 100 %, and a pulse or ramp lasts up to {} s.", MAX_TOY_PULSE_MS / 1000));
            }
        }
        Action::ToyPattern { pattern_id, ms, while_held, .. } => {
            if pattern_id.is_empty() || pattern_id == crate::toys::patterns::OFF_ID {
                return Err("Pick a pattern.".into());
            }
            if !while_held && !(1..=MAX_TOY_PATTERN_MS).contains(ms) {
                return Err(format!("A pattern plays for up to {} minutes.", MAX_TOY_PATTERN_MS / 60_000));
            }
        }
        Action::StopToys => {}
        Action::AutomationEnabled { automation_id, .. } => {
            if automation_id.is_empty() {
                return Err("Pick the automation it turns on or off.".into());
            }
        }
        Action::VrToast { text } => {
            if text.trim().is_empty() || text.chars().count() > CHATBOX_MAX {
                return Err(format!("Write the message, up to {CHATBOX_MAX} characters."));
            }
        }
        Action::OneOf { options } => {
            if !(2..=MAX_ONE_OF).contains(&options.len()) {
                return Err(format!("\"One of these\" picks from 2 to {MAX_ONE_OF} things."));
            }
            for option in options {
                if matches!(option, Action::OneOf { .. }) {
                    return Err("\"One of these\" can't hold another one.".into());
                }
                validate_step(&Step { action: option.clone(), delay_ms: 0, after_previous: false }, Runs::Moment, trigger)?;
            }
        }
        Action::OscMessage { address, value } => {
            if !address.starts_with('/') || address.len() < 2 || address.contains(char::is_whitespace) {
                return Err("An OSC address starts with / and has no spaces, like /input/Jump.".into());
            }
            if matches!(value, crate::types::OscArg::Str(text) if text.chars().count() > CHATBOX_MAX) {
                return Err(format!("An OSC text value takes up to {CHATBOX_MAX} characters."));
            }
        }
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
                held_ms: 0,
                also: Vec::new()
            },
            then: vec![Step { action: Action::BoardOutput { board_id: "a1b2c3".into(), pin: 4, output, chatbox: false }, delay_ms: 0, after_previous: false }],
            cooldown_ms: 0,
            retrigger: Retrigger::Restart,
            only_if: Vec::new(),
            only_if_any: false,
            created_by: None
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
    fn falls_below_and_between() {
        assert_eq!(Condition::FallsBelow { value: 0.3 }.evaluate(Some(0.5), 0.2), Edge::Fire);
        assert_eq!(Condition::FallsBelow { value: 0.3 }.evaluate(None, 0.2), Edge::None, "not on first sight");
        assert_eq!(Condition::IsBetween { min: 0.2, max: 0.6 }.evaluate(Some(0.1), 0.4), Edge::Enter);
        assert_eq!(Condition::IsBetween { min: 0.2, max: 0.6 }.evaluate(Some(0.4), 0.7), Edge::Exit);
        assert!(validate(&sample(Condition::IsBetween { min: 0.6, max: 0.2 }, ValueType::Float, OutputAction::Hold)).is_err());
        assert!(validate(&sample(Condition::Taps { count: 1, within_ms: 1000 }, ValueType::Bool, OutputAction::Toggle)).is_err());
        assert!(validate(&sample(Condition::Taps { count: 3, within_ms: 1000 }, ValueType::Bool, OutputAction::Toggle)).is_ok());
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
            vec![Step { action: Action::BoardOutput { board_id: "a1b2c3".into(), pin: 4, output: OutputAction::Toggle, chatbox: false }, delay_ms: 0, after_previous: false }]
        );
    }

    fn parameter_step(parameter: &str, value_type: ValueType, set: ParameterAction) -> Step {
        Step { action: Action::AvatarParameter { parameter: parameter.into(), value_type, set }, delay_ms: 0, after_previous: false }
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
            delay_ms: 0,
            after_previous: false
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
        a.then = vec![Step { action: Action::Chatbox { text: "x".repeat(CHATBOX_MAX + 1) }, delay_ms: 0, after_previous: false }];
        assert!(validate(&a).is_err(), "chatbox too long");
        a.then = vec![Step { action: Action::Profile { profile_id: None, while_held: true }, delay_ms: 0, after_previous: false }];
        assert!(validate(&a).is_err(), "profile hold on a moment");

        let mut held = sample(Condition::IsOn, ValueType::Bool, OutputAction::Hold);
        held.then = vec![
            control_step("intiface-pattern", ControlRun::Hold { value: serde_json::json!("pulse") }),
            Step { action: Action::Profile { profile_id: Some("p".into()), while_held: true }, delay_ms: 0, after_previous: false }
        ];
        assert!(validate(&held).is_ok());
        held.then.push(Step { action: Action::StopEverything, delay_ms: 0, after_previous: false });
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
        changed.then[0] = Step { action: Action::BoardOutput { board_id: "b".into(), pin: 4, output: OutputAction::Hold, chatbox: false }, delay_ms: 0, after_previous: false };
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

    #[test]
    fn output_chatbox() {
        let mut a = sample(Condition::TurnsOn, ValueType::Bool, OutputAction::Toggle);
        let json = serde_json::to_value(&a).unwrap();
        assert!(json["then"][0].get("chatbox").is_none(), "left out when off");
        if let Action::BoardOutput { chatbox, .. } = &mut a.then[0].action {
            *chatbox = true;
        }
        assert!(validate(&a).is_ok());
        assert_eq!(serde_json::to_value(&a).unwrap()["then"][0]["chatbox"], true);

        let mut follow = sample(Condition::Any, ValueType::Float, OutputAction::Follow { min: 0.0, max: 1.0 });
        assert!(validate(&follow).is_ok());
        if let Action::BoardOutput { chatbox, .. } = &mut follow.then[0].action {
            *chatbox = true;
        }
        assert!(validate(&follow).is_err(), "follow can't post");
    }

    #[test]
    fn output_chatbox_text() {
        assert_eq!(format_duration(500), "0.5s");
        assert_eq!(format_duration(2000), "2s");
        assert_eq!(format_duration(1500), "1.5s");
        assert_eq!(format_duration(60_000), "1m");
        assert_eq!(format_duration(90_000), "1m 30s");
        assert_eq!(chatbox_text("Buzzer", BoardNote::Pulse { on_ms: 2000, count: 1 }), "Buzzer activated for 2s");
        assert_eq!(chatbox_text("Buzzer", BoardNote::Pulse { on_ms: 500, count: 3 }), "Buzzer activated for 0.5s, 3 times");
        assert_eq!(chatbox_text("Lamp", BoardNote::On), "Lamp activated");
        assert_eq!(chatbox_text("Lamp", BoardNote::Off), "Lamp turned off");
        assert_eq!(chatbox_text(&"x".repeat(200), BoardNote::On).chars().count(), CHATBOX_MAX);
    }

    #[test]
    fn owo_steps_read_as_the_webview_writes_them() {
        use crate::owo::model::Muscle;
        let step: Step = serde_json::from_str(r#"{"kind":"owo-sensation","sensation":"pulse","muscles":["LumbarR","LumbarL"],"intensity":0.3}"#).unwrap();
        assert_eq!(step.action, Action::OwoSensation { sensation: "pulse".into(), muscles: vec![Muscle::LumbarR, Muscle::LumbarL], intensity: 0.3 });
        assert!(validate_step(&step, Runs::Moment, None).is_ok());
        let step: Step = serde_json::from_str(r#"{"kind":"owo-sensation","sensation":"explode","muscles":["ArmL"],"intensity":0.3}"#).unwrap();
        assert!(validate_step(&step, Runs::Moment, None).is_err());
        let step: Step = serde_json::from_str(r#"{"kind":"owo-sensation","sensation":"tap","muscles":[],"intensity":0.3}"#).unwrap();
        assert!(validate_step(&step, Runs::Moment, None).is_err());
    }

    #[test]
    fn bhaptics_steps_read_as_the_webview_writes_them() {
        use crate::bhaptics::model::Position;
        let step: Step = serde_json::from_str(r#"{"kind":"bhaptics-pulse","position":"VestBack","motors":[0,1],"intensity":0.5,"ms":800}"#).unwrap();
        assert_eq!(step.action, Action::BhapticsPulse { position: Position::VestBack, motors: vec![0, 1], intensity: 0.5, ms: 800 });
        assert_eq!(step.action.busy_ms(), 800);
        assert!(validate_step(&step, Runs::Moment, None).is_ok());
        let step: Step = serde_json::from_str(r#"{"kind":"bhaptics-effect","effect":"rain","positions":["VestFront","VestBack"],"intensity":1}"#).unwrap();
        assert!(validate_step(&step, Runs::Moment, None).is_ok());
        let step: Step = serde_json::from_str(r#"{"kind":"bhaptics-pulse","position":"Head","motors":[6],"intensity":0.5,"ms":800}"#).unwrap();
        assert!(validate_step(&step, Runs::Moment, None).is_err(), "a Tactal has 6 motors");
        let step: Step = serde_json::from_str(r#"{"kind":"bhaptics-effect","effect":"explode","positions":["Head"],"intensity":1}"#).unwrap();
        assert!(validate_step(&step, Runs::Moment, None).is_err());
    }

    #[test]
    fn toy_steps_read_as_the_webview_writes_them() {
        let step: Step = serde_json::from_str(r#"{"kind":"toy","toys":[],"run":{"type":"pulse","level":0.6,"ms":2000}}"#).unwrap();
        assert_eq!(step.action, Action::Toy { toys: vec![], run: ToyRun::Pulse { level: 0.6, ms: 2000 } });
        assert_eq!(step.action.runs(), Runs::Moment);
        assert_eq!(step.action.busy_ms(), 2000);
        assert!(step.action.in_app());
        assert!(validate_step(&step, Runs::Moment, None).is_ok());

        let hold: Step = serde_json::from_str(
            r#"{"kind":"toy","toys":[{"deviceIndex":0,"deviceName":"Lush","actuatorIndex":0,"actuatorDescription":"Vibrate","actuatorType":"Vibrate"}],"run":{"type":"hold","level":1}}"#
        )
        .unwrap();
        assert_eq!(hold.action.runs(), Runs::State);
        assert!(validate_step(&hold, Runs::Moment, None).is_err(), "hold on a moment");

        let ramp: Step = serde_json::from_str(r#"{"kind":"toy","run":{"type":"ramp","from":0,"to":1.5,"ms":3000}}"#).unwrap();
        assert!(validate_step(&ramp, Runs::Moment, None).is_err(), "level above 100 %");

        let pattern: Step = serde_json::from_str(r#"{"kind":"toy-pattern","patternId":"wave","ms":5000}"#).unwrap();
        assert_eq!(pattern.action.runs(), Runs::Moment);
        assert!(validate_step(&pattern, Runs::Moment, None).is_ok());
        let held: Step = serde_json::from_str(r#"{"kind":"toy-pattern","patternId":"wave","whileHeld":true}"#).unwrap();
        assert_eq!(held.action.runs(), Runs::State);
        assert!(validate_step(&held, Runs::State, None).is_ok());
        let endless: Step = serde_json::from_str(r#"{"kind":"toy-pattern","patternId":"wave"}"#).unwrap();
        assert!(validate_step(&endless, Runs::Moment, None).is_err(), "a moment pattern needs a length");

        let stop: Step = serde_json::from_str(r#"{"kind":"stop-toys"}"#).unwrap();
        assert_eq!(stop.action, Action::StopToys);
    }

    #[test]
    fn more_steps_validate() {
        let ok = |json: &str| validate_step(&serde_json::from_str::<Step>(json).unwrap(), Runs::Moment, None).is_ok();
        assert!(ok(r#"{"kind":"one-of","options":[{"kind":"chatbox","text":"a"},{"kind":"stop-toys"}]}"#));
        assert!(!ok(r#"{"kind":"one-of","options":[{"kind":"chatbox","text":"a"}]}"#), "needs two");
        assert!(!ok(r#"{"kind":"one-of","options":[{"kind":"chatbox","text":"a"},{"kind":"toy","run":{"type":"hold","level":1}}]}"#), "moments only");
        assert!(ok(r#"{"kind":"osc-message","address":"/input/Jump","value":true}"#));
        assert!(!ok(r#"{"kind":"osc-message","address":"input Jump","value":1}"#));
        assert!(ok(r#"{"kind":"avatar-parameter","parameter":"/avatar/parameters/Outfit","valueType":"Int","set":{"type":"add","amount":1}}"#));
        assert!(!ok(r#"{"kind":"avatar-parameter","parameter":"/avatar/parameters/Ears","valueType":"Bool","set":{"type":"add","amount":1}}"#));
        assert!(ok(r#"{"kind":"vr-toast","text":"Hi {viewer}"}"#));

        let mut a = sample(Condition::TurnsOn, ValueType::Bool, OutputAction::Toggle);
        a.then = vec![Step { action: Action::AutomationEnabled { automation_id: a.id.clone(), enabled: false }, delay_ms: 0, after_previous: false }];
        assert!(validate(&a).is_err(), "turns itself off");
    }

    #[test]
    fn leash_events_and_step_read_as_the_webview_writes_them() {
        let when: Trigger = serde_json::from_str(r#"{"kind":"leash","event":"running"}"#).unwrap();
        assert_eq!(when, Trigger::Leash { event: LeashEvent::Running });
        assert_eq!(when.runs(), Runs::Moment);

        let step: Step = serde_json::from_str(r#"{"kind":"leash-enabled","enabled":false}"#).unwrap();
        assert_eq!(step.action, Action::LeashEnabled { enabled: false });
        assert_eq!(step.action.runs(), Runs::Moment);
        assert!(!step.action.in_app());
        assert!(validate_step(&step, Runs::Moment, None).is_ok());
    }
}
