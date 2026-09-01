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

export class ClientsDB extends Dexie {
  clients!: Table<Client, number>

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
  }
}

export const db = new ClientsDB()
