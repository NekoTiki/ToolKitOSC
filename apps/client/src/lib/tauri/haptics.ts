// Avatar haptics - src-tauri/src/haptics/. Mirrors haptics/model.rs (what haptics.json holds) and
// the `haptics-state` event.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export interface HapticsShape {
  // Speed instead of depth: the toy responds to thrusting, not to holding still.
  followMovement: boolean
  // 0-1. Values under it read 0, the rest is stretched back to 0-1.
  deadZone: number
  // 0-1, the source's Max strength.
  scale: number
}

// Which zones of the source's kind it reads, by name. An empty `include` means every zone.
export interface HapticsZoneFilter {
  include: string[]
  exclude: string[]
}

interface SourceBase {
  zones: HapticsZoneFilter
  ownHands: boolean
  otherHands: boolean
  shape: HapticsShape
}

export interface HapticsSocketSource extends SourceBase {
  kind: 'sps-socket'
  myPlugs: boolean
  otherPlugs: boolean
  // Rubbing against other people's sockets.
  otherSockets: boolean
}

export interface HapticsPlugSource extends SourceBase {
  kind: 'sps-plug'
  mySockets: boolean
  otherSockets: boolean
  // Rubbing against other people's plugs.
  otherPlugs: boolean
}

export interface HapticsTouchSource extends SourceBase {
  kind: 'sps-touch'
}

export type HapticsSource = HapticsSocketSource | HapticsPlugSource | HapticsTouchSource

export interface HapticsActuator {
  deviceIndex: number
  actuatorIndex: number
  actuatorType: string
  deviceName: string
  actuatorDescription: string
}

export interface HapticsOutput {
  actuator: HapticsActuator
  sources: HapticsSource[]
  // 0-1, the cap for this actuator whatever its sources say.
  maxStrength: number
}

export interface AvatarHaptics {
  // The last name seen, for the page.
  name: string
  // Off: no haptics while wearing this avatar.
  enabled: boolean
  // Each replaces the global output for the same actuator.
  outputs: HapticsOutput[]
}

export interface HapticsSettings {
  enabled: boolean
  priority: { threshold: number; graceMs: number }
  outputs: HapticsOutput[]
  // Keyed by avatar id.
  avatars: Record<string, AvatarHaptics>
}

export type HapticsZoneKind = 'socket' | 'plug' | 'touch'

export interface HapticsZone {
  family: 'ogb' | 'vfh' | 'tps'
  kind: HapticsZoneKind
  name: string
  // The strongest contact on it right now, 0-1.
  level: number
}

export interface HapticsOutputLevel {
  deviceIndex: number
  actuatorIndex: number
  level: number
  // Each source's level, in order.
  sources: number[]
}

export interface HapticsState {
  avatarId: string | null
  zones: HapticsZone[]
  outputs: HapticsOutputLevel[]
}

export const hapticsApi = {
  hapticsSettings: (): Promise<HapticsSettings> => invoke('haptics_settings'),
  hapticsSave: (settings: HapticsSettings): Promise<void> => invoke('haptics_save', { settings }),
  hapticsState: (): Promise<HapticsState> => invoke('haptics_state'),
  onHapticsSettings: (callback: (settings: HapticsSettings) => void): void => {
    void listen<HapticsSettings>('haptics-settings', (event) => callback(event.payload))
  },
  onHapticsState: (callback: (state: HapticsState) => void): void => {
    void listen<HapticsState>('haptics-state', (event) => callback(event.payload))
  }
}
