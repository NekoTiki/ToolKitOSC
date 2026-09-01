import type { Table } from 'dexie'
import Dexie from 'dexie'

export interface Command {
  id?: number
  peerId: string
  discordId?: string
  ip: string
  controlId: string
  createdAt: number
}

export class CommandsDB extends Dexie {
  commands!: Table<Command, number>

  constructor() {
    super('CommandDB')

    this.version(2).stores({
      commands: `
        ++id,
        peerId,
        discordId,
        ip,
        controlId,
        createdAt,
        [controlId+createdAt]
      `
    })
  }
}

export const db = new CommandsDB()
