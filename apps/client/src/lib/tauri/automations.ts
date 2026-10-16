// Automations - src-tauri/src/automations/.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// Automations - mirrors src-tauri/src/automations/model.rs. "When" and "do" are tagged on `kind`
// so other kinds (app events, chatbox...) can be added next to these.
export type ParameterValueType = 'Bool' | 'Int' | 'Float'

export type ParameterCondition =
  | { type: 'turns-on' }
  | { type: 'turns-off' }
  | { type: 'is-on' }
  | { type: 'becomes'; value: number }
  | { type: 'is'; value: number }
  | { type: 'rises-above'; value: number }
  | { type: 'is-above'; value: number }
  | { type: 'any' }

export type AvatarParameterTrigger = {
  kind: 'avatar-parameter'
  avatarId: string
  avatarName: string
  // Full OSC address, e.g. /avatar/parameters/Boop.
  parameter: string
  valueType: ParameterValueType
  condition: ParameterCondition
  // Runs on any avatar with a parameter of this name.
  anyAvatar: boolean
  // A state only counts once it has held this long, 0 = at once.
  heldMs: number
}

// You start wearing an avatar: this one, or any (null). Also when the app first sees it.
export type AvatarChangedTrigger = {
  kind: 'avatar-changed'
  avatarId: string | null
  avatarName: string | null
}

export type AppEvent =
  // A viewer uses a control: this one, any of this type (e.g. a shocker), or any.
  | { type: 'viewer-command'; controlId?: string | null; controlType?: string | null }
  | { type: 'viewer-joined' }
  // The share page goes from no viewers to some.
  | { type: 'first-viewer' }
  | { type: 'last-viewer-left' }
  // Stop everything is turned off.
  | { type: 'resumed' }

export type AppEventTrigger = { kind: 'app-event'; event: AppEvent }

export type AutomationTrigger = AvatarParameterTrigger | AvatarChangedTrigger | AppEventTrigger

// Has to be true for an automation to start; checked when it starts, not while it runs.
export type AutomationGuard =
  | {
      kind: 'parameter'
      parameter: string
      valueType: ParameterValueType
      condition: ParameterCondition
      not: boolean
    }
  // profileId null: No profile.
  | { kind: 'profile'; profileId: string | null }
  | { kind: 'viewers'; online: boolean }

// What "only if" needs from the app.
export interface AutomationContext {
  profileId: string | null
  viewers: number
}

export type OutputAction =
  | { type: 'pulse'; onMs: number; offMs: number; count: number }
  | { type: 'hold' }
  | { type: 'toggle' }
  | { type: 'off' }
  // Duty 0-1.
  | { type: 'follow'; min: number; max: number }

export type ParameterAction =
  // Bools are 0/1.
  | { type: 'set'; value: number }
  | { type: 'toggle' }
  // The value while the condition holds, then back to what it was.
  | { type: 'hold'; value: number }
  // A Float follows the trigger, -1 to 1.
  | { type: 'follow'; min: number; max: number }

export type BoardOutputAction = {
  kind: 'board-output'
  boardId: string
  pin: number
  output: OutputAction
  // Also says so in the chatbox (model.rs chatbox_text). Left out when off.
  chatbox?: boolean
}
export type AvatarParameterAction = {
  kind: 'avatar-parameter'
  // Full OSC address, e.g. /avatar/parameters/Ears.
  parameter: string
  valueType: ParameterValueType
  set: ParameterAction
}
// How a step runs a control. Values are what the control's command takes: a bool for toggles,
// an option index for enums, 0-1 for sliders and toys, a pattern id for patterns.
export type ControlRun =
  | { type: 'set'; value: boolean | number | string }
  | { type: 'toggle' }
  // A toy at `value` for `ms`, then off.
  | { type: 'pulse'; value: number; ms: number }
  // Presets and shockers.
  | { type: 'press' }
  | { type: 'hold'; value: boolean | number | string }
  | { type: 'follow'; min: number; max: number }

export type ControlAction = {
  kind: 'control'
  groupId: string
  controlId: string
  controlType: string
  run: ControlRun
}
// profileId null: No profile.
export type ProfileAction = { kind: 'profile'; profileId: string | null; whileHeld: boolean }
export type ChatboxStepAction = { kind: 'chatbox'; text: string }
export type StopEverythingAction = { kind: 'stop-everything' }

export type AutomationAction =
  | BoardOutputAction
  | AvatarParameterAction
  | ControlAction
  | ProfileAction
  | ChatboxStepAction
  | StopEverythingAction

// The backend asking the app to carry out a step (see useAutomationRunner.ts). `start` and `stop`
// bracket a hold; `follow` carries the trigger's value, 0-1.
export type AutomationRun = { automationId: string; step: number; action: AutomationAction } & (
  { phase: 'run' | 'start' | 'stop' } | { phase: 'follow'; value: number }
)

// One thing an automation does, starting `delayMs` after the trigger.
export type AutomationStep = AutomationAction & { delayMs: number }

export interface Automation {
  id: string
  name: string
  enabled: boolean
  when: AutomationTrigger
  then: AutomationStep[]
  // Minimum gap between two runs, 0 = none.
  cooldownMs: number
  // What a moment does while the previous run is still going.
  retrigger: 'restart' | 'ignore'
  // All of these have to be true for it to start.
  onlyIf: AutomationGuard[]
}

export interface AutomationActivity {
  id: string
  running: boolean
}

export const automationsApi = {
  automationsList: (): Promise<Automation[]> => invoke('automations_list'),
  // Rejects with a message to show.
  automationSave: (automation: Automation): Promise<void> =>
    invoke('automation_save', { automation }),
  automationDelete: (id: string): Promise<void> => invoke('automation_delete', { id }),
  automationSetEnabled: (id: string, enabled: boolean): Promise<void> =>
    invoke('automation_set_enabled', { id, enabled }),
  automationTest: (id: string): Promise<void> => invoke('automation_test', { id }),
  automationsSetPaused: (paused: boolean): Promise<void> =>
    invoke('automations_set_paused', { paused }),
  automationEvent: (event: AppEvent): Promise<void> => invoke('automation_event', { event }),
  automationsSetContext: (context: AutomationContext): Promise<void> =>
    invoke('automations_set_context', { context }),
  onAutomationsChanged: (callback: (automations: Automation[]) => void): void => {
    void listen<Automation[]>('automations-changed', (event) => callback(event.payload))
  },
  onAutomationActivity: (callback: (activity: AutomationActivity) => void): void => {
    void listen<AutomationActivity>('automation-activity', (event) => callback(event.payload))
  },
  // An automation started, with its id.
  onAutomationRan: (callback: (id: string) => void): void => {
    void listen<string>('automation-ran', (event) => callback(event.payload))
  },
  onAutomationRun: (callback: (run: AutomationRun) => void): void => {
    void listen<AutomationRun>('automation-run', (event) => callback(event.payload))
  },
  // An output step with "Send to chatbox" switched its output on or off; the text to post.
  onAutomationChatbox: (callback: (text: string) => void): void => {
    void listen<string>('automation-chatbox', (event) => callback(event.payload))
  }
}
