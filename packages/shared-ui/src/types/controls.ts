// Control-definition domain model, shared by the desktop client (authoring) and the server's
// viewer page (read-only rendering) and Nitro relay routes.
//
// Bug fix vs. the two previously-duplicated `controls.d.ts` copies: 'boolean-enum' was missing
// from `ControlTypes`/implied by `ControlType`, even though `BooleanEnumControl` and every
// dispatcher (`Control.vue`) already handled it at runtime.
export type ControlTypes =
  | 'boolean'
  | 'boolean-group'
  | 'boolean-enum'
  | 'enum'
  | 'slider'
  | 'step-enum'
  | 'open-shock-shocker'
  | 'intiface-toy'
  | 'intiface-pattern'

// Nuxt UI's default semantic color palette - Badge/etc. accept any of these as `color`.
export type UiColor = 'primary' | 'secondary' | 'success' | 'info' | 'warning' | 'error' | 'neutral'

// Single source of truth for how a control type is presented outside its own editor - the type
// picker (ControlModal's "Type" select) and the client logs viewer (ClientLogsModal) both read
// from this instead of keeping their own copies, so a badge always matches the name the control
// was created under. One color per type used to mean every badge was distinguishable at a glance,
// but all 7 of Nuxt UI's semantic colors are already spoken for - 'intiface-toy' reuses 'slider''s
// 'success' since it's functionally a slider under the hood, rather than adding a non-semantic
// color just to stay unique.
export const CONTROL_TYPE_LABELS: Record<ControlTypes, string> = {
  boolean: 'Toggle',
  'boolean-group': 'Toggle Group',
  'boolean-enum': 'Toggle Logic',
  enum: 'Enum',
  slider: 'Slider',
  'step-enum': 'Step Enum',
  // Named for what each one does rather than the integration it belongs to - the type picker
  // already groups these under an "OpenShock"/"Intiface" heading (see ControlModal.vue), so
  // repeating the brand name in the label itself was redundant there.
  'open-shock-shocker': 'Shocker',
  'intiface-toy': 'Toy',
  'intiface-pattern': 'Toy Pattern'
}

export const CONTROL_TYPE_COLORS: Record<ControlTypes, UiColor> = {
  boolean: 'primary',
  'boolean-group': 'secondary',
  'boolean-enum': 'info',
  enum: 'warning',
  slider: 'success',
  'step-enum': 'neutral',
  'open-shock-shocker': 'error',
  'intiface-toy': 'success',
  // Reuses 'enum''s 'warning' - it's a chip-picker built on the exact same UX, and we're already
  // out of unique semantic colors (see the comment above).
  'intiface-pattern': 'warning'
}

export interface BaseControl {
  id: string
  name: string
  icon?: string
  locked?: boolean
  // Distinct from `locked` (an admin choosing to lock a control): set when the control depends on
  // an external service (currently only OpenShock) that isn't configured/reachable, so it isn't
  // usable regardless of any lock group. Computed client-side (see useControls.ts) and enforced
  // host-side the same way `locked` is (see useWebsocketHost.ts).
  unavailable?: boolean
  type: ControlTypes
}

export interface BooleanControl extends BaseControl {
  type: 'boolean'
  reverse?: boolean
  inputAddress: string
}

export interface BooleanGroupControl extends BaseControl {
  type: 'boolean-group'
  inputs: {
    name?: string
    inputAddress: string
  }[]
}

export interface BooleanEnumControl extends BaseControl {
  type: 'boolean-enum'
  inputs: {
    id: string
    name: string
    icon?: string
    inputAddress: {
      false: string[]
      true: string[]
    }
  }[]
}

export interface EnumControl extends BaseControl {
  type: 'enum'
  inputAddress: string
  options: {
    name: string
    icon?: string
    value: number
  }[]
}

export interface SliderControl extends BaseControl {
  type: 'slider'
  inputAddress: string
}

export interface StepEnumControl extends BaseControl {
  type: 'step-enum'
  inputAddress: string
  options: {
    name: string
    icon?: string
    value: number
  }[]
}

export interface OpenShockControl extends BaseControl {
  type: 'open-shock-shocker'
  mode: 'Shock' | 'Vibrate'
  shockers: string[]
  intensity: {
    min: number
    max: number
  }
  duration: {
    min: number
    max: number
  }
  cooldown: number
  animationDuration: 3000
}

// One entry per selected Buttplug actuator, not per toy - a toy with more than one motor is asked
// about independently (see IntifaceFields.vue), so the same control can drive several actuators
// - of any kind (vibrate, rotate, oscillate, ...), not just vibration - across several toys with
// a single value. `deviceName`/`actuatorDescription` are display names captured at selection time
// (mirrors OpenShockControl's use of ids alone, resolved to a name at command time instead - here
// there's no equivalent lookup available once a toy is unplugged, so the label is kept on the
// control itself). `actuatorType` (Buttplug's "Vibrate"/"Rotate"/"Oscillate"/...) is likewise kept
// on the control - it has to be sent back verbatim in the ScalarCmd for this actuator.
export interface IntifaceActuatorRef {
  deviceIndex: number
  deviceName: string
  actuatorIndex: number
  actuatorDescription: string
  actuatorType: string
}

// Named 'intiface-toy' (not '...vibrator') and `actuators` (not `vibrators`) since a control can
// target any mix of actuator types across one or more toys, not just vibration motors - see
// IntifaceActuatorRef above.
export interface IntifaceToyControl extends BaseControl {
  type: 'intiface-toy'
  actuators: IntifaceActuatorRef[]
}

// The implicit pattern-command value meaning "stop playing, drive nothing" - never itself one of a
// control's configurable `allowedPatterns` (see below), always available regardless of them.
export const INTIFACE_PATTERN_OFF_ID = 'off'

// One entry per pattern the control's author has chosen to allow, in the order they should be
// offered. `name`/`icon` are captured at authoring time from whatever pattern registry defines
// them (currently only apps/client's built-in one - see useIntifacePatterns.ts) rather than
// looked up live by `id`, the same denormalization IntifaceActuatorRef above already does for a
// toy's name - this control's data stays fully self-describing even if the registry entry it came
// from is later renamed or removed. A future custom/user-authored pattern only needs to add itself
// to that registry and it can be picked the same way as any built-in one; nothing here has to
// change to accommodate it.
export interface IntifacePatternRef {
  id: string
  name: string
  icon?: string
}

// A chip-picker (see ControlIntifacePattern.vue), same shape of UI as EnumControl, but playing a
// named, time-varying pattern against a set of Intiface actuators instead of setting one static
// OSC value - always includes an implicit "Off" choice on top of `allowedPatterns` (stopping
// playback), which isn't itself one of the configurable patterns.
export interface IntifacePatternControl extends BaseControl {
  type: 'intiface-pattern'
  actuators: IntifaceActuatorRef[]
  allowedPatterns: IntifacePatternRef[]
}

export type ControlType =
  | BooleanControl
  | BooleanGroupControl
  | BooleanEnumControl
  | EnumControl
  | SliderControl
  | StepEnumControl
  | OpenShockControl
  | IntifaceToyControl
  | IntifacePatternControl

export interface BaseControlCommand {
  groupId: string
  controlId: string
  type: ControlTypes
}

export interface BooleanControlCommand extends BaseControlCommand {
  type: 'boolean'
  value: boolean
}

export interface BooleanGroupControlCommand extends BaseControlCommand {
  type: 'boolean-group'
  value: boolean
}

export interface BooleanEnumControlCommand extends BaseControlCommand {
  type: 'boolean-enum'
  value: number
}

export interface EnumControlCommand extends BaseControlCommand {
  type: 'enum'
  value: number
}

export interface SliderControlCommand extends BaseControlCommand {
  type: 'slider'
  value: number
}

export interface StepEnumControlCommand extends BaseControlCommand {
  type: 'step-enum'
  value: number
}

export interface OpenShockControlCommand extends BaseControlCommand {
  type: 'open-shock-shocker'
}

export interface IntifaceToyControlCommand extends BaseControlCommand {
  type: 'intiface-toy'
  value: number
}

// `value` is a pattern id from the control's own `allowedPatterns`, or the implicit 'off'.
export interface IntifacePatternControlCommand extends BaseControlCommand {
  type: 'intiface-pattern'
  value: string
}

export type ControlCommand =
  | BooleanControlCommand
  | BooleanGroupControlCommand
  | BooleanEnumControlCommand
  | EnumControlCommand
  | SliderControlCommand
  | StepEnumControlCommand
  | OpenShockControlCommand
  | IntifaceToyControlCommand
  | IntifacePatternControlCommand

export type CommandWithoutIds =
  | Omit<BooleanControlCommand, 'groupId' | 'controlId'>
  | Omit<BooleanGroupControlCommand, 'groupId' | 'controlId'>
  | Omit<BooleanEnumControlCommand, 'groupId' | 'controlId'>
  | Omit<EnumControlCommand, 'groupId' | 'controlId'>
  | Omit<SliderControlCommand, 'groupId' | 'controlId'>
  | Omit<StepEnumControlCommand, 'groupId' | 'controlId'>
  | Omit<OpenShockControlCommand, 'groupId' | 'controlId'>
  | Omit<IntifaceToyControlCommand, 'groupId' | 'controlId'>
  | Omit<IntifacePatternControlCommand, 'groupId' | 'controlId'>

export interface ControlGroup {
  id: string
  name: string
  // Client-side only: a hidden group is never included in the `controls-update` payload sent to
  // the server (see apps/client's useWebsocketHost.ts `sendControls`), so it stays invisible to
  // the share page and anyone viewing it - the host app itself still shows it (dimmed) so it can
  // be un-hidden later.
  hidden?: boolean
  controls: ControlType[]
}

// The prop shape Control.vue needs to render a "last used by" avatar popover. Both apps already
// produce structurally-compatible objects (the client's Dexie `Client` row is a near-superset of
// this — `discordId` is nullable there since Dexie/IndexedDB stores "none" as `null`, wherever
// the server's ephemeral version just omits the field, hence `string | null | undefined` here).
export type LastUser = {
  avatar: string
  displayName: string
  discordId?: string | null
  ip: string
}
