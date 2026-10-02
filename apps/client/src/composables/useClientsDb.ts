import type { Client } from '@renderer/db/clients.db'
import { db } from '@renderer/db/clients.db'
import type { PromiseExtended } from 'dexie'
import { ref, watch } from 'vue'

const onlineClients = ref<string[]>([])
export const formatUniqueKey = (ip: string, discordId?: string | null): string =>
  `${ip}-${discordId || null}`

// "Last seen": stamped on each viewer as they drop off the online list, and once a minute while
// they're on it - so it stays close even when the app is closed while they're still connected.
const stampLastSeen = (keys: string[]): void => {
  if (keys.length) void db.clients.where('uniqueKey').anyOf(keys).modify({ lastSeenAt: Date.now() })
}

watch(onlineClients, (online, before) => stampLastSeen(before.filter((key) => !online.includes(key))))
setInterval(() => stampLastSeen(onlineClients.value), 60_000)

export function useClientsDb(): {
  onlineClients: typeof onlineClients
  add: (command: Omit<Client, 'id' | 'createdAt' | 'uniqueKey'>) => void
  get: (ip: string, discordId?: string) => PromiseExtended<Client | undefined>
  getClients: () => PromiseExtended<Client[]>
} {
  const add = (client: Omit<Client, 'id' | 'createdAt' | 'uniqueKey'>): void => {
    db.clients
      .get({ uniqueKey: formatUniqueKey(client.ip, client.discordId) })
      .then((existingClient) => {
        if (!existingClient) {
          db.clients.add({
            ...client,
            createdAt: Date.now(),
            lastSeenAt: Date.now(),
            uniqueKey: formatUniqueKey(client.ip, client.discordId)
          })
        } else {
          const updatedClient = { ...existingClient, ...client, lastSeenAt: Date.now() } as Client

          db.clients.put(updatedClient)
        }
      })
  }

  const get = (ip: string, discordId?: string): PromiseExtended<Client | undefined> => {
    if (!discordId) return db.clients.get({ ip })
    return db.clients.get({ uniqueKey: formatUniqueKey(ip, discordId) })
  }

  const getClients = (): PromiseExtended<Client[]> => db.clients.toArray()

  return { onlineClients, add, get, getClients }
}
