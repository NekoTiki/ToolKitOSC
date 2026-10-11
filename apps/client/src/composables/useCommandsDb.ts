import type { Command } from '@renderer/db/commands.db'
import { db } from '@renderer/db/commands.db'
import { useLocalStorage } from '@vueuse/core'
import _ from 'lodash'

type NewCommand = Omit<Command, 'id' | 'createdAt'>

const DAY_MS = 24 * 60 * 60 * 1000

// How long the activity log keeps commands, in days - chosen in Settings → Data & logs.
export const LOG_RETENTION_CHOICES = [1, 7, 30] as const
export const logRetentionDays = useLocalStorage<number>('logRetentionDays', 7)

// Not tied to any component's lifecycle - see useLogRetention.ts, which is what actually schedules
// this (once at app startup, then on a recurring interval so it also catches logs that cross the
// retention window mid-session, not just at the moment some particular component happens to
// mount).
export const purgeOldCommands = (): Promise<number> =>
  db.commands
    .where('createdAt')
    .below(Date.now() - logRetentionDays.value * DAY_MS)
    .delete()
    .catch((error) => {
      console.error('Failed to purge old command logs:', error)
      return 0
    })

// Continuous controls (a slider being dragged) can fire dozens of commands a second. Logging
// every single one would flood a client's history, so writes for these types are debounced per
// client+control - trailing only, so the log ends up with the settled value rather than noise,
// with a maxWait so a long, continuous drag still leaves a few entries instead of going silent.
const DEBOUNCED_LOG_TYPES = new Set(['slider'])
const LOG_DEBOUNCE_WAIT = 500
const LOG_DEBOUNCE_MAX_WAIT = 2000

const debouncedWriters = new Map<string, _.DebouncedFunc<(command: NewCommand) => void>>()

export function useCommandsDb(): {
  // Resolves to the new row's id, or undefined when the write was debounced or failed.
  add: (command: NewCommand) => Promise<number | undefined>
  update: (id: number, changes: Partial<Pick<Command, 'endedAt'>>) => void
} {
  const write = (command: NewCommand): Promise<number | undefined> =>
    // Fire-and-forget, but don't let a failed write (e.g. a value Dexie's structured-clone can't
    // store) vanish silently - it used to just drop the log entry with no trace.
    db.commands.add({ ...command, createdAt: Date.now() }).catch((error) => {
      console.error('Failed to write command log:', error)

      return undefined
    })

  const update = (id: number, changes: Partial<Pick<Command, 'endedAt'>>): void => {
    db.commands.update(id, changes).catch((error) => console.error('Failed to update command log:', error))
  }

  const add = async (command: NewCommand): Promise<number | undefined> => {
    if (!DEBOUNCED_LOG_TYPES.has(command.type)) return write(command)

    const key = `${command.peerId}:${command.controlId}`

    if (!debouncedWriters.has(key)) {
      debouncedWriters.set(
        key,
        _.debounce((next: NewCommand) => void write(next), LOG_DEBOUNCE_WAIT, {
          leading: false,
          trailing: true,
          maxWait: LOG_DEBOUNCE_MAX_WAIT
        })
      )
    }

    debouncedWriters.get(key)!(command)

    return undefined
  }

  return { add, update }
}
