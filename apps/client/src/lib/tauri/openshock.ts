// OpenShock - src-tauri/src/openshock/. The token never comes back from Rust.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { OpenShockControlValue } from '@toolkitosc/shared-ui'

// OpenShock - mirrors src-tauri/src/openshock/mod.rs. The token never comes back from Rust.
export interface OpenShockShocker {
  id: string
  name: string
  isPaused: boolean
}

// GET /1/shockers/own returns one entry per hub (the device), each grouping the shockers paired
// to it.
export interface OpenShockHub {
  id: string
  name: string
  shockers: OpenShockShocker[]
}

export interface OpenShockState {
  enabled: boolean
  hasToken: boolean
  status: 'unconfigured' | 'checking' | 'valid' | 'invalid'
  hubs: OpenShockHub[]
  // Whether Rust is connected to OpenShock's live hub. Only then is `online` known.
  live: boolean
  // Ids of the hubs that are online.
  online: string[]
  // Where the token is saved: the system's password store, or a file when there's none.
  secretStore: 'keyring' | 'file'
}

// One rolled shock or vibration. Rust sends it once `delayMs` (the reel animation) is over.
export interface OpenShockFire {
  shockers: string[]
  mode: 'Shock' | 'Vibrate'
  intensity: number
  duration: number
  delayMs: number
  // Shown in the OpenShock log after "ToolKitOSC".
  viewerName: string | null
}

export const openshockApi = {
  onShockValue: (callback: (value: { controlId: string; value: OpenShockControlValue }) => void): void => {
    void listen<{ controlId: string; value: OpenShockControlValue }>('shock-value', (event) => callback(event.payload))
  },
  openShockState: (): Promise<OpenShockState> => invoke('openshock_state'),
  // Once, from the values the webview used to keep in localStorage.
  openShockMigrate: (token: string | null, enabled: boolean | null): Promise<void> =>
    invoke('openshock_migrate', { token, enabled }),
  // An empty token removes it. Rejects with a message when the password store refuses it.
  openShockSetToken: (token: string): Promise<void> => invoke('openshock_set_token', { token }),
  openShockSetEnabled: (enabled: boolean): Promise<void> => invoke('openshock_set_enabled', { enabled }),
  openShockRecheck: (): Promise<void> => invoke('openshock_recheck'),
  openShockShockers: (): Promise<OpenShockHub[]> => invoke('openshock_shockers'),
  // Rejects when OpenShock isn't available. A failure while sending arrives as onOpenShockError.
  openShockFire: (fire: OpenShockFire): Promise<void> => invoke('openshock_fire', { fire }),
  // Drops pending shocks and stops every shocker. Rejects when OpenShock can't be reached.
  openShockStopAll: (): Promise<void> => invoke('openshock_stop_all'),
  onOpenShockState: (callback: (state: OpenShockState) => void): void => {
    void listen<OpenShockState>('openshock-state', (event) => callback(event.payload))
  },
  onOpenShockError: (callback: (message: string) => void): void => {
    void listen<string>('openshock-error', (event) => callback(event.payload))
  }
}
