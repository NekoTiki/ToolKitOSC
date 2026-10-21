import { useAddons } from '@renderer/composables/useAddons'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import { useStopEverything } from '@renderer/composables/useStopEverything'
import { useTheme } from '@renderer/composables/useTheme'
import { useViewerGroups } from '@renderer/composables/useViewerGroups'
import type { OverlayTheme, Rgb } from '@renderer/lib/tauri/overlay'
import { api } from '@renderer/lib/tauri-bridge'
import type { Ref } from 'vue'
import { ref, watch } from 'vue'

// The controls pinned to your wrist on the avatar you're wearing (overlay.json, in Rust). Shared by
// the Controls page's right-click menu and the control editor.
const wristPins = ref<string[]>([])

const loadWristPins = (): void => {
  api.overlayPinned().then((ids) => (wristPins.value = ids)).catch(() => {})
}

// A theme color as RGB, for the panel: Tailwind's colors are oklch, which a 1x1 canvas converts.
// Null when this webview can't read it.
const rgb = (variable: string): Rgb | null => {
  const context = document.createElement('canvas').getContext('2d', { willReadFrequently: true })
  const color = getComputedStyle(document.documentElement).getPropertyValue(variable).trim()
  if (!context || !color) return null

  // A color the canvas can't parse leaves fillStyle as it was.
  context.fillStyle = '#000001'
  context.fillStyle = color
  if (context.fillStyle === '#000001') return null

  context.fillRect(0, 0, 1, 1)
  const [r, g, b] = context.getImageData(0, 0, 1, 1).data

  return [r, g, b]
}

// The panel is always dark, so it takes the shades the dark theme uses.
const panelTheme = (): OverlayTheme | null => {
  const primary = rgb('--ui-color-primary-400')
  const secondary = rgb('--ui-color-secondary-400')

  return primary && secondary ? { primary, secondary } : null
}

// The wrist panel (src-tauri/src/wrist) runs in Rust, but Stop everything's pause and the profiles
// live here: this tells Rust the profiles, the viewer groups' names and your theme colors, and
// carries out Stop, Resume and profile changes pressed on the panel. Called once, from App.vue.
export function useVrOverlay(): void {
  const { lockedControlGroups, currentLockedControlsGroup } = useLockedControls()
  const { stop, resume } = useStopEverything()
  const { viewerGroups } = useViewerGroups()
  const { isOn } = useAddons()
  const { selectedPrimary, selectedSecondary } = useTheme()

  watch(
    [
      () => lockedControlGroups.value.map(({ id, name }) => ({ id, name })),
      currentLockedControlsGroup,
      () => viewerGroups.value.map(({ id, name }) => ({ id, name })),
      selectedPrimary,
      selectedSecondary
    ],
    ([profiles, profileId, groups]) => void api.overlaySetContext(profiles, profileId, groups, panelTheme()),
    { immediate: true, deep: true }
  )

  // Pins are per avatar.
  watch(() => isOn('overlay'), (on) => on && loadWristPins(), { immediate: true })
  useAvatarDetails(() => loadWristPins())

  api.onOverlayAction((action) => {
    switch (action.type) {
      case 'stop':
        stop()
        break
      case 'resume':
        resume()
        break
      case 'profile':
        // A profile deleted since is ignored.
        if (action.id === null || lockedControlGroups.value.some((p) => p.id === action.id)) currentLockedControlsGroup.value = action.id
        break
    }
  })
}

export function useWristPins(): {
  wristPins: Ref<string[]>
  // Pins or unpins a control of the avatar you're wearing, right away.
  setWristPin: (controlId: string, pinned: boolean) => void
} {
  const toast = useToast()

  const setWristPin = (controlId: string, pinned: boolean): void => {
    const before = wristPins.value
    wristPins.value = pinned ? [...before.filter((id) => id !== controlId), controlId] : before.filter((id) => id !== controlId)
    api.overlaySetPinned(controlId, pinned).catch((error) => {
      wristPins.value = before
      toast.add({ title: 'Could not pin it to your wrist', description: String(error), color: 'error' })
    })
  }

  return { wristPins, setWristPin }
}
