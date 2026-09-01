import type { Command } from '@renderer/db/commands.db'
import { db } from '@renderer/db/commands.db'
import _ from 'lodash'
import { onMounted } from 'vue'

type NewCommand = Omit<Command, 'id' | 'createdAt'>

// Continuous controls (a slider being dragged) can fire dozens of commands a second. Logging
// every single one would flood a client's history, so writes for these types are debounced per
// client+control - trailing only, so the log ends up with the settled value rather than noise,
// with a maxWait so a long, continuous drag still leaves a few entries instead of going silent.
const DEBOUNCED_LOG_TYPES = new Set(['slider'])
const LOG_DEBOUNCE_WAIT = 500
const LOG_DEBOUNCE_MAX_WAIT = 2000

const debouncedWriters = new Map<string, _.DebouncedFunc<(command: NewCommand) => void>>()

export function useCommandsDb(): {
  add: (command: NewCommand) => void
} {
  const write = (command: NewCommand): void => {
    void db.commands.add({ ...command, createdAt: Date.now() })
  }

  const add = (command: NewCommand): void => {
    if (!DEBOUNCED_LOG_TYPES.has(command.type)) {
      write(command)
      return
    }

    const key = `${command.peerId}:${command.controlId}`

    if (!debouncedWriters.has(key)) {
      debouncedWriters.set(
        key,
        _.debounce(write, LOG_DEBOUNCE_WAIT, {
          leading: false,
          trailing: true,
          maxWait: LOG_DEBOUNCE_MAX_WAIT
        })
      )
    }

    debouncedWriters.get(key)!(command)
  }

  const purge = (): void => {
    void db.commands
      .where('createdAt')
      .below(Date.now() - 7 * 24 * 60 * 60 * 1000)
      .delete()
  }

  onMounted(() => purge())

  return { add }
}
