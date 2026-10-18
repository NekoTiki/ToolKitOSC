import { useAddons } from '@renderer/composables/useAddons'
import type { BhapticsPosition, BhapticsSettings, BhapticsState } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import _ from 'lodash'
import type { Ref } from 'vue'
import { ref, toRaw, watch } from 'vue'

// bHaptics runs in Rust (src-tauri/src/bhaptics): avatar touch plays on your bHaptics gear through
// bHaptics Player. This keeps a copy of bhaptics.json for Settings > bHaptics, saves edits back (a
// moment later, so a dragged slider isn't written on every step) and follows what plays.

// Each position's label, and how its motors are laid out for the live view: columns per row.
export const BHAPTICS_POSITIONS: Record<BhapticsPosition, { label: string; columns: number; motors: number }> = {
  VestFront: { label: 'Vest front', columns: 4, motors: 20 },
  VestBack: { label: 'Vest back', columns: 4, motors: 20 },
  Head: { label: 'Head', columns: 6, motors: 6 },
  ForearmL: { label: 'Left arm', columns: 3, motors: 6 },
  ForearmR: { label: 'Right arm', columns: 3, motors: 6 },
  HandL: { label: 'Left hand', columns: 3, motors: 3 },
  HandR: { label: 'Right hand', columns: 3, motors: 3 },
  GloveL: { label: 'Left glove', columns: 4, motors: 8 },
  GloveR: { label: 'Right glove', columns: 4, motors: 8 },
  FootL: { label: 'Left foot', columns: 3, motors: 3 },
  FootR: { label: 'Right foot', columns: 3, motors: 3 }
}

const EMPTY_STATE: BhapticsState = {
  status: 'disabled',
  installed: false,
  simulated: false,
  devices: [],
  connected: [],
  playing: {},
  touched: [],
  otherKeys: [],
  avatar: null,
  testing: null
}

// Module-scope singletons, shared by everything that shows the vest.
const settings = ref<BhapticsSettings | null>(null)
const state = ref<BhapticsState>(EMPTY_STATE)

const SAVE_DELAY_MS = 250
let saveTimer: ReturnType<typeof setTimeout> | undefined
let saving = 0

// Rust announces every save, ours included. While one of ours is pending, an older copy coming
// back would undo what was just changed, so it's ignored.
api.onBhapticsSettings((value) => {
  if (!saveTimer && !saving) settings.value = value
})
api.onBhapticsState((value) => (state.value = value))

const { isOn } = useAddons()

watch(
  () => isOn('bhaptics'),
  (on) => {
    if (!on) return
    void api.bhapticsSettings().then((value) => (settings.value = value)).catch(() => {})
    void api.bhapticsState().then((value) => (state.value = value)).catch(() => {})
  },
  { immediate: true }
)

const save = (): void => {
  clearTimeout(saveTimer)
  saveTimer = setTimeout(() => {
    saveTimer = undefined
    if (!settings.value) return
    saving++
    void api
      .bhapticsSave(_.cloneDeep(toRaw(settings.value)))
      .catch((error) => console.error('Could not save the bHaptics settings:', error))
      .finally(() => saving--)
  }, SAVE_DELAY_MS)
}

export function useBhaptics(): {
  settings: Ref<BhapticsSettings | null>
  state: Ref<BhapticsState>
  change: (mutate: (settings: BhapticsSettings) => void) => void
  strength: (position: BhapticsPosition) => number
} {
  const change = (mutate: (settings: BhapticsSettings) => void): void => {
    if (!settings.value) return

    mutate(settings.value)
    save()
  }

  const strength = (position: BhapticsPosition): number => settings.value?.positions[position]?.strength ?? 1

  return { settings, state, change, strength }
}
