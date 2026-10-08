import type {
  Automation,
  AutomationAction,
  AutomationTrigger,
  Board,
  OutputAction,
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

export const conditionsFor = (valueType: ParameterValueType): ConditionType[] =>
  (Object.keys(CONDITIONS) as ConditionType[]).filter((type) => CONDITIONS[type].valueType === valueType)

export const actionsFor = (condition: ConditionType): OutputActionType[] =>
  (Object.keys(OUTPUT_ACTIONS) as OutputActionType[]).filter((type) => OUTPUT_ACTIONS[type].fits(condition))

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

export const parameterName = (address: string): string => address.replace(/^\/avatar\/parameters\//, '')

export const formatMs = (ms: number): string =>
  ms >= 1000 ? `${Number((ms / 1000).toFixed(1))} s` : `${ms} ms`

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
export const describeTarget = (action: AutomationAction, boards: Board[]): string => {
  const board = boards.find((b) => b.id === action.boardId)
  const output = board?.config?.find((o) => o.pin === action.pin)
  if (!board) return `GPIO ${action.pin} (removed board)`
  return output ? `${output.label} (${board.name})` : `GPIO ${action.pin} (${board.name})`
}

export const describe = (automation: Automation, boards: Board[]): string =>
  `When ${describeWhen(automation.when)} → ${describeTarget(automation.then, boards)}: ${describeOutputAction(automation.then.output)}`

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
  then: { kind: 'board-output', boardId: '', pin: -1, output: defaultOutput('pulse') },
  cooldownMs: 0,
  retrigger: 'restart'
})
