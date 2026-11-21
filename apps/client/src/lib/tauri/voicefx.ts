// Voice effects - src-tauri/src/voicefx/. Mirrors voicefx/model.rs (what voicefx.json holds) and
// the `voicefx-state` event.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { VoiceFxControlState, VoiceFxEffect } from '@toolkitosc/shared-ui'

export interface VoiceFxMouth {
  enabled: boolean
  // Follows the ticked sockets. Otherwise every socket whose name looks like a mouth.
  custom: boolean
  // Kept while custom is off.
  sockets: string[]
  others: boolean
  own: boolean
  hands: boolean
  effect: VoiceFxEffect
  // Strength at the tip and all the way in, 0-1.
  tip: number
  deep: number
  releaseMs: number
}

export interface VoiceFxSettings {
  micMode: 'one' | 'split'
  // Device names. null: the system's default microphone.
  mic: string | null
  micVr: string | null
  // The cable VRChat listens to.
  output: string | null
  monitor: boolean
  lastVr: boolean
  defaultEffect: VoiceFxEffect
  level: number
  mouth: VoiceFxMouth
}

export type VoiceFxSource = { kind: 'you' } | { kind: 'avatar' } | { kind: 'mouth' } | { kind: 'automation' } | { kind: 'control'; id: string }

export interface VoiceFxHold {
  source: VoiceFxSource
  // The viewer behind a control's hold.
  by: string | null
  effect: VoiceFxEffect
  level: number
  // When a timed hold ends, in ms since 1970.
  endsAt: number | null
  playing: boolean
}

export interface VoiceFxState {
  // The one playing first, then the ones waiting.
  holds: VoiceFxHold[]
  controls: Record<string, VoiceFxControlState>
  paused: boolean
  running: boolean
  error: string | null
  mic: string | null
  micFallback: boolean
  vr: boolean
  peak: number
  sockets: { name: string; depth: number }[]
  // The sockets the mouth link follows.
  mouthSockets: string[]
}

export interface VoiceFxDevices {
  inputs: string[]
  outputs: string[]
}

export const voicefxApi = {
  voicefxSettings: (): Promise<VoiceFxSettings> => invoke('voicefx_settings'),
  voicefxSave: (settings: VoiceFxSettings): Promise<void> => invoke('voicefx_save', { settings }),
  voicefxState: (): Promise<VoiceFxState> => invoke('voicefx_state'),
  voicefxDevices: (): Promise<VoiceFxDevices> => invoke('voicefx_devices'),
  voicefxHold: (on: boolean): Promise<void> => invoke('voicefx_hold', { on }),
  voicefxRelease: (source: VoiceFxSource): Promise<void> => invoke('voicefx_release', { source }),
  onVoiceFxSettings: (callback: (settings: VoiceFxSettings) => void): void => {
    void listen<VoiceFxSettings>('voicefx-settings', (event) => callback(event.payload))
  },
  onVoiceFxState: (callback: (state: VoiceFxState) => void): void => {
    void listen<VoiceFxState>('voicefx-state', (event) => callback(event.payload))
  }
}
