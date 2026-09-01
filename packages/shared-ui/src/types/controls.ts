// Control-definition domain model, shared by the desktop client (authoring) and the server's
// viewer page (read-only rendering) and Nitro relay routes.
//
// Bug fix vs. the two previously-duplicated `controls.d.ts` copies: 'boolean-enum' was missing
// from `ControlTypes`/implied by `ControlType`, even though `BooleanEnumControl` and every
// dispatcher (`Control.vue`) already handled it at runtime.
type ControlTypes =
  | 'boolean'
  | 'boolean-group'
  | 'boolean-enum'
  | 'enum'
  | 'slider'
  | 'step-enum'
  | 'open-shock-shocker'

export interface BaseControl {
  id: string
  name: string
  icon?: string
  locked?: boolean
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

export type ControlType =
  | BooleanControl
  | BooleanGroupControl
  | BooleanEnumControl
  | EnumControl
  | SliderControl
  | StepEnumControl
  | OpenShockControl

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

export type ControlCommand =
  | BooleanControlCommand
  | BooleanGroupControlCommand
  | BooleanEnumControlCommand
  | EnumControlCommand
  | SliderControlCommand
  | StepEnumControlCommand
  | OpenShockControlCommand

export type CommandWithoutIds =
  | Omit<BooleanControlCommand, 'groupId' | 'controlId'>
  | Omit<BooleanGroupControlCommand, 'groupId' | 'controlId'>
  | Omit<BooleanEnumControlCommand, 'groupId' | 'controlId'>
  | Omit<EnumControlCommand, 'groupId' | 'controlId'>
  | Omit<SliderControlCommand, 'groupId' | 'controlId'>
  | Omit<StepEnumControlCommand, 'groupId' | 'controlId'>
  | Omit<OpenShockControlCommand, 'groupId' | 'controlId'>

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
