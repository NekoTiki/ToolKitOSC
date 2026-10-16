import type { Client, GroupMembership } from '@renderer/db/clients.db'
import { db } from '@renderer/db/clients.db'
import { getUUID } from '@renderer/utils/uuid'
import type { ViewerGroupInfo } from '@toolkitosc/shared-ui'
import { liveQuery } from 'dexie'
import { computed, ref, shallowRef } from 'vue'

// Viewer groups (Friends, Mods...): the host puts viewers in them by hand, and each profile can give a
// group more than everyone else gets (see groupOverrides in useLockedControls.ts). The groups
// themselves are global, not per avatar, since they're about people. Who is in which group is saved
// on each viewer's row in ClientsDB.
export type ViewerGroup = ViewerGroupInfo

const STORAGE_KEY = 'viewerGroups'

// Group colors to pick from, picked in turn for new groups.
export const GROUP_COLORS = ['#f472b6', '#a78bfa', '#60a5fa', '#22d3ee', '#34d399', '#a3e635', '#fbbf24', '#fb923c', '#f87171']

// How long a membership lasts, offered wherever a viewer is added to a group. null: until removed.
export const MEMBERSHIP_DURATIONS: { label: string; ms: number | null }[] = [
  { label: 'Until I remove them', ms: null },
  { label: 'For 10 minutes', ms: 10 * 60_000 },
  { label: 'For 1 hour', ms: 60 * 60_000 },
  { label: 'For 24 hours', ms: 24 * 60 * 60_000 }
]

const load = (): ViewerGroup[] => {
  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '[]')

    return Array.isArray(saved) ? saved : []
  } catch {
    return []
  }
}

const viewerGroups = ref<ViewerGroup[]>(load())

const save = (): void => localStorage.setItem(STORAGE_KEY, JSON.stringify(viewerGroups.value))

// Ticks when a temporary membership ends, so everything that depends on who's in which group
// (the share page included) updates on time.
const now = ref(Date.now())
let expiryTimer: ReturnType<typeof setTimeout> | undefined

// Every viewer row that's in at least one group, by viewer key. Kept in sync at module scope so the
// command check in useWebsocketHost.ts can read it synchronously, like bans.
const memberships = shallowRef<Map<string, GroupMembership[]>>(new Map())

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

liveQuery(() => db.clients.filter((client) => !!client.groups?.length).toArray()).subscribe({
  next: (rows) => {
    memberships.value = new Map(rows.map((row) => [row.key, row.groups!]))
    now.value = Date.now()
    scheduleExpiry()
  },
  error: (error) => console.error('Failed to load viewer groups', error)
})

const isActive = (membership: GroupMembership): boolean => !membership.until || membership.until > now.value

const groupExists = (id: string): boolean => viewerGroups.value.some((group) => group.id === id)

// The groups a viewer is in right now, by viewer key (see viewerKey in useClientsDb.ts).
const activeGroupIds = (key: string): string[] =>
  (memberships.value.get(key) ?? []).filter((membership) => isActive(membership) && groupExists(membership.id)).map((membership) => membership.id)

// Everyone in at least one group right now: viewer key -> group ids. Rust checks viewer commands
// against it (see useWebsocketHost.ts).
const activeMemberships = computed<Record<string, string[]>>(() =>
  Object.fromEntries([...memberships.value.keys()].map((key) => [key, activeGroupIds(key)] as const).filter(([, ids]) => ids.length))
)

// A viewer's current memberships, for showing them: expired ones and deleted groups left out.
const membershipsOf = (client: Pick<Client, 'groups'>): (GroupMembership & { group: ViewerGroup })[] =>
  (client.groups ?? []).flatMap((membership) => {
    const group = viewerGroups.value.find((g) => g.id === membership.id)

    return group && isActive(membership) ? [{ ...membership, group }] : []
  })

const getGroup = (id: string): ViewerGroup | undefined => viewerGroups.value.find((group) => group.id === id)

const addGroup = (name: string): ViewerGroup => {
  const group = { id: getUUID(), name, color: GROUP_COLORS[viewerGroups.value.length % GROUP_COLORS.length]! }

  viewerGroups.value = [...viewerGroups.value, group]
  save()

  return group
}

const updateGroup = (id: string, changes: Partial<Omit<ViewerGroup, 'id'>>): void => {
  viewerGroups.value = viewerGroups.value.map((group) => (group.id === id ? { ...group, ...changes } : group))
  save()
}

// Takes everyone out of the group too. Profiles keep their rules for it, but nothing reads them.
const deleteGroup = async (id: string): Promise<void> => {
  viewerGroups.value = viewerGroups.value.filter((group) => group.id !== id)
  save()

  await db.clients
    .filter((client) => !!client.groups?.some((membership) => membership.id === id))
    .modify((client) => {
      client.groups = client.groups!.filter((membership) => membership.id !== id)
    })
}

// Expired memberships are dropped whenever a viewer's groups change.
const setMemberships = async (client: Client, change: (groups: GroupMembership[]) => GroupMembership[]): Promise<void> => {
  if (client.id === undefined) return

  await db.clients
    .where('id')
    .equals(client.id)
    .modify((row) => {
      row.groups = change((row.groups ?? []).filter((membership) => !membership.until || membership.until > Date.now()))
    })
}

const addToGroup = (client: Client, groupId: string, durationMs: number | null): Promise<void> =>
  setMemberships(client, (groups) => [
    ...groups.filter((membership) => membership.id !== groupId),
    durationMs ? { id: groupId, until: Date.now() + durationMs } : { id: groupId }
  ])

const removeFromGroup = (client: Client, groupId: string): Promise<void> =>
  setMemberships(client, (groups) => groups.filter((membership) => membership.id !== groupId))

// How many viewers are in each group right now.
const memberCount = (groupId: string): number =>
  [...memberships.value.values()].filter((list) => list.some((membership) => membership.id === groupId && isActive(membership))).length

export function useViewerGroups(): {
  viewerGroups: typeof viewerGroups
  now: typeof now
  getGroup: typeof getGroup
  addGroup: typeof addGroup
  updateGroup: typeof updateGroup
  deleteGroup: typeof deleteGroup
  activeGroupIds: typeof activeGroupIds
  activeMemberships: typeof activeMemberships
  membershipsOf: typeof membershipsOf
  addToGroup: typeof addToGroup
  removeFromGroup: typeof removeFromGroup
  memberCount: typeof memberCount
} {
  return {
    viewerGroups,
    now,
    getGroup,
    addGroup,
    updateGroup,
    deleteGroup,
    activeGroupIds,
    activeMemberships,
    membershipsOf,
    addToGroup,
    removeFromGroup,
    memberCount
  }
}
