import { controlGroups as exampleGroups } from '@renderer/assets/controlsExample'
import { db as clientsDb } from '@renderer/db/clients.db'
import type { LegacySetup, SetupSnapshot, ViewersSetup } from '@renderer/lib/tauri-bridge'
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

// Viewer groups, who is in which and bans, as Rust keeps them (src-tauri/src/setup/viewers.rs).
export const viewersSetup = ref<ViewersSetup>({ groups: [], memberships: {}, bans: [] })

api.onSetupViewers((viewers) => (viewersSetup.value = viewers))

// Before Rust: groups in localStorage `viewerGroups`, memberships on each viewer's row and bans in
// ClientsDB. Handed over once; the old copies stay for now.
const VIEWERS_MIGRATED_KEY = 'viewersMigrated'

const legacyViewers = async (): Promise<ViewersSetup> => {
  let groups: ViewersSetup['groups'] = []

  try {
    const saved: unknown = JSON.parse(localStorage.getItem('viewerGroups') ?? '[]')

    groups = Array.isArray(saved) ? saved : []
  } catch {
    // Unreadable: nothing to keep.
  }

  const members = await clientsDb.clients.filter((client) => !!client.groups?.length).toArray()
  const bans = await clientsDb.bannedClients.toArray()

  return {
    groups,
    memberships: Object.fromEntries(members.map((client) => [client.key, client.groups!])),
    bans: bans.map(({ scope, value, reason, createdAt }) => ({ scope, value, reason, createdAt }))
  }
}

const fetchViewers = (): void => void api.setupViewers().then((viewers) => (viewersSetup.value = viewers))

if (localStorage.getItem(VIEWERS_MIGRATED_KEY) === null) {
  void legacyViewers()
    .then((legacy) => api.setupMigrateViewers(legacy))
    .then(() => localStorage.setItem(VIEWERS_MIGRATED_KEY, '1'))
    .catch((error) => console.error('Could not move the viewer groups and bans to the app:', error))
    .finally(fetchViewers)
} else {
  fetchViewers()
}
