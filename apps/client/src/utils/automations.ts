import type {
  Automation,
  AutomationAction,
  AutomationStep,
  AutomationTrigger,
  AvatarParameterAction,
  Board,
  BoardOutputAction,
  OutputAction,
  ParameterAction,
  ParameterCondition,
  ParameterValueType
} from '@renderer/lib/tauri-bridge'

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

export const conditionsFor = (valueType: ParameterValueType): ConditionType[] =>
  (Object.keys(CONDITIONS) as ConditionType[]).filter((type) => CONDITIONS[type].valueType === valueType)

export const actionsFor = (condition: ConditionType): OutputActionType[] =>
  (Object.keys(OUTPUT_ACTIONS) as OutputActionType[]).filter((type) => OUTPUT_ACTIONS[type].fits(condition))

export const parameterActionsFor = (condition: ConditionType, valueType: ParameterValueType): ParameterActionType[] =>
  (Object.keys(PARAMETER_ACTIONS) as ParameterActionType[]).filter(
    (type) => PARAMETER_ACTIONS[type].fits(condition) && PARAMETER_ACTIONS[type].valueTypes.includes(valueType)
  )

// Whether a step can run on this condition: moments, states and follows each take their own.
export const stepFits = (step: AutomationAction, condition: ConditionType): boolean =>
  step.kind === 'board-output'
    ? OUTPUT_ACTIONS[step.output.type].fits(condition)
    : PARAMETER_ACTIONS[step.set.type].fits(condition) && PARAMETER_ACTIONS[step.set.type].valueTypes.includes(step.valueType)

// Follows run on every change, so they can't wait.
export const stepCanWait = (step: AutomationAction): boolean =>
  (step.kind === 'board-output' ? step.output.type : step.set.type) !== 'follow'

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

// The same kind of step, changed to fit a new condition.
export const fitStep = (step: AutomationStep, condition: ConditionType): AutomationStep => {
  if (stepFits(step, condition)) return stepCanWait(step) ? step : { ...step, delayMs: 0 }
  if (step.kind === 'board-output') return { ...step, output: defaultOutput(actionsFor(condition)[0]!), delayMs: condition === 'any' ? 0 : step.delayMs }

  const types = parameterActionsFor(condition, step.valueType)
  // A follow needs a Float: anything else starts over.
  if (!types.length) return newParameterStep(condition)
  return { ...step, set: defaultParameterAction(types[0]!, step.valueType), delayMs: condition === 'any' ? 0 : step.delayMs }
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

export const describeStep = (step: AutomationStep, boards: Board[]): string => {
  const what =
    step.kind === 'board-output'
      ? `${describeTarget(step, boards)}: ${describeOutputAction(step.output)}`
      : describeParameterAction(step)
  return step.delayMs ? `after ${formatMs(step.delayMs)}, ${what}` : what
}

export const describe = (automation: Automation, boards: Board[]): string =>
  `When ${describeWhen(automation.when)} → ${automation.then.map((step) => describeStep(step, boards)).join(' · ')}`

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
