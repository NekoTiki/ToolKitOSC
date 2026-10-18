// VR overlay - src-tauri/src/overlay/. Mirrors overlay/model.rs (what overlay.json holds).
import { invoke } from '@tauri-apps/api/core'

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

export interface OverlaySettings {
  notify: OverlayNotify
}

export const overlayApi = {
  overlaySettings: (): Promise<OverlaySettings> => invoke('overlay_settings'),
  overlaySave: (settings: OverlaySettings): Promise<void> => invoke('overlay_save', { settings }),
  overlayTestToast: (): Promise<void> => invoke('overlay_test_toast')
}
