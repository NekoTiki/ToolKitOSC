import { useAddons } from '@renderer/composables/useAddons'
import type { AvatarLeash, LeashSettings, LeashState } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import _ from 'lodash'
import type { ComputedRef, Ref } from 'vue'
import { computed, ref, toRaw, watch } from 'vue'

// The avatar leash runs in Rust (src-tauri/src/leash): a pull on your avatar's leash walks you.
// This keeps a copy of leash.json for the Leash page, saves edits back (a moment later, so a
// dragged slider isn't written on every step) and follows the live pull.

// OSCLeash's defaults, as leash/model.rs has them.
export const DEFAULT_AVATAR_LEASH: AvatarLeash = {
  name: '',
  walk: 0.15,
  run: 0.7,
  strength: 1.2,
  upDownDeadzone: 0.5,
  upDownCompensation: 1,
  turning: { enabled: false, multiplier: 0.8, deadzone: 0.15, goalDegrees: 90 },
  leashes: {}
}

// Only the bones called a leash move you until you pick others (`default_on` in leash/model.rs).
export const movesYou = (avatar: AvatarLeash, leash: string): boolean =>
  avatar.leashes[leash] ?? leash.toLowerCase().includes('leash')

const EMPTY_STATE: LeashState = {
  avatarId: null,
  avatarName: null,
  detected: { leashes: [], compass: null, problems: [] },
  checks: { stretches: 'waiting', compass: 'waiting', needle: 'waiting' },
  held: [],
  active: null,
  stretch: 0,
  directions: { zPos: 0, zNeg: 0, xPos: 0, xNeg: 0, yPos: 0, yNeg: 0 },
  output: { vertical: 0, horizontal: 0, look: 0, run: false, motion: 'idle' },
  blocked: null,
  testing: false
}

// Module-scope singletons, shared by the Leash page and its setup guide.
const settings = ref<LeashSettings | null>(null)
const state = ref<LeashState>(EMPTY_STATE)

const SAVE_DELAY_MS = 250
let saveTimer: ReturnType<typeof setTimeout> | undefined
let saving = 0

// Rust announces every save, ours included. While one of ours is pending, an older copy coming
// back would undo what was just changed, so it's ignored.
api.onLeashSettings((value) => {
  if (!saveTimer && !saving) settings.value = value
})
api.onLeashState((value) => (state.value = value))

const { isOn } = useAddons()

watch(
  () => isOn('leash'),
  (on) => {
    if (!on) return
    void api.leashSettings().then((value) => (settings.value = value)).catch(() => {})
    void api.leashState().then((value) => (state.value = value)).catch(() => {})
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
      .leashSave(_.cloneDeep(toRaw(settings.value)))
      .catch((error) => console.error('Could not save the leash settings:', error))
      .finally(() => saving--)
  }, SAVE_DELAY_MS)
}

export function useLeash(): {
  settings: Ref<LeashSettings | null>
  state: Ref<LeashState>
  avatarName: ComputedRef<string>
  // The worn avatar's settings, or the defaults until it has its own.
  avatar: ComputedRef<AvatarLeash>
  change: (mutate: (settings: LeashSettings) => void) => void
  // Edits the worn avatar's own settings, made from the defaults when missing.
  changeAvatar: (mutate: (avatar: AvatarLeash) => void) => void
} {
  const avatarName = computed(() => state.value.avatarName ?? 'This avatar')
  const avatar = computed(() => {
    const id = state.value.avatarId

    return (id && settings.value?.avatars[id]) || DEFAULT_AVATAR_LEASH
  })

  const change = (mutate: (settings: LeashSettings) => void): void => {
    if (!settings.value) return

    mutate(settings.value)
    save()
  }

  const changeAvatar = (mutate: (avatar: AvatarLeash) => void): void => {
    const id = state.value.avatarId

    if (!id) return

    change((target) => {
      const own = (target.avatars[id] ??= _.cloneDeep(DEFAULT_AVATAR_LEASH))
      own.name = avatarName.value
      mutate(own)
    })
  }

  return { settings, state, avatarName, avatar, change, changeAvatar }
}
