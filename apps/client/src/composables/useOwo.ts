import { useAddons } from '@renderer/composables/useAddons'
import type { OwoMuscle, OwoSettings, OwoSide, OwoState } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import _ from 'lodash'
import type { Ref } from 'vue'
import { ref, toRaw, watch } from 'vue'

// The OWO Skin runs in Rust (src-tauri/src/owo): avatar touch fires its muscles through the OWO
// app. This keeps a copy of owo.json for Settings > OWO, saves edits back (a moment later, so a
// dragged slider isn't written on every step) and follows what plays.

// Each muscle's label, its short name on the body view, and its side.
export const OWO_MUSCLES: Record<OwoMuscle, { label: string; short: string; side: OwoSide }> = {
  PectoralR: { label: 'Right chest', short: 'Chest R', side: 'front' },
  PectoralL: { label: 'Left chest', short: 'Chest L', side: 'front' },
  AbdominalR: { label: 'Right belly', short: 'Belly R', side: 'front' },
  AbdominalL: { label: 'Left belly', short: 'Belly L', side: 'front' },
  ArmR: { label: 'Right arm', short: 'Arm R', side: 'front' },
  ArmL: { label: 'Left arm', short: 'Arm L', side: 'front' },
  DorsalR: { label: 'Right upper back', short: 'Upper R', side: 'back' },
  DorsalL: { label: 'Left upper back', short: 'Upper L', side: 'back' },
  LumbarR: { label: 'Right lower back', short: 'Lower R', side: 'back' },
  LumbarL: { label: 'Left lower back', short: 'Lower L', side: 'back' }
}

export const OWO_DEFAULT_STRENGTH = 0.6

const EMPTY_STATE: OwoState = { status: 'disabled', address: null, simulated: false, playing: {}, touched: [], avatar: null, testing: null }

// Module-scope singletons, shared by everything that shows the suit.
const settings = ref<OwoSettings | null>(null)
const state = ref<OwoState>(EMPTY_STATE)

const SAVE_DELAY_MS = 250
let saveTimer: ReturnType<typeof setTimeout> | undefined
let saving = 0

// Rust announces every save, ours included. While one of ours is pending, an older copy coming
// back would undo what was just changed, so it's ignored.
api.onOwoSettings((value) => {
  if (!saveTimer && !saving) settings.value = value
})
api.onOwoState((value) => (state.value = value))

const { isOn } = useAddons()

watch(
  () => isOn('owo'),
  (on) => {
    if (!on) return
    void api.owoSettings().then((value) => (settings.value = value)).catch(() => {})
    void api.owoState().then((value) => (state.value = value)).catch(() => {})
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
      .owoSave(_.cloneDeep(toRaw(settings.value)))
      .catch((error) => console.error('Could not save the OWO settings:', error))
      .finally(() => saving--)
  }, SAVE_DELAY_MS)
}

export function useOwo(): {
  settings: Ref<OwoSettings | null>
  state: Ref<OwoState>
  change: (mutate: (settings: OwoSettings) => void) => void
  strength: (muscle: OwoMuscle) => number
} {
  const change = (mutate: (settings: OwoSettings) => void): void => {
    if (!settings.value) return

    mutate(settings.value)
    save()
  }

  const strength = (muscle: OwoMuscle): number => settings.value?.muscles[muscle]?.strength ?? OWO_DEFAULT_STRENGTH

  return { settings, state, change, strength }
}
