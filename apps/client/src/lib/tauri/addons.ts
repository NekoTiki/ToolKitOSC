// Add-ons - src-tauri/src/addons/. Only the app window turns them on or off.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// What an add-on is and what it brings - mirrors `Manifest` in src-tauri/src/addons/mod.rs.
export interface AddonManifest {
  id: string
  name: string
  // Which group of cards it's in on the Add-ons page.
  group: string
  // One line on what it does, shown while it's off.
  pitch: string
  platforms: string[]
  // A safety note shown as a badge, e.g. 'shocks'.
  badge: string | null
  // Add-ons it can't work without.
  needs: string[]
  // Add-ons it needs at least one of (avatar haptics: Intiface toys or bHaptics).
  needsAny: string[]
  controlTypes: string[]
  automationKinds: string[]
  // Rail pages it brings, by route name.
  pages: string[]
}

// 'busy': connecting, checking or starting. 'warn': works, but something needs you.
export type AddonState = 'off' | 'busy' | 'ok' | 'warn' | 'error'

export interface Addon extends AddonManifest {
  enabled: boolean
  status: { state: AddonState; text: string }
}

export interface AddonsState {
  addons: Addon[]
  // Whether "What do you use?" was answered (or an add-on was already in use before).
  setupDone: boolean
  // What the first launch with add-ons kept on, until its notice is dismissed.
  kept: string[]
}

// An add-on Stop everything couldn't reach.
export interface AddonStopFailure {
  id: string
  message: string
}

// An automation step was skipped because its add-on is off. Once per automation.
export interface AutomationSkipped {
  automationId: string
  message: string
}

export const addonsApi = {
  addonsList: (): Promise<AddonsState> => invoke('addons_list'),
  // Starts or stops it right away. Turning one off stops its devices first.
  addonsSetEnabled: (id: string, enabled: boolean): Promise<void> => invoke('addons_set_enabled', { id, enabled }),
  addonsFinishSetup: (dismissKept: boolean): Promise<void> => invoke('addons_finish_setup', { dismissKept }),
  addonsStopEverything: (): Promise<AddonStopFailure[]> => invoke('addons_stop_everything'),
  onAddonsUpdate: (callback: (state: AddonsState) => void): void => {
    void listen<AddonsState>('addons-update', (event) => callback(event.payload))
  },
  onAutomationSkipped: (callback: (skipped: AutomationSkipped) => void): void => {
    void listen<AutomationSkipped>('automation-skipped', (event) => callback(event.payload))
  }
}
