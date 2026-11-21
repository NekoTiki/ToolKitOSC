import { useAddons } from '@renderer/composables/useAddons'
import type { VoiceFxSettings, VoiceFxState } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import { useVoiceFxControl } from '@toolkitosc/shared-ui'
import _ from 'lodash'
import type { Ref } from 'vue'
import { ref, toRaw, watch } from 'vue'

// Voice effects run in Rust (src-tauri/src/voicefx): your microphone goes through the app on its
// way to VRChat. This keeps a copy of voicefx.json for the page, saves edits back (a moment later,
// so a dragged slider isn't written on every step) and follows what's held.

const EMPTY_STATE: VoiceFxState = {
  holds: [],
  controls: {},
  paused: false,
  running: false,
  error: null,
  mic: null,
  micFallback: false,
  vr: false,
  peak: 0,
  sockets: [],
  mouthSocket: null
}

// Module-scope singletons, shared by the page and the tiles.
const settings = ref<VoiceFxSettings | null>(null)
const state = ref<VoiceFxState>(EMPTY_STATE)

const SAVE_DELAY_MS = 250
let saveTimer: ReturnType<typeof setTimeout> | undefined
let saving = 0

const follow = (value: VoiceFxState): void => {
  state.value = value
  // The tiles, and through useWebsocketHost.ts the share page.
  useVoiceFxControl().setControls(value.controls)
}

// Rust announces every save, ours included. While one of ours is pending, an older copy coming
// back would undo what was just changed, so it's ignored.
api.onVoiceFxSettings((value) => {
  if (!saveTimer && !saving) settings.value = value
})
api.onVoiceFxState(follow)

const { isOn } = useAddons()

watch(
  () => isOn('voicefx'),
  (on) => {
    if (!on) return follow(EMPTY_STATE)
    void api.voicefxSettings().then((value) => (settings.value = value)).catch(() => {})
    void api.voicefxState().then(follow).catch(() => {})
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
      .voicefxSave(_.cloneDeep(toRaw(settings.value)))
      .catch((error) => console.error('Could not save the voice effects settings:', error))
      .finally(() => saving--)
  }, SAVE_DELAY_MS)
}

export function useVoiceFx(): {
  settings: Ref<VoiceFxSettings | null>
  state: Ref<VoiceFxState>
  change: (mutate: (settings: VoiceFxSettings) => void) => void
} {
  const change = (mutate: (settings: VoiceFxSettings) => void): void => {
    if (!settings.value) return

    mutate(settings.value)
    save()
  }

  return { settings, state, change }
}
