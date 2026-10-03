import type { Client } from '@renderer/db/clients.db'
import { db as clientsDb } from '@renderer/db/clients.db'
import type { Command } from '@renderer/db/commands.db'
import { db } from '@renderer/db/commands.db'
import { viewerLookup } from '@renderer/utils/viewers'
import { from, useObservable } from '@vueuse/rxjs'
import type { Subscription } from 'dexie'
import { liveQuery } from 'dexie'
import type { ComputedRef, Ref } from 'vue'
import { computed, onScopeDispose, shallowRef, watch } from 'vue'

// Live "who used what" data for the Controls page, from the local command log: the last viewer to
// use each shown control (a small avatar badge on its tile) and the latest command overall (the
// log line under the tiles).
export interface ControlActivity {
  lastUsers: ComputedRef<Record<string, Client | undefined>>
  latest: ComputedRef<{ command: Command; client?: Client } | null>
}

export function useControlActivity(controlIds: Ref<string[]>): ControlActivity {
  const clients = useObservable(from(liveQuery(() => clientsDb.clients.toArray())), {
    initialValue: [] as Client[]
  })

  const latestCommand = useObservable(from(liveQuery(() => db.commands.orderBy('createdAt').last())), {
    initialValue: undefined
  })

  // One indexed lookup per shown control for its most recent command. liveQuery re-runs on table
  // writes but can't see the `controlIds` ref, so the subscription is rebuilt whenever the shown
  // controls change (another group, a search).
  const lastCommands = shallowRef<Command[]>([])
  let subscription: Subscription | undefined

  watch(
    controlIds,
    (ids) => {
      subscription?.unsubscribe()
      subscription = liveQuery(() =>
        Promise.all(
          ids.map((id) =>
            db.commands
              .where('[controlId+createdAt]')
              .between([id, 0], [id, Infinity])
              .reverse()
              .first()
          )
        ).then((commands) => commands.filter((c): c is Command => !!c))
      ).subscribe((commands) => (lastCommands.value = commands))
    },
    { immediate: true }
  )

  onScopeDispose(() => subscription?.unsubscribe())

  const findClient = computed(() => viewerLookup(clients.value))

  const lastUsers = computed(() => {
    const record: Record<string, Client | undefined> = {}

    lastCommands.value.forEach((command) => (record[command.controlId] = findClient.value(command)))

    return record
  })

  const latest = computed(() =>
    latestCommand.value ? { command: latestCommand.value, client: findClient.value(latestCommand.value) } : null
  )

  return { lastUsers, latest }
}
