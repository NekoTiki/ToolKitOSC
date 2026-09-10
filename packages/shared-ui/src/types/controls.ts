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
  | 'intiface-vibrator'

// Nuxt UI's default semantic color palette - Badge/etc. accept any of these as `color`.
export type UiColor = 'primary' | 'secondary' | 'success' | 'info' | 'warning' | 'error' | 'neutral'

// Single source of truth for how a control type is presented outside its own editor - the type
// picker (ControlModal's "Type" select) and the client logs viewer (ClientLogsModal) both read
// from this instead of keeping their own copies, so a badge always matches the name the control
// was created under. One color per type used to mean every badge was distinguishable at a glance,
// but all 7 of Nuxt UI's semantic colors are already spoken for - 'intiface-vibrator' reuses
// 'slider''s 'success' since it's functionally a slider under the hood, rather than adding a
// non-semantic color just to stay unique.
export const CONTROL_TYPE_LABELS: Record<ControlTypes, string> = {
  boolean: 'Toggle',
  'boolean-group': 'Toggle Group',
  'boolean-enum': 'Toggle Logic',
  enum: 'Enum',
  slider: 'Slider',
  'step-enum': 'Step Enum',
  'open-shock-shocker': 'Open Shock',
  'intiface-vibrator': 'Intiface'
}

export const CONTROL_TYPE_COLORS: Record<ControlTypes, UiColor> = {
  boolean: 'primary',
  'boolean-group': 'secondary',
  'boolean-enum': 'info',
  enum: 'warning',
  slider: 'success',
  'step-enum': 'neutral',
  'open-shock-shocker': 'error',
  'intiface-vibrator': 'success'
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

// One entry per selected Buttplug actuator, not per toy - a toy with more than one vibration
// motor is asked about independently (see IntifaceFields.vue), so the same control can drive
// several actuators across several toys with a single value. `deviceName`/`actuatorDescription`
// are display names captured at selection time (mirrors OpenShockControl's use of ids alone,
// resolved to a name at command time instead - here there's no equivalent lookup available once
// a toy is unplugged, so the label is kept on the control itself).
export interface IntifaceVibratorRef {
  deviceIndex: number
  deviceName: string
  actuatorIndex: number
  actuatorDescription: string
}

export interface IntifaceVibratorControl extends BaseControl {
  type: 'intiface-vibrator'
  vibrators: IntifaceVibratorRef[]
}

export type ControlType =
  | BooleanControl
  | BooleanGroupControl
  | BooleanEnumControl
  | EnumControl
  | SliderControl
  | StepEnumControl
  | OpenShockControl
  | IntifaceVibratorControl

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

export interface IntifaceVibratorControlCommand extends BaseControlCommand {
  type: 'intiface-vibrator'
  value: number
}

export type ControlCommand =
  | BooleanControlCommand
  | BooleanGroupControlCommand
  | BooleanEnumControlCommand
  | EnumControlCommand
  | SliderControlCommand
  | StepEnumControlCommand
  | OpenShockControlCommand
  | IntifaceVibratorControlCommand

export type CommandWithoutIds =
  | Omit<BooleanControlCommand, 'groupId' | 'controlId'>
  | Omit<BooleanGroupControlCommand, 'groupId' | 'controlId'>
  | Omit<BooleanEnumControlCommand, 'groupId' | 'controlId'>
  | Omit<EnumControlCommand, 'groupId' | 'controlId'>
  | Omit<SliderControlCommand, 'groupId' | 'controlId'>
  | Omit<StepEnumControlCommand, 'groupId' | 'controlId'>
  | Omit<OpenShockControlCommand, 'groupId' | 'controlId'>
  | Omit<IntifaceVibratorControlCommand, 'groupId' | 'controlId'>

export interface ControlGroup {
  id: string
  name: string
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
