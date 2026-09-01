import type { Table } from 'dexie'
import Dexie from 'dexie'

export interface Client {
  id?: number
  ip: string
  discordId: string | null
  avatar: string
  displayName: string
  createdAt: number
  uniqueKey: string
}

// A ban targets either a stable IP or a Discord ID, never a pair of the two (banning "by IP"
// shouldn't spare that same IP under a different Discord account, and vice versa) - so bans are
// keyed on [scope+value] rather than reusing the ip+discordId uniqueKey used for known clients.
export type BanScope = 'ip' | 'discord'

export interface BannedClient {
  id?: number
  scope: BanScope
  value: string
  reason?: string
  createdAt: number
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
  }
}

export const db = new ClientsDB()
