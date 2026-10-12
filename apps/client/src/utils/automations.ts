import type {
  Automation,
  AutomationAction,
  AutomationStep,
  AutomationTrigger,
  AvatarParameterAction,
  Board,
  BoardOutputAction,
  ControlRun,
  OutputAction,
  ParameterAction,
  ParameterCondition,
  ParameterValueType
} from '@renderer/lib/tauri-bridge'
import type { ControlType, ControlTypes } from '@toolkitosc/shared-ui'

// What each kind of "when" and "do" means, for the editor and the list. The backend checks the
// same rules (src-tauri/src/automations/model.rs `validate`); this only keeps the editor from
// offering pairs it would refuse.

export type ConditionType = ParameterCondition['type']
export type OutputActionType = OutputAction['type']

export const CONDITIONS: Record<
  ConditionType,
  {
    label: string
    valueType: ParameterValueType
    // A moment (an edge) rather than a state (a level).
    moment: boolean
    needsValue: boolean
  }
> = {
  'turns-on': { label: 'Turns on', valueType: 'Bool', moment: true, needsValue: false },
  'turns-off': { label: 'Turns off', valueType: 'Bool', moment: true, needsValue: false },
  'is-on': { label: 'Is on', valueType: 'Bool', moment: false, needsValue: false },
  becomes: { label: 'Becomes', valueType: 'Int', moment: true, needsValue: true },
  is: { label: 'Is', valueType: 'Int', moment: false, needsValue: true },
  'rises-above': { label: 'Rises above', valueType: 'Float', moment: true, needsValue: true },
  'is-above': { label: 'Is above', valueType: 'Float', moment: false, needsValue: true },
  any: { label: 'Any change', valueType: 'Float', moment: false, needsValue: false }
}

export const OUTPUT_ACTIONS: Record<
  OutputActionType,
  { label: string; description: string; fits: (condition: ConditionType) => boolean; needsPwm: boolean }
> = {
  pulse: {
    label: 'Pulse',
    description: 'On for a moment, optionally repeated.',
    fits: (c) => CONDITIONS[c].moment,
    needsPwm: false
  },
  toggle: {
    label: 'Toggle',
    description: 'Flips it on or off, never past its max on-time.',
    fits: (c) => CONDITIONS[c].moment,
    needsPwm: false
  },
  off: { label: 'Off', description: 'Switches it off.', fits: (c) => CONDITIONS[c].moment, needsPwm: false },
  hold: {
    label: 'Hold',
    description: 'On for as long as the condition is true.',
    fits: (c) => !CONDITIONS[c].moment && c !== 'any',
    needsPwm: false
  },
  follow: {
    label: 'Follow',
    description: 'Dims it with the value.',
    fits: (c) => c === 'any',
    needsPwm: true
  }
}

export type ParameterActionType = ParameterAction['type']

export const PARAMETER_ACTIONS: Record<
  ParameterActionType,
  {
    label: string
    description: string
    fits: (condition: ConditionType) => boolean
    // Which parameter types it can set.
    valueTypes: ParameterValueType[]
  }
> = {
  set: {
    label: 'Set',
    description: 'Sets it to a value.',
    fits: (c) => CONDITIONS[c].moment,
    valueTypes: ['Bool', 'Int', 'Float']
  },
  toggle: { label: 'Toggle', description: 'Flips it on or off.', fits: (c) => CONDITIONS[c].moment, valueTypes: ['Bool'] },
  hold: {
    label: 'Hold',
    description: 'This value while the condition is true, then back to what it was.',
    fits: (c) => !CONDITIONS[c].moment && c !== 'any',
    valueTypes: ['Bool', 'Int', 'Float']
  },
  follow: { label: 'Follow', description: 'Follows the value, mapped onto a range.', fits: (c) => c === 'any', valueTypes: ['Float'] }
}

// Most steps one automation can run, and the longest "after" delay (model.rs MAX_STEPS, MAX_DELAY_MS).
export const MAX_STEPS = 8
export const MAX_DELAY_MS = 10_000
// model.rs CHATBOX_MAX, MAX_TOY_PULSE_MS.
export const CHATBOX_MAX = 144
export const MAX_TOY_PULSE_MS = 60_000

export const conditionsFor = (valueType: ParameterValueType): ConditionType[] =>
  (Object.keys(CONDITIONS) as ConditionType[]).filter((type) => CONDITIONS[type].valueType === valueType)

export const actionsFor = (condition: ConditionType): OutputActionType[] =>
  (Object.keys(OUTPUT_ACTIONS) as OutputActionType[]).filter((type) => OUTPUT_ACTIONS[type].fits(condition))

export const parameterActionsFor = (condition: ConditionType, valueType: ParameterValueType): ParameterActionType[] =>
  (Object.keys(PARAMETER_ACTIONS) as ParameterActionType[]).filter(
    (type) => PARAMETER_ACTIONS[type].fits(condition) && PARAMETER_ACTIONS[type].valueTypes.includes(valueType)
  )

export type ControlRunType = ControlRun['type']
// What kind of trigger edge a step runs on (model.rs `Runs`).
export type Runs = 'moment' | 'state' | 'follow'

const runsFor = (condition: ConditionType): Runs =>
  CONDITIONS[condition].moment ? 'moment' : condition === 'any' ? 'follow' : 'state'

// The ways to run a control, and the control types each fits (model.rs `run_fits`).
export const CONTROL_RUNS: Record<ControlRunType, { label: string; description: string; runs: Runs; controlTypes: ControlTypes[] }> = {
  set: {
    label: 'Set',
    description: 'Sets it, the way a viewer would.',
    runs: 'moment',
    controlTypes: ['boolean', 'boolean-group', 'boolean-enum', 'enum', 'step-enum', 'slider', 'intiface-toy', 'intiface-pattern']
  },
  toggle: { label: 'Toggle', description: 'Flips it on or off.', runs: 'moment', controlTypes: ['boolean', 'boolean-group'] },
  pulse: { label: 'Pulse', description: 'The toy at this intensity for a moment, then off.', runs: 'moment', controlTypes: ['intiface-toy'] },
  press: { label: 'Press', description: 'Presses it once.', runs: 'moment', controlTypes: ['preset', 'open-shock-shocker'] },
  hold: {
    label: 'Hold',
    description: 'This while the condition is true, then back to what it was.',
    runs: 'state',
    controlTypes: ['boolean', 'boolean-group', 'enum', 'step-enum', 'slider', 'intiface-toy', 'intiface-pattern']
  },
  follow: { label: 'Follow', description: 'Follows the value, mapped onto a range.', runs: 'follow', controlTypes: ['slider', 'intiface-toy'] }
}

export const controlRunsFor = (condition: ConditionType, controlType: string): ControlRunType[] =>
  (Object.keys(CONTROL_RUNS) as ControlRunType[]).filter(
    (type) => CONTROL_RUNS[type].runs === runsFor(condition) && CONTROL_RUNS[type].controlTypes.includes(controlType as ControlTypes)
  )

export const stepRuns = (step: AutomationAction): Runs => {
  switch (step.kind) {
    case 'board-output':
      return step.output.type === 'hold' ? 'state' : step.output.type === 'follow' ? 'follow' : 'moment'
    case 'avatar-parameter':
      return step.set.type === 'hold' ? 'state' : step.set.type === 'follow' ? 'follow' : 'moment'
    case 'control':
      return CONTROL_RUNS[step.run.type].runs
    case 'profile':
      return step.whileHeld ? 'state' : 'moment'
    default:
      return 'moment'
  }
}

// Whether a step can run on this condition: moments, states and follows each take their own.
export const stepFits = (step: AutomationAction, condition: ConditionType): boolean => {
  if (step.kind === 'avatar-parameter' && !PARAMETER_ACTIONS[step.set.type].valueTypes.includes(step.valueType)) return false
  return stepRuns(step) === runsFor(condition)
}

// Follows run on every change, so they can't wait.
export const stepCanWait = (step: AutomationAction): boolean => stepRuns(step) !== 'follow'

// The kinds of step that have something to do on this condition.
export const STEP_KINDS: Record<AutomationStep['kind'], { label: string; description: string; icon: string; runs: Runs[] }> = {
  'board-output': {
    label: 'ESP32 output',
    description: 'Switch something wired to one of your boards.',
    icon: 'i-lucide-cpu',
    runs: ['moment', 'state', 'follow']
  },
  'avatar-parameter': {
    label: 'Avatar parameter',
    description: "Set one of your avatar's own parameters.",
    icon: 'i-lucide-sliders-horizontal',
    runs: ['moment', 'state', 'follow']
  },
  control: {
    label: 'Control',
    description: 'Run one of your controls, toys included.',
    icon: 'i-lucide-gamepad-2',
    runs: ['moment', 'state', 'follow']
  },
  profile: { label: 'Profile', description: 'Switch to another profile.', icon: 'i-lucide-shield', runs: ['moment', 'state'] },
  chatbox: { label: 'Chatbox', description: 'Post a message to the VRChat chatbox.', icon: 'i-lucide-message-square', runs: ['moment'] },
  'stop-everything': {
    label: 'Stop everything',
    description: 'Same as the button in the top bar.',
    icon: 'i-lucide-octagon-x',
    runs: ['moment']
  }
}

export const stepKindsFor = (condition: ConditionType): AutomationStep['kind'][] =>
  (Object.keys(STEP_KINDS) as AutomationStep['kind'][]).filter((kind) => STEP_KINDS[kind].runs.includes(runsFor(condition)))

export const defaultCondition = (type: ConditionType): ParameterCondition => {
  switch (type) {
    case 'becomes':
    case 'is':
      return { type, value: 1 }
    case 'rises-above':
    case 'is-above':
      return { type, value: 0.5 }
    default:
      return { type }
  }
}

export const defaultOutput = (type: OutputActionType): OutputAction => {
  switch (type) {
    case 'pulse':
      return { type, onMs: 1000, offMs: 500, count: 1 }
    case 'follow':
      return { type, min: 0, max: 1 }
    default:
      return { type }
  }
}

// A value that fits the parameter's type: Bools are 0/1, Ints 0–255, Floats -1 to 1.
export const defaultParameterAction = (type: ParameterActionType, valueType: ParameterValueType): ParameterAction => {
  switch (type) {
    case 'set':
    case 'hold':
      return { type, value: 1 }
    case 'follow':
      return { type, min: 0, max: 1 }
    default:
      return valueType === 'Bool' ? { type } : { type: 'set', value: 1 }
  }
}

export const newBoardStep = (condition: ConditionType): AutomationStep => ({
  kind: 'board-output',
  boardId: '',
  pin: -1,
  output: defaultOutput(actionsFor(condition)[0]!),
  delayMs: 0
})

export const newParameterStep = (condition: ConditionType): AutomationStep => ({
  kind: 'avatar-parameter',
  parameter: '',
  valueType: 'Bool',
  set: defaultParameterAction(parameterActionsFor(condition, 'Bool')[0] ?? 'follow', 'Bool'),
  delayMs: 0
})

// A value that fits the control: on for toggles, the first option or pattern, half for sliders.
export const defaultControlValue = (control: ControlType | undefined): boolean | number | string => {
  switch (control?.type) {
    case 'boolean':
    case 'boolean-group':
      return true
    case 'slider':
    case 'intiface-toy':
      return 0.5
    case 'intiface-pattern':
      return control.allowedPatterns[0]?.id ?? 'off'
    default:
      return 0
  }
}

export const defaultControlRun = (type: ControlRunType, control: ControlType | undefined): ControlRun => {
  switch (type) {
    case 'set':
    case 'hold':
      return { type, value: defaultControlValue(control) }
    case 'pulse':
      return { type, value: 0.5, ms: 2000 }
    case 'follow':
      return { type, min: 0, max: 1 }
    default:
      return { type }
  }
}

export const newStep = (kind: AutomationStep['kind'], condition: ConditionType): AutomationStep => {
  switch (kind) {
    case 'board-output':
      return newBoardStep(condition)
    case 'avatar-parameter':
      return newParameterStep(condition)
    case 'control':
      return { kind, groupId: '', controlId: '', controlType: '', run: { type: 'press' }, delayMs: 0 }
    case 'profile':
      return { kind, profileId: null, whileHeld: runsFor(condition) === 'state', delayMs: 0 }
    case 'chatbox':
      return { kind, text: '', delayMs: 0 }
    default:
      return { kind, delayMs: 0 }
  }
}

// The same kind of step, changed to fit a new condition. Steps that can't fit any more are
// left as they are; the editor says what's wrong with them.
export const fitStep = (step: AutomationStep, condition: ConditionType, control?: ControlType): AutomationStep => {
  const delayMs = runsFor(condition) === 'follow' ? 0 : step.delayMs
  if (stepFits(step, condition)) return { ...step, delayMs }

  switch (step.kind) {
    case 'board-output':
      return { ...step, output: defaultOutput(actionsFor(condition)[0]!), delayMs }
    case 'avatar-parameter': {
      const types = parameterActionsFor(condition, step.valueType)
      // A follow needs a Float: anything else starts over.
      if (!types.length) return newParameterStep(condition)
      return { ...step, set: defaultParameterAction(types[0]!, step.valueType), delayMs }
    }
    case 'control': {
      const types = controlRunsFor(condition, step.controlType)
      return types.length ? { ...step, run: defaultControlRun(types[0]!, control), delayMs } : step
    }
    case 'profile':
      return runsFor(condition) === 'follow' ? step : { ...step, whileHeld: runsFor(condition) === 'state', delayMs }
    default:
      return step
  }
}

export const parameterName = (address: string): string => address.replace(/^\/avatar\/parameters\//, '')

// ESP32 timings are always shown in milliseconds, like the board itself counts them.
export const formatMs = (ms: number): string => `${Math.round(ms)} ms`

const formatValue = (value: number, valueType: ParameterValueType): string =>
  valueType === 'Bool' ? (value ? 'on' : 'off') : String(value)

export const describeWhen = (when: AutomationTrigger): string => {
  const condition = CONDITIONS[when.condition.type]
  const value = 'value' in when.condition ? ` ${when.condition.value}` : ''
  return `${parameterName(when.parameter)} ${condition.label.toLowerCase()}${value}`
}

export const describeOutputAction = (output: OutputAction): string => {
  switch (output.type) {
    case 'pulse':
      return output.count > 1
        ? `pulse ${formatMs(output.onMs)} × ${output.count}`
        : `pulse ${formatMs(output.onMs)}`
    case 'follow':
      return `dim ${Math.round(output.min * 100)}–${Math.round(output.max * 100)} %`
    default:
      return OUTPUT_ACTIONS[output.type].label.toLowerCase()
  }
}

// "Fan (Desk board)", or what's left of it when the board or output is gone.
export const describeTarget = (action: BoardOutputAction, boards: Board[]): string => {
  const board = boards.find((b) => b.id === action.boardId)
  const output = board?.config?.find((o) => o.pin === action.pin)
  if (!board) return `GPIO ${action.pin} (removed board)`
  return output ? `${output.label} (${board.name})` : `GPIO ${action.pin} (${board.name})`
}

export const describeParameterAction = (action: AvatarParameterAction): string => {
  const name = parameterName(action.parameter)
  switch (action.set.type) {
    case 'set':
      return `${name}: set ${formatValue(action.set.value, action.valueType)}`
    case 'hold':
      return `${name}: ${formatValue(action.set.value, action.valueType)} while held`
    case 'follow':
      return `${name}: follow ${action.set.min} to ${action.set.max}`
    default:
      return `${name}: toggle`
  }
}

// Names the steps point at. Controls and profiles are only known for the worn avatar.
export interface DescribeContext {
  boards: Board[]
  controlName?: (groupId: string, controlId: string) => string | undefined
  profileName?: (id: string) => string | undefined
}

const describeControlRun = (run: ControlRun): string => {
  switch (run.type) {
    case 'pulse':
      return `pulse ${Math.round(run.value * 100)} % for ${formatMs(run.ms)}`
    case 'follow':
      return `follow ${Math.round(run.min * 100)}–${Math.round(run.max * 100)} %`
    default:
      return CONTROL_RUNS[run.type].label.toLowerCase()
  }
}

export const describeStep = (step: AutomationStep, context: DescribeContext): string => {
  let what: string
  switch (step.kind) {
    case 'board-output':
      what = `${describeTarget(step, context.boards)}: ${describeOutputAction(step.output)}`
      break
    case 'avatar-parameter':
      what = describeParameterAction(step)
      break
    case 'control':
      what = `${context.controlName?.(step.groupId, step.controlId) ?? 'a control'}: ${describeControlRun(step.run)}`
      break
    case 'profile': {
      const name = step.profileId ? (context.profileName?.(step.profileId) ?? 'a profile') : 'No profile'
      what = step.whileHeld ? `${name} while held` : `switch to ${name}`
      break
    }
    case 'chatbox':
      what = `chatbox "${step.text}"`
      break
    default:
      what = 'stop everything'
  }
  return step.delayMs ? `after ${formatMs(step.delayMs)}, ${what}` : what
}

export const describe = (automation: Automation, context: DescribeContext): string =>
  `When ${describeWhen(automation.when)} → ${automation.then.map((step) => describeStep(step, context)).join(' · ')}`

export const newAutomation = (avatarId: string, avatarName: string): Automation => ({
  id: crypto.randomUUID(),
  name: '',
  enabled: true,
  when: {
    kind: 'avatar-parameter',
    avatarId,
    avatarName,
    parameter: '',
    valueType: 'Bool',
    condition: { type: 'turns-on' }
  },
  then: [newBoardStep('turns-on')],
  cooldownMs: 0,
  retrigger: 'restart'
})
