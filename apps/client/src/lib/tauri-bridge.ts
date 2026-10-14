// Replaces the old Electron preload's `window.api` (contextBridge) surface. Deliberately keeps
// the same method names/shapes as that API so the composables built on top of it (useOscMessages,
// useAvatarDetails) barely had to change when this app moved from Electron to Tauri — see
// apps/client's entry in the monorepo plan for the reasoning.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { AvatarDetails, OscCommand, OSCMessage, PresetStore } from '@toolkitosc/shared-ui'

// Whether an optional integration (SteamVR, VRCX) can be offered here - mirrors `Availability` in
// src-tauri/src/commands.rs. 'unsupported' means it can't work on this OS at all, 'not-installed'
// that it could but the app it hooks into wasn't found.
export type IntegrationAvailability = 'available' | 'not-installed' | 'unsupported'

// ESP32 boards - mirrors src-tauri/src/boards/mod.rs. 'needs-pairing': the board refused its token
// (reset, or paired to another PC since). 'update-firmware': it speaks another protocol version.
export type BoardStatus = 'connecting' | 'online' | 'offline' | 'needs-pairing' | 'update-firmware'

export interface BoardOutputPin {
  pin: number
  pwm: boolean
  // Silkscreen name, when the board's profile knows it.
  label?: string
}

// One output set up in the app ("Fan on GPIO 4"). The label stays in the app.
export interface BoardOutputConfig {
  pin: number
  label: string
  // For relay modules that switch on when the pin is LOW.
  activeLow: boolean
  maxOnMs: number
}

// A board refused something (see the firmware's PROTOCOL.md for the codes).
export interface BoardError {
  id: string
  code: string
  pin: number | null
}

export interface Board {
  id: string
  name: string
  host: string
  port: number
  fw: string | null
  chip: string | null
  board: string | null
  // Pins the board offers as outputs.
  outputs: BoardOutputPin[]
  // The outputs set up in the app, null until first saved.
  config: BoardOutputConfig[] | null
  status: BoardStatus
  // Unix ms, while offline.
  offlineSince: number | null
}

// The share server link - mirrors `Status` in src-tauri/src/server/mod.rs. 'rejected': the server
// refused the token (auth-error). 'mismatch': it speaks another protocol version. Neither retries.
export type ServerStatus = 'closed' | 'connecting' | 'open' | 'rejected' | 'mismatch'

export interface FoundBoard {
  id: string
  name: string
  host: string
  port: number
  fw: string | null
  proto: number | null
  // Some app holds a token for it.
  paired: boolean
  // Already in this app's list.
  added: boolean
}

export interface BoardsSnapshot {
  boards: Board[]
  found: FoundBoard[]
}

export type BoardPairingEvent =
  | { state: 'connecting'; name: string }
  | { state: 'waiting'; name: string; timeoutMs: number }
  | { state: 'paired'; id: string; name: string }
  | { state: 'failed'; reason: string }
  | { state: 'cancelled' }

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

export const api = {
  ready: (): void => {
    void invoke('ready')
  },
  openUrl: (url: string): void => {
    void invoke('open_url', { url })
  },
  intifaceCentralAvailable: (): Promise<boolean> => invoke('intiface_central_available'),
  startIntifaceCentral: (): Promise<void> => invoke('start_intiface_central'),
  intifacePortOpen: (host: string, port: number): Promise<boolean> =>
    invoke('intiface_port_open', { host, port }),
  intifaceCentralRunning: (): Promise<boolean> => invoke('intiface_central_running'),
  steamVrAvailable: (): Promise<IntegrationAvailability> => invoke('steamvr_available'),
  steamVrSetAutoLaunch: (enable: boolean): Promise<void> =>
    invoke('steamvr_set_auto_launch', { enable }),
  steamVrGetAutoLaunch: (): Promise<boolean> => invoke('steamvr_get_auto_launch'),
  vrcxAvailable: (): Promise<IntegrationAvailability> => invoke('vrcx_available'),
  vrcxSetAutoLaunch: (enable: boolean): Promise<void> => invoke('vrcx_set_auto_launch', { enable }),
  vrcxGetAutoLaunch: (): Promise<boolean> => invoke('vrcx_get_auto_launch'),
  setMinimizeToTray: (enabled: boolean): void => {
    void invoke('set_minimize_to_tray', { enabled })
  },
  onOscMessage: (callback: (msg: OSCMessage) => void): void => {
    void listen<OSCMessage>('vrc-osc-message', (event) => callback(event.payload))
  },
  onOscMessageBulk: (callback: (msg: OSCMessage[]) => void): void => {
    void listen<OSCMessage[]>('vrc-osc-message-bulk', (event) => callback(event.payload))
  },
  // Current values read from VRChat after an avatar loads (see the backend's
  // pull_on_avatar_load): merged into what's known, not a replacement like the bulk event.
  onOscMessageMerge: (callback: (msg: OSCMessage[]) => void): void => {
    void listen<OSCMessage[]>('vrc-osc-message-merge', (event) => callback(event.payload))
  },
  // Fired every time VRChat answers the backend's periodic avatar check (see poll_avatar_changes),
  // whether or not anything changed.
  onOscAlive: (callback: () => void): void => {
    void listen('vrc-osc-alive', () => callback())
  },
  sendOscMessage: (msg: OscCommand): void => {
    void invoke('send_osc_message', { msg })
  },
  onAvatarDetails: (callback: (details: AvatarDetails) => void): void => {
    void listen<AvatarDetails>('vrc-avatar-details', (event) => callback(event.payload))
  },
  loadPresets: (avatarId: string): Promise<PresetStore> => invoke('load_presets', { avatarId }),
  savePresets: (avatarId: string, store: PresetStore): Promise<void> =>
    invoke('save_presets', { avatarId, store }),
  forcePullParameters: (missingOnly: boolean): Promise<number> =>
    invoke('force_pull_parameters', { missingOnly }),
  boardsList: (): Promise<BoardsSnapshot> => invoke('boards_list'),
  // Both resolve once someone pressed the board's BOOT button, and reject with a message to show.
  boardAdd: (id: string): Promise<Board> => invoke('board_add', { id }),
  boardAddByAddress: (address: string): Promise<Board> =>
    invoke('board_add_by_address', { address }),
  boardPairCancel: (): Promise<void> => invoke('board_pair_cancel'),
  boardRemove: (id: string): Promise<void> => invoke('board_remove', { id }),
  boardSaveOutputs: (id: string, outputs: BoardOutputConfig[]): Promise<void> =>
    invoke('board_save_outputs', { id, outputs }),
  boardTestOutput: (id: string, pin: number): Promise<void> =>
    invoke('board_test_output', { id, pin }),
  boardsStopAll: (): Promise<void> => invoke('boards_stop_all'),
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
  },
  onBoardError: (callback: (error: BoardError) => void): void => {
    void listen<BoardError>('board-error', (event) => callback(event.payload))
  },
  onBoardsChanged: (callback: (snapshot: BoardsSnapshot) => void): void => {
    void listen<BoardsSnapshot>('boards-changed', (event) => callback(event.payload))
  },
  onBoardPairing: (callback: (event: BoardPairingEvent) => void): void => {
    void listen<BoardPairingEvent>('board-pairing', (event) => callback(event.payload))
  },
  // Asking again for the link that's already up keeps it and replays its status, auth-success and
  // client-list, which is how a reloaded webview catches up.
  serverConnect: (url: string, token: string | undefined, protocolVersion: number): Promise<void> =>
    invoke('server_connect', { url, token: token ?? null, protocolVersion }),
  serverDisconnect: (): Promise<void> => invoke('server_disconnect'),
  // One `{type, message}` envelope as JSON text. Dropped while the link is down.
  serverSend: (text: string): Promise<void> => invoke('server_send', { text }),
  // The avatar parameters the share page shows. Rust sends their values itself, now and on change.
  serverSetForwardedAddresses: (addresses: string[]): Promise<void> =>
    invoke('server_set_forwarded_addresses', { addresses }),
  // These three resolve once the listener is registered.
  onServerStatus: (callback: (status: ServerStatus) => void): Promise<unknown> =>
    listen<ServerStatus>('server-status', (event) => callback(event.payload)),
  // A message from the server, as JSON text.
  onServerMessage: (callback: (text: string) => void): Promise<unknown> =>
    listen<string>('server-message', (event) => callback(event.payload)),
  // Dev builds: a message Rust sent by itself (auth-token with the token blanked, args-update).
  onServerDebug: (callback: (text: string) => void): Promise<unknown> =>
    listen<string>('server-debug', (event) => callback(event.payload))
}

export type TauriApiType = typeof api
