import { controlGroups as exampleGroups } from '@renderer/assets/controlsExample'
import type { LegacySetup, SetupSnapshot } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import { ref } from 'vue'

// The worn avatar's controls and profiles as Rust keeps them (src-tauri/src/setup/). useControls
// and useLockedControls show and edit this; every change comes back from Rust.
export const setup = ref<SetupSnapshot>({ avatarId: null, groups: [], profiles: [], defaultProfile: null, activeProfile: null })

api.onSetupControls((snapshot) => (setup.value = snapshot))

// Where the webview kept them before Rust took over: `controls_<avatarId>`,
// `lockedControlsGroups_<avatarId>` and `defaultLockedControlsGroup_<avatarId>`. Handed over once.
// The keys stay for now, so nothing is lost if the hand-over goes wrong.
const MIGRATED_KEY = 'setupMigrated'
const PREFIXES = ['controls_', 'lockedControlsGroups_', 'defaultLockedControlsGroup_']
// This avatar always showed the example controls instead of its saved ones.
const EXAMPLE_AVATAR = 'avtr_b89f691a-d31d-4a84-83fe-5cff212b3761'

const parse = (key: string): unknown[] => {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(key) ?? '[]')

    return Array.isArray(value) ? value : []
  } catch {
    return []
  }
}

const legacySetups = (): LegacySetup[] => {
  const ids = new Set<string>()

  for (let i = 0; i < localStorage.length; i++) {
    const key = localStorage.key(i) ?? ''
    const prefix = PREFIXES.find((p) => key.startsWith(p))
    const id = prefix && key.slice(prefix.length)

    if (id && (id.startsWith('avtr_') || id.startsWith('local:'))) ids.add(id)
  }

  return [...ids].map((avatarId) => ({
    avatarId,
    groups: (avatarId === EXAMPLE_AVATAR ? exampleGroups : parse(`controls_${avatarId}`)) as LegacySetup['groups'],
    profiles: parse(`lockedControlsGroups_${avatarId}`) as LegacySetup['profiles'],
    defaultProfile: localStorage.getItem(`defaultLockedControlsGroup_${avatarId}`)
  }))
}

const fetchSetup = (): void => void api.setupControls().then((snapshot) => (setup.value = snapshot))

if (localStorage.getItem(MIGRATED_KEY) === null) {
  void api
    .setupMigrate(legacySetups())
    .then(() => localStorage.setItem(MIGRATED_KEY, '1'))
    .catch((error) => console.error('Could not move the controls to the app:', error))
    .finally(fetchSetup)
} else {
  fetchSetup()
}
