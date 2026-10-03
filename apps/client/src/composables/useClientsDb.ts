import type { Client } from '@renderer/db/clients.db'
import { db, mergeClients } from '@renderer/db/clients.db'
import { getStableIp } from '@renderer/utils/stableIp'
import type { ClientListEntry } from '@toolkitosc/shared-ui'
import { guestName } from '@toolkitosc/shared-ui'
import { shallowRef, watch } from 'vue'

// Who a viewer is: their Discord account when they're signed in, wherever they connect from;
// otherwise their guest session (the same id their "Adjective Animal" name comes from), so guests
// sharing a network stay apart.
export const viewerKey = (discordId: string | null | undefined, peerId: string): string =>
  discordId ? `discord:${discordId}` : `guest:${peerId}`

export const getGuestAvatar = (peerId: string, username: string = guestName(peerId)): string =>
  `https://ui-avatars.com/api/?name=${username}&background=random&size=256&${peerId}`

export interface Viewer {
  key: string
  discordId: string | null
  peerId: string
  // Their guest session from before they logged in with Discord, if the server knows it.
  guestId?: string
  ip: string
  displayName: string
  avatar: string
}

export const describeViewer = (client: ClientListEntry): Viewer => {
  const discordId = client.user?.discord?.id || null
  const guestDisplayName = client.user?.userDefinedDisplayName || guestName(client.peerId)

  return {
    key: viewerKey(discordId, client.peerId),
    discordId,
    peerId: client.peerId,
    guestId: client.guestId,
    ip: getStableIp(client.ip),
    displayName: client.user?.discord?.name || client.user?.username || guestDisplayName,
    avatar: client.user?.discord?.avatar || getGuestAvatar(client.peerId, guestDisplayName)
  }
}

// Keys of everyone connected right now.
const onlineClients = shallowRef<Set<string>>(new Set())

// "Last seen": stamped on each viewer as they drop off the online list, and once a minute while
// they're on it - so it stays close even when the app is closed while they're still connected.
const stampLastSeen = (keys: string[]): void => {
  if (keys.length) void db.clients.where('key').anyOf(keys).modify({ lastSeenAt: Date.now() })
}

watch(onlineClients, (online, before) => stampLastSeen([...before].filter((key) => !online.has(key))))
setInterval(() => stampLastSeen([...onlineClients.value]), 60_000)

const toRow = (viewer: Viewer, now: number): Client => ({
  key: viewer.key,
  discordId: viewer.discordId,
  peerIds: viewer.discordId ? [] : [viewer.peerId],
  ips: [viewer.ip],
  ip: viewer.ip,
  avatar: viewer.avatar,
  displayName: viewer.displayName,
  createdAt: now,
  lastSeenAt: now
})

// Records viewers in one transaction, so the same person in two tabs (or two client lists arriving
// back to back) can't both find "nobody yet" and add them twice.
const upsertViewers = (viewers: Viewer[]): Promise<void> =>
  db
    .transaction('rw', db.clients, async () => {
      const now = Date.now()

      for (const viewer of new Map(viewers.map((v) => [v.key, v])).values()) {
        let row = await db.clients.get({ key: viewer.key })

        // A guest who just logged in with Discord: their guest history becomes their account's.
        // The guest row keeps its id when there's no account row yet, so links to it still work.
        const guest = viewer.discordId && viewer.guestId ? await db.clients.get({ key: viewerKey(null, viewer.guestId) }) : undefined

        if (guest?.id !== undefined) {
          await db.clients.delete(guest.id)
          row = row ? mergeClients(row, guest) : { ...guest, key: viewer.key, discordId: viewer.discordId }
        }

        await db.clients.put(row ? mergeClients(row, toRow(viewer, now)) : toRow(viewer, now))
      }
    })
    .catch((error) => console.error('Failed to record viewers', error))

export function useClientsDb(): {
  onlineClients: typeof onlineClients
  isOnline: (client: Client) => boolean
  upsertViewers: typeof upsertViewers
} {
  const isOnline = (client: Client): boolean => onlineClients.value.has(client.key)

  return { onlineClients, isOnline, upsertViewers }
}
