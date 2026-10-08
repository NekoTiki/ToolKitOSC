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

export type AutomationTrigger = {
  kind: 'avatar-parameter'
  avatarId: string
  avatarName: string
  // Full OSC address, e.g. /avatar/parameters/Boop.
  parameter: string
  valueType: ParameterValueType
  condition: ParameterCondition
}

export type OutputAction =
  | { type: 'pulse'; onMs: number; offMs: number; count: number }
  | { type: 'hold' }
  | { type: 'toggle' }
  | { type: 'off' }
  // Duty 0-1.
  | { type: 'follow'; min: number; max: number }

export type AutomationAction = { kind: 'board-output'; boardId: string; pin: number; output: OutputAction }

export interface Automation {
  id: string
  name: string
  enabled: boolean
  when: AutomationTrigger
  then: AutomationAction
  // Minimum gap between two runs, 0 = none.
  cooldownMs: number
  // What a moment does while the previous run is still going.
  retrigger: 'restart' | 'ignore'
}

export interface AutomationActivity {
  id: string
  running: boolean
  // When it ends on its own (pulses, toggles).
  forMs: number | null
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
  intifacePortOpen: (host: string, port: number): Promise<boolean> => invoke('intiface_port_open', { host, port }),
  intifaceCentralRunning: (): Promise<boolean> => invoke('intiface_central_running'),
  steamVrAvailable: (): Promise<IntegrationAvailability> => invoke('steamvr_available'),
  steamVrSetAutoLaunch: (enable: boolean): Promise<void> => invoke('steamvr_set_auto_launch', { enable }),
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
  boardAddByAddress: (address: string): Promise<Board> => invoke('board_add_by_address', { address }),
  boardPairCancel: (): Promise<void> => invoke('board_pair_cancel'),
  boardRemove: (id: string): Promise<void> => invoke('board_remove', { id }),
  boardSaveOutputs: (id: string, outputs: BoardOutputConfig[]): Promise<void> =>
    invoke('board_save_outputs', { id, outputs }),
  boardTestOutput: (id: string, pin: number): Promise<void> => invoke('board_test_output', { id, pin }),
  boardsStopAll: (): Promise<void> => invoke('boards_stop_all'),
  automationsList: (): Promise<Automation[]> => invoke('automations_list'),
  // Rejects with a message to show.
  automationSave: (automation: Automation): Promise<void> => invoke('automation_save', { automation }),
  automationDelete: (id: string): Promise<void> => invoke('automation_delete', { id }),
  automationSetEnabled: (id: string, enabled: boolean): Promise<void> =>
    invoke('automation_set_enabled', { id, enabled }),
  automationTest: (id: string): Promise<void> => invoke('automation_test', { id }),
  automationsSetPaused: (paused: boolean): Promise<void> => invoke('automations_set_paused', { paused }),
  onAutomationsChanged: (callback: (automations: Automation[]) => void): void => {
    void listen<Automation[]>('automations-changed', (event) => callback(event.payload))
  },
  onAutomationActivity: (callback: (activity: AutomationActivity) => void): void => {
    void listen<AutomationActivity>('automation-activity', (event) => callback(event.payload))
  },
  onBoardError: (callback: (error: BoardError) => void): void => {
    void listen<BoardError>('board-error', (event) => callback(event.payload))
  },
  onBoardsChanged: (callback: (snapshot: BoardsSnapshot) => void): void => {
    void listen<BoardsSnapshot>('boards-changed', (event) => callback(event.payload))
  },
  onBoardPairing: (callback: (event: BoardPairingEvent) => void): void => {
    void listen<BoardPairingEvent>('board-pairing', (event) => callback(event.payload))
  }
}

export type TauriApiType = typeof api
