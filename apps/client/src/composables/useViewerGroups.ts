import { useAddons } from '@renderer/composables/useAddons'
import { viewersSetup } from '@renderer/composables/useSetup'
import type { GroupMembership } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import { getUUID } from '@renderer/utils/uuid'
import type { ViewerGroupInfo } from '@toolkitosc/shared-ui'
import { computed, ref, watch } from 'vue'

// Viewer groups (Friends, Mods...): the host puts viewers in them by hand, and each profile can give a
// group more than everyone else gets (see groupOverrides in useLockedControls.ts). The groups
// themselves are global, not per avatar, since they're about people. Rust keeps them, and who is in
// which by viewer key (src-tauri/src/setup/viewers.rs).
export type ViewerGroup = ViewerGroupInfo

// Group colors to pick from, picked in turn for new groups.
export const GROUP_COLORS = ['#f472b6', '#a78bfa', '#60a5fa', '#22d3ee', '#34d399', '#a3e635', '#fbbf24', '#fb923c', '#f87171']

// How long a membership lasts, offered wherever a viewer is added to a group. null: until removed.
export const MEMBERSHIP_DURATIONS: { label: string; ms: number | null }[] = [
  { label: 'Until I remove them', ms: null },
  { label: 'For 10 minutes', ms: 10 * 60_000 },
  { label: 'For 1 hour', ms: 60 * 60_000 },
  { label: 'For 24 hours', ms: 24 * 60 * 60_000 }
]

// Built in, while the AI assistants add-on is on (src-tauri/src/mcp): every assistant is in it, so
// a profile can give assistants less, or more, than everyone. Nobody else can be put in it, and it
// can't be renamed or deleted, so it's left out of the Viewer groups page.
export const ASSISTANTS_GROUP: ViewerGroup = { id: 'assistants', name: 'Assistants', color: '#a78bfa' }

// The groups you made.
const savedGroups = computed<ViewerGroup[]>(() => viewersSetup.value.groups)
const viewerGroups = computed<ViewerGroup[]>(() => (useAddons().isOn('mcp') ? [...savedGroups.value, ASSISTANTS_GROUP] : savedGroups.value))

// Who is in which group, by viewer key, expired memberships included.
const memberships = computed(() => new Map(Object.entries(viewersSetup.value.memberships)))

// Ticks when a temporary membership ends, so what's shown updates on time. Rust tells the share
// page and the headset by itself.
const now = ref(Date.now())
let expiryTimer: ReturnType<typeof setTimeout> | undefined

const scheduleExpiry = (): void => {
  clearTimeout(expiryTimer)

  const next = Math.min(
    ...[...memberships.value.values()].flat().flatMap((membership) => (membership.until && membership.until > Date.now() ? [membership.until] : []))
  )

  if (Number.isFinite(next)) {
    // setTimeout can't wait longer than ~24.8 days; it just checks again then.
    expiryTimer = setTimeout(() => {
      now.value = Date.now()
      scheduleExpiry()
    }, Math.min(next - Date.now() + 50, 2 ** 31 - 1))
  }
}

watch(
  memberships,
  () => {
    now.value = Date.now()
    scheduleExpiry()
  },
  { immediate: true }
)

const isActive = (membership: GroupMembership): boolean => !membership.until || membership.until > now.value

const groupExists = (id: string): boolean => viewerGroups.value.some((group) => group.id === id)

// The groups a viewer is in right now, by viewer key (see viewerKey in useClientsDb.ts).
const activeGroupIds = (key: string): string[] =>
  (memberships.value.get(key) ?? []).filter((membership) => isActive(membership) && groupExists(membership.id)).map((membership) => membership.id)

// A viewer's current memberships, for showing them: expired ones and deleted groups left out.
const membershipsOf = (client: { key: string }): (GroupMembership & { group: ViewerGroup })[] =>
  (memberships.value.get(client.key) ?? []).flatMap((membership) => {
    const group = viewerGroups.value.find((g) => g.id === membership.id)

    return group && isActive(membership) ? [{ ...membership, group }] : []
  })

const getGroup = (id: string): ViewerGroup | undefined => viewerGroups.value.find((group) => group.id === id)

// Shown straight away; Rust saves them and sends them back.
const saveGroups = (groups: ViewerGroup[]): void => {
  viewersSetup.value = { ...viewersSetup.value, groups }
  api.setupSaveViewerGroups(JSON.parse(JSON.stringify(groups))).catch((error) => console.error('Could not save the viewer groups:', error))
}

const addGroup = (name: string): ViewerGroup => {
  const group = { id: getUUID(), name, color: GROUP_COLORS[savedGroups.value.length % GROUP_COLORS.length]! }

  saveGroups([...savedGroups.value, group])

  return group
}

const updateGroup = (id: string, changes: Partial<Omit<ViewerGroup, 'id'>>): void => {
  saveGroups(savedGroups.value.map((group) => (group.id === id ? { ...group, ...changes } : group)))
}

// Rust takes everyone out of the group too. Profiles keep their rules for it, but nothing reads them.
const deleteGroup = async (id: string): Promise<void> => {
  saveGroups(savedGroups.value.filter((group) => group.id !== id))
}

const addToGroup = (client: { key: string }, groupId: string, durationMs: number | null): Promise<void> =>
  api.setupAddToGroup(client.key, groupId, durationMs)

const removeFromGroup = (client: { key: string }, groupId: string): Promise<void> => api.setupRemoveFromGroup(client.key, groupId)

// How many viewers are in each group right now.
const memberCount = (groupId: string): number =>
  [...memberships.value.values()].filter((list) => list.some((membership) => membership.id === groupId && isActive(membership))).length

export function useViewerGroups(): {
  viewerGroups: typeof viewerGroups
  // Only the groups you made, without Assistants.
  savedGroups: typeof savedGroups
  memberships: typeof memberships
  now: typeof now
  getGroup: typeof getGroup
  addGroup: typeof addGroup
  updateGroup: typeof updateGroup
  deleteGroup: typeof deleteGroup
  activeGroupIds: typeof activeGroupIds
  membershipsOf: typeof membershipsOf
  addToGroup: typeof addToGroup
  removeFromGroup: typeof removeFromGroup
  memberCount: typeof memberCount
} {
  return {
    viewerGroups,
    savedGroups,
    memberships,
    now,
    getGroup,
    addGroup,
    updateGroup,
    deleteGroup,
    activeGroupIds,
    membershipsOf,
    addToGroup,
    removeFromGroup,
    memberCount
  }
}
