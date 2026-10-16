// Intiface, toy levels and patterns - src-tauri/src/intiface/ and toys/.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// Intiface - mirrors src-tauri/src/intiface. See useIntiface.ts for what each status means.
export type IntifaceStatus = 'disabled' | 'connecting' | 'connected' | 'error' | 'refused' | 'busy' | 'launching'

export interface IntifaceActuator {
  // Its position in the device's ScalarCmd list.
  index: number
  description: string
  actuatorType: string
}

export interface IntifaceDevice {
  index: number
  name: string
  actuators: IntifaceActuator[]
  batterySensorIndex: number | null
  // Once a reading came back.
  battery: number | null
}

export interface IntifaceState {
  // Empty for the default.
  url: string
  defaultUrl: string
  enabled: boolean
  autoLaunch: boolean
  status: IntifaceStatus
  devices: IntifaceDevice[]
  // Whether a local Intiface Central install was found.
  installed: boolean
  launchAttempts: number
  maxLaunchAttempts: number
  // Intiface's own message when it refused the handshake.
  refusedReason: string
}

export interface ToyActuatorRef {
  deviceIndex: number
  actuatorIndex: number
  actuatorType: string
}

// What a pattern plays: a built-in or saved pattern by id, or samples the caller holds.
export interface PatternSource {
  id: string
  // Intensities 0-100, one per 50 ms. When set, played instead of looking the id up.
  samples?: number[]
}

export interface SavedPatternData {
  id: string
  name: string
  icon?: string
  samples: number[]
  updatedAt: number
}

// An actuator avatar touch is driving right now (see src-tauri/src/haptics).
export interface ToyTouched {
  deviceIndex: number
  actuatorIndex: number
}

export interface PatternValueEvent {
  controlId: string
  value: string
  info: { by?: string; endsAt?: number } | null
}

export const toysApi = {
  intifaceState: (): Promise<IntifaceState> => invoke('intiface_state'),
  // Once, from the settings the webview used to keep in localStorage.
  intifaceMigrate: (url: string | null, enabled: boolean | null, autoLaunch: boolean | null): Promise<void> =>
    invoke('intiface_migrate', { url, enabled, autoLaunch }),
  // Empty goes back to the default URL. Reconnects right away.
  intifaceSetUrl: (url: string | null): Promise<void> => invoke('intiface_set_url', { url }),
  intifaceSetAutoLaunch: (autoLaunch: boolean): Promise<void> => invoke('intiface_set_auto_launch', { autoLaunch }),
  // Opens the local Intiface Central install. Rejects with a message when it isn't found.
  intifaceLaunch: (): Promise<void> => invoke('intiface_launch'),
  onIntifaceState: (callback: (state: IntifaceState) => void): void => {
    void listen<IntifaceState>('intiface-state', (event) => callback(event.payload))
  },
  // Sets these actuators to `level` (0-1). Only toys Intiface has now are sent to.
  toysSet: (actuators: ToyActuatorRef[], level: number): Promise<void> => invoke('toys_set', { actuators, level }),
  // Stops every connected toy, not only the ones a control drives.
  toysStopAll: (): Promise<void> => invoke('toys_stop_all'),
  // Plays a pattern for this control after taking its actuators from any other pattern. Resolves
  // to the run's number (onPatternEnded carries it), or null for an unknown pattern.
  patternsPlay: (
    controlId: string,
    source: PatternSource,
    actuators: ToyActuatorRef[],
    stopAfterMs?: number | null,
    by?: string
  ): Promise<number | null> => invoke('patterns_play', { controlId, source, actuators, stopAfterMs: stopAfterMs ?? null, by: by ?? null }),
  // Stops this control's pattern and zeroes its toys.
  patternsOff: (controlId: string, actuators: ToyActuatorRef[]): Promise<void> => invoke('patterns_off', { controlId, actuators }),
  // Stops this control's pattern without touching its toys.
  patternsStop: (controlId: string): Promise<void> => invoke('patterns_stop', { controlId }),
  // Stops every other pattern on these actuators, before something else drives them.
  patternsRelease: (actuators: ToyActuatorRef[], except?: string): Promise<void> =>
    invoke('patterns_release', { actuators, except: except ?? null }),
  patternsStopAll: (): Promise<void> => invoke('patterns_stop_all'),
  patternsSavedList: (): Promise<SavedPatternData[]> => invoke('patterns_saved_list'),
  // Adds a recorded pattern, or replaces the one with its id.
  patternsSave: (pattern: SavedPatternData): Promise<void> => invoke('patterns_save', { pattern }),
  patternsDelete: (id: string): Promise<void> => invoke('patterns_delete', { id }),
  // Once, from the recordings the webview used to keep in localStorage.
  patternsMigrate: (patterns: SavedPatternData[]): Promise<void> => invoke('patterns_migrate', { patterns }),
  onSavedPatterns: (callback: (patterns: SavedPatternData[]) => void): void => {
    void listen<SavedPatternData[]>('saved-patterns', (event) => callback(event.payload))
  },
  // A control's pattern changed in Rust (started, stopped, taken over by another, timed out).
  onPatternValue: (callback: (value: PatternValueEvent) => void): void => {
    void listen<PatternValueEvent>('pattern-value', (event) => callback(event.payload))
  },
  onPatternEnded: (callback: (ended: { controlId: string; run: number }) => void): void => {
    void listen<{ controlId: string; run: number }>('pattern-ended', (event) => callback(event.payload))
  },
  // The actuators avatar touch drives right now. Empty while haptics are off.
  toysTouched: (): Promise<ToyTouched[]> => invoke('toys_touched'),
  onToysState: (callback: (state: { touched: ToyTouched[] }) => void): void => {
    void listen<{ touched: ToyTouched[] }>('toys-state', (event) => callback(event.payload))
  },
  onToyValue: (callback: (value: { controlId: string; value: number }) => void): void => {
    void listen<{ controlId: string; value: number }>('toy-value', (event) => callback(event.payload))
  }
}
