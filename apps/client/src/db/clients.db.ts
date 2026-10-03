import type { Table } from 'dexie'
import Dexie from 'dexie'
import _ from 'lodash'

// One row per person: a Discord account (whatever network they connect from), or a guest's
// session. See viewerKey in useClientsDb.ts.
export interface Client {
  id?: number
  // `discord:<id>`, `guest:<session id>`, or `legacy-ip:<ip>` for guests recorded before viewers
  // were keyed by session (their IP is all we have for them).
  key: string
  discordId: string | null
  // Guest session ids that belong to this viewer: a guest's own, plus, on a Discord row, the guest
  // sessions they had before logging in. Their commands are logged under these.
  peerIds: string[]
  // Every stable IP they've connected from, most recent first; `ip` is the most recent.
  ips: string[]
  ip: string
  avatar: string
  displayName: string
  createdAt: number
  // When they were last connected to the share page - see useClientsDb.ts. Not indexed, so no
  // schema bump; viewers recorded before it existed don't have one until they next connect.
  lastSeenAt?: number
}

// A ban targets either a stable IP or a Discord ID, never a pair of the two (banning "by IP"
// shouldn't spare that same IP under a different Discord account, and vice versa) - so bans are
// keyed on [scope+value] rather than on a viewer's key.
export type BanScope = 'ip' | 'discord'

export interface BannedClient {
  id?: number
  scope: BanScope
  value: string
  reason?: string
  createdAt: number
}

type LegacyClient = Omit<Client, 'key' | 'peerIds' | 'ips'> & { uniqueKey?: string }

// Folds rows that are the same person into one: the earliest first-seen, the latest name, avatar
// and last-seen, and every IP and session id either of them had.
export const mergeClients = (into: Client, from: Client): Client => {
  const newer = (from.lastSeenAt ?? from.createdAt) > (into.lastSeenAt ?? into.createdAt)
  const latest = newer ? from : into
  const lastSeen = Math.max(into.lastSeenAt ?? 0, from.lastSeenAt ?? 0)

  return {
    ...into,
    discordId: into.discordId ?? from.discordId,
    displayName: latest.displayName,
    avatar: latest.avatar,
    ip: latest.ip,
    ips: [...new Set(newer ? [...from.ips, ...into.ips] : [...into.ips, ...from.ips])],
    peerIds: [...new Set([...into.peerIds, ...from.peerIds])],
    createdAt: Math.min(into.createdAt, from.createdAt),
    ...(lastSeen ? { lastSeenAt: lastSeen } : {})
  }
}

// v5: rows used to be keyed by IP + Discord id, which split a Discord user into one row per network
// and lumped every guest on one network into a single row.
export const migrateLegacyClients = (rows: LegacyClient[]): { keep: Client[]; remove: number[] } => {
  const merged = new Map<string, Client>()
  const remove: number[] = []

  rows.forEach((legacy) => {
    const row = _.omit(legacy, 'uniqueKey')
    const client: Client = {
      ...row,
      key: row.discordId ? `discord:${row.discordId}` : `legacy-ip:${row.ip}`,
      peerIds: [],
      ips: [row.ip]
    }
    const existing = merged.get(client.key)

    if (!existing) {
      merged.set(client.key, client)
      return
    }

    // Keep the older row's id, so links to it still work.
    const [keepRow, dropRow] = (existing.id ?? 0) <= (client.id ?? 0) ? [existing, client] : [client, existing]

    if (dropRow.id !== undefined) remove.push(dropRow.id)
    merged.set(client.key, { ...mergeClients(keepRow, dropRow), id: keepRow.id })
  })

  return { keep: [...merged.values()], remove }
}

export class ClientsDB extends Dexie {
  clients!: Table<Client, number>
  bannedClients!: Table<BannedClient, number>

  constructor() {
    super('ClientsDB')

    this.version(3).stores({
      clients: `
        ++id,
        ip,
        discordId,
        avatar,
        displayName,
        createdAt,
        &uniqueKey
      `
    })

    // v4: adds bannedClients for the ban feature.
    this.version(4).stores({
      bannedClients: `
        ++id,
        scope,
        value,
        createdAt,
        &[scope+value]
      `
    })

    // v5: one row per Discord account or guest session (see migrateLegacyClients).
    this.version(5)
      .stores({
        clients: `
          ++id,
          ip,
          discordId,
          createdAt,
          &key,
          *ips,
          *peerIds
        `
      })
      .upgrade(async (tx) => {
        const table = tx.table<LegacyClient, number>('clients')
        const { keep, remove } = migrateLegacyClients(await table.toArray())

        await table.bulkDelete(remove)
        await table.bulkPut(keep)
      })
  }
}

export const db = new ClientsDB()
