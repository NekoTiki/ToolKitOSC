// bHaptics - src-tauri/src/bhaptics/. Mirrors bhaptics/model.rs (what bhaptics.json holds) and the
// `bhaptics-state` event.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// Player's names for each place on the body, as bhaptics/model.rs `Position` has them.
export type BhapticsPosition =
  | 'Head'
  | 'VestFront'
  | 'VestBack'
  | 'ForearmL'
  | 'ForearmR'
  | 'HandL'
  | 'HandR'
  | 'GloveL'
  | 'GloveR'
  | 'FootL'
  | 'FootR'

export interface BhapticsSettings {
  // "Use bHaptics", also TKOSC/bHaptics. Off mutes avatar touch only.
  enabled: boolean
  // Who reads the avatar's touch parameters: the app, or bHaptics Player's own VRChat handling.
  touchSource: 'toolkitosc' | 'player'
  touchFrom: { self: boolean; others: boolean }
  // 0 to 1. A position missing here plays at full strength.
  positions: Partial<Record<BhapticsPosition, { strength: number }>>
}

export type BhapticsStatus = 'disabled' | 'connecting' | 'connected' | 'not-running'

export interface BhapticsDeviceCheck {
  position: BhapticsPosition
  receivers: number
  motors: number
  own: boolean
  others: boolean
}

export interface BhapticsAvatarCheck {
  id: string
  name: string
  schemes: ('v2' | 'v2m' | 'v1' | 'legacy')[]
  devices: BhapticsDeviceCheck[]
  spsZones: number
  switch: boolean
}

export interface BhapticsState {
  status: BhapticsStatus
  installed: boolean
  simulated: boolean
  devices: string[]
  connected: BhapticsPosition[]
  // 0-100 per motor, for positions that play now.
  playing: Partial<Record<BhapticsPosition, number[]>>
  touched: BhapticsPosition[]
  // Keys another app plays on Player right now (usually Player's own VRChat handling).
  otherKeys: string[]
  avatar: BhapticsAvatarCheck | null
  testing: BhapticsPosition | null
}

export const bhapticsApi = {
  bhapticsSettings: (): Promise<BhapticsSettings> => invoke('bhaptics_settings'),
  bhapticsSave: (settings: BhapticsSettings): Promise<void> => invoke('bhaptics_save', { settings }),
  bhapticsState: (): Promise<BhapticsState> => invoke('bhaptics_state'),
  bhapticsLaunchPlayer: (): Promise<void> => invoke('bhaptics_launch_player'),
  bhapticsSimulate: (simulated: boolean): Promise<void> => invoke('bhaptics_simulate', { simulated }),
  bhapticsTestMotors: (position: BhapticsPosition): Promise<void> => invoke('bhaptics_test_motors', { position }),
  bhapticsDiagnostics: (): Promise<string> => invoke('bhaptics_diagnostics'),
  onBhapticsSettings: (callback: (settings: BhapticsSettings) => void): void => {
    void listen<BhapticsSettings>('bhaptics-settings', (event) => callback(event.payload))
  },
  onBhapticsState: (callback: (state: BhapticsState) => void): void => {
    void listen<BhapticsState>('bhaptics-state', (event) => callback(event.payload))
  }
}
