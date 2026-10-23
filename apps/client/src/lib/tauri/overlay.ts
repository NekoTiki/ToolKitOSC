// VR overlay - src-tauri/src/overlay/. Mirrors overlay/model.rs (what overlay.json holds) and the
// panel's presses (wrist/protocol.rs `ToApp`).
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

export interface OverlayPanel {
  enabled: boolean
  hand: 'left' | 'right'
  widthCm: number
  opacity: number
  showWithinDeg: number
}

export type OverlayCorner = 'bottom-left' | 'bottom-right' | 'top-left' | 'top-right'

export interface OverlayHud {
  enabled: boolean
  corner: OverlayCorner
  // Degrees in from the corner, 0-25.
  insetDeg: number
  // 0.5-2, 1 is the usual size.
  size: number
  opacity: number
  // Meters.
  distance: number
  follow: 'lazy' | 'glued'
  // 0.1-1.
  catchUp: number
  hideWithPanel: boolean
  status: boolean
  viewers: boolean
  lastAction: boolean
  toys: boolean
  leash: boolean
  warnings: boolean
}

export interface OverlaySettings {
  notify: OverlayNotify
  panel: OverlayPanel
  hud: OverlayHud
  // Avatar id -> the controls pinned to the wrist (set with overlaySetPinned).
  pinned: Record<string, string[]>
}

// What each part of the overlay is doing (see overlay/commands.rs `OverlayStatus`).
export interface OverlayStatus {
  inVr: boolean
  gaveUp: boolean
  toastsTo: OverlayTarget | null
}

export type Rgb = [number, number, number]

// The colors the wrist panel draws with.
export interface OverlayTheme {
  primary: Rgb
  secondary: Rgb
}

export const overlayApi = {
  overlaySettings: (): Promise<OverlaySettings> => invoke('overlay_settings'),
  overlaySave: (settings: OverlaySettings): Promise<void> => invoke('overlay_save', { settings }),
  overlayTestToast: (): Promise<void> => invoke('overlay_test_toast'),
  overlayStatus: (): Promise<OverlayStatus> => invoke('overlay_status'),
  // Your theme colors, for the panel. Rust knows the profiles and viewer groups itself.
  overlaySetTheme: (theme: OverlayTheme | null): Promise<void> => invoke('overlay_set_theme', { theme }),
  // The controls pinned to your wrist on the avatar you're wearing.
  overlayPinned: (): Promise<string[]> => invoke('overlay_pinned'),
  overlaySetPinned: (controlId: string, pinned: boolean): Promise<void> => invoke('overlay_set_pinned', { controlId, pinned })
}
