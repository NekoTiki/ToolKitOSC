// Avatar leash - src-tauri/src/leash/. Mirrors leash/model.rs (what leash.json holds) and the
// `leash-state` event.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export interface LeashTurning {
  enabled: boolean
  multiplier: number
  deadzone: number
  goalDegrees: number
}

export interface AvatarLeash {
  name: string
  walk: number
  run: number
  strength: number
  upDownDeadzone: number
  upDownCompensation: number
  turning: LeashTurning
  // Which grabbable PhysBones move you. One missing here moves you when its name has "leash".
  leashes: Record<string, boolean>
}

export interface LeashSettings {
  // "Leash moves me", also TKOSC/Leash.
  enabled: boolean
  avatars: Record<string, AvatarLeash>
}

export type LeashProblem =
  | { kind: 'no-leash' }
  | { kind: 'missing-stretch'; leash: string }
  | { kind: 'no-compass' }
  | { kind: 'missing-directions'; missing: string[] }
  | { kind: 'wrong-type'; parameter: string; expected: string; found: string }

export type LeashCheck = 'waiting' | 'passed' | 'failed'
export type LeashMotion = 'idle' | 'grabbed' | 'walking' | 'running'

export interface LeashState {
  avatarId: string | null
  avatarName: string | null
  detected: { leashes: string[]; compass: string | null; problems: LeashProblem[] }
  checks: { stretches: LeashCheck; compass: LeashCheck; needle: LeashCheck }
  held: string[]
  active: string | null
  stretch: number
  directions: { zPos: number; zNeg: number; xPos: number; xNeg: number; yPos: number; yNeg: number }
  output: { vertical: number; horizontal: number; look: number; run: boolean; motion: LeashMotion }
  // Why a held leash doesn't move you right now.
  blocked: 'paused' | 'off' | 'silent' | null
  testing: boolean
}

export const leashApi = {
  leashSettings: (): Promise<LeashSettings> => invoke('leash_settings'),
  leashSave: (settings: LeashSettings): Promise<void> => invoke('leash_save', { settings }),
  leashState: (): Promise<LeashState> => invoke('leash_state'),
  leashTestPull: (): Promise<void> => invoke('leash_test_pull'),
  onLeashSettings: (callback: (settings: LeashSettings) => void): void => {
    void listen<LeashSettings>('leash-settings', (event) => callback(event.payload))
  },
  onLeashState: (callback: (state: LeashState) => void): void => {
    void listen<LeashState>('leash-state', (event) => callback(event.payload))
  }
}
