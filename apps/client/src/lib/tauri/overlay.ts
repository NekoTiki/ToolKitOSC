// VR overlay - src-tauri/src/overlay/. Mirrors overlay/model.rs (what overlay.json holds) and the
// panel's presses (wrist/protocol.rs `ToApp`).
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export type OverlayTarget = 'auto' | 'xsoverlay' | 'ovrtoolkit' | 'off'

export interface OverlayNotify {
  to: OverlayTarget
  shocks: boolean
  toys: boolean
  controls: boolean
  viewers: boolean
  stop: boolean
  automations: boolean
}

export interface OverlayPanel {
  enabled: boolean
  hand: 'left' | 'right'
  widthCm: number
  opacity: number
  showWithinDeg: number
}

export interface OverlaySettings {
  notify: OverlayNotify
  panel: OverlayPanel
}

export type OverlayAction = { type: 'stop' } | { type: 'resume' } | { type: 'profile'; id: string | null }

export const overlayApi = {
  overlaySettings: (): Promise<OverlaySettings> => invoke('overlay_settings'),
  overlaySave: (settings: OverlaySettings): Promise<void> => invoke('overlay_save', { settings }),
  overlayTestToast: (): Promise<void> => invoke('overlay_test_toast'),
  overlaySetContext: (profiles: { id: string; name: string }[], profileId: string | null): Promise<void> =>
    invoke('overlay_set_context', { profiles, profileId }),
  // Something pressed on the wrist panel.
  onOverlayAction: (callback: (action: OverlayAction) => void): void => {
    void listen<OverlayAction>('overlay-action', (event) => callback(event.payload))
  }
}
