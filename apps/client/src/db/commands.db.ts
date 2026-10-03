import type { OpenShockCommandResult } from '@toolkitosc/shared-ui'
import type { Table } from 'dexie'
import Dexie from 'dexie'

export interface Command {
  id?: number
  peerId: string
  discordId?: string
  ip: string
  groupId: string
  controlId: string
  controlName: string
  type: string
  value?: boolean | number | string | OpenShockCommandResult
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

    // v3: adds groupId/controlName/type/value (plain, unindexed - existing rows are simply
    // missing them) and compound indexes so a client's log can be queried in recency order
    // by ip or discordId, the same way [controlId+createdAt] already supports per-control logs.
    this.version(3).stores({
      commands: `
        ++id,
        peerId,
        discordId,
        ip,
        controlId,
        createdAt,
        [controlId+createdAt],
        [ip+createdAt],
        [discordId+createdAt]
      `
    })

    // v4: a guest's log is looked up by their session id now (see utils/viewers.ts).
    this.version(4).stores({
      commands: `
        ++id,
        peerId,
        discordId,
        ip,
        controlId,
        createdAt,
        [controlId+createdAt],
        [ip+createdAt],
        [discordId+createdAt],
        [peerId+createdAt]
      `
    })
  }
}

export const db = new CommandsDB()
