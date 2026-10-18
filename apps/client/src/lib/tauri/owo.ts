// OWO - src-tauri/src/owo/. Mirrors owo/model.rs (what owo.json holds) and the `owo-state` event.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// The OWO Skin's muscle groups, as owo/model.rs `Muscle` has them.
export type OwoMuscle =
  | 'PectoralR'
  | 'PectoralL'
  | 'AbdominalR'
  | 'AbdominalL'
  | 'ArmR'
  | 'ArmL'
  | 'DorsalR'
  | 'DorsalL'
  | 'LumbarR'
  | 'LumbarL'

export type OwoSide = 'front' | 'back'

export interface OwoSettings {
  // "Use the OWO Skin", also TKOSC/OWO. Off mutes avatar touch only.
  enabled: boolean
  // The OWO app's IP, or ip:port. null finds it by broadcast.
  address: string | null
  // Hz, 1 to 100.
  frequency: number
  // 0 to 1. A muscle missing here plays at 0.6.
  muscles: Partial<Record<OwoMuscle, { strength: number }>>
  useAvatarIntensity: boolean
}

export type OwoStatus = 'disabled' | 'connecting' | 'connected'

export interface OwoAvatarCheck {
  id: string
  name: string
  // The muscles it has a receiver for.
  muscles: OwoMuscle[]
  intensity: boolean
  spsZones: number
  switch: boolean
}

export interface OwoState {
  status: OwoStatus
  // Where the OWO app answered from.
  address: string | null
  simulated: boolean
  // 0-100, for muscles that play now.
  playing: Partial<Record<OwoMuscle, number>>
  touched: OwoSide[]
  avatar: OwoAvatarCheck | null
  testing: OwoMuscle | null
}

// One packet the OWO tester got (development builds only).
export interface OwoTesterEntry {
  atMs: number
  text: string
  muscles: Partial<Record<OwoMuscle, number>>
  ms: number
  stop: boolean
}

export const owoApi = {
  owoSettings: (): Promise<OwoSettings> => invoke('owo_settings'),
  owoSave: (settings: OwoSettings): Promise<void> => invoke('owo_save', { settings }),
  owoState: (): Promise<OwoState> => invoke('owo_state'),
  owoSimulate: (simulated: boolean): Promise<void> => invoke('owo_simulate', { simulated }),
  owoTestMuscles: (): Promise<void> => invoke('owo_test_muscles'),
  owoTesterLog: (): Promise<OwoTesterEntry[]> => invoke('owo_tester_log'),
  onOwoSettings: (callback: (settings: OwoSettings) => void): void => {
    void listen<OwoSettings>('owo-settings', (event) => callback(event.payload))
  },
  onOwoState: (callback: (state: OwoState) => void): void => {
    void listen<OwoState>('owo-state', (event) => callback(event.payload))
  }
}
