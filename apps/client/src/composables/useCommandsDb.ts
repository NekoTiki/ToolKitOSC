import type { Command } from '@renderer/db/commands.db'
import { db } from '@renderer/db/commands.db'
import type { PromiseExtended } from 'dexie'
import { onMounted } from 'vue'

export function useCommandsDb(): {
  add: (command: Omit<Command, 'id' | 'createdAt'>) => PromiseExtended<number>
} {
  const add = (command: Omit<Command, 'id' | 'createdAt'>): PromiseExtended<number> =>
    db.commands.add({ ...command, createdAt: Date.now() })

  const purge = (): PromiseExtended<number> =>
    db.commands
      .where('createdAt')
      .below(Date.now() - 7 * 24 * 60 * 60 * 1000)
      .delete()

  onMounted(() => purge())

  return { add }
}
