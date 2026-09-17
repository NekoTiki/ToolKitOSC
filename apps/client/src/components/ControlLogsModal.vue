<script setup lang="ts">
import { formatUniqueKey, useClientsDb } from '@renderer/composables/useClientsDb'
import type { Client } from '@renderer/db/clients.db'
import type { Command } from '@renderer/db/commands.db'
import { db } from '@renderer/db/commands.db'
import { formatCommandValue } from '@renderer/utils/commandLog'
import type { ControlTypes, UiColor } from '@vrc-osc-toolkit/shared-ui'
import { CONTROL_TYPE_COLORS, CONTROL_TYPE_LABELS } from '@vrc-osc-toolkit/shared-ui'
import { from, useObservable } from '@vueuse/rxjs'
import type { Subscription } from 'dexie'
import { liveQuery } from 'dexie'
import { computed, onScopeDispose, ref, watch } from 'vue'

const props = defineProps<{ controlId: string; controlName: string }>()
const open = defineModel<boolean>('open')

const LOG_LIMIT = 200

const logs = ref<Command[]>([])

// `controlId` is fixed for this component's whole lifetime in practice (useOverlay mounts a fresh
// instance per open, see useControlLogsModal.ts), but this still watches it rather than assuming
// that - a plain immediate watch handles both cases identically (mirrors ClientLogsModal.vue).
let subscription: Subscription | undefined

watch(
  () => props.controlId,
  (current) => {
    subscription?.unsubscribe()

    const observable = liveQuery<Command[]>(() =>
      db.commands
        .where('[controlId+createdAt]')
        .between([current, 0], [current, Infinity])
        .reverse()
        .limit(LOG_LIMIT)
        .toArray()
    )

    subscription = observable.subscribe({
      next: (result) => (logs.value = result),
      error: (error) => console.error('Failed to load control logs:', error)
    })
  },
  { immediate: true }
)

onScopeDispose(() => subscription?.unsubscribe())

const { getClients } = useClientsDb()
const clientsObservable = from(getClients())
const clients = useObservable(clientsObservable, { initialValue: [] as Client[] })

const clientsByKey = computed<Map<string, Client>>(() => {
  const map = new Map<string, Client>()

  clients.value.forEach((client) => map.set(client.uniqueKey, client))

  return map
})

// Who fired this specific command - resolved per row (not just the most recent, unlike
// ControlGroup.vue's own commandsLastUser) since this shows the whole history, not just one
// avatar. A row can resolve to nothing if that client was never written to the clients table (or
// has since been forgotten) - shown as a generic placeholder rather than hiding the row.
const clientFor = (log: Command): Client | undefined =>
  clientsByKey.value.get(formatUniqueKey(log.ip, log.discordId))

const typeLabel = (type: string): string => CONTROL_TYPE_LABELS[type as ControlTypes] ?? type

const typeColor = (type: string): UiColor => CONTROL_TYPE_COLORS[type as ControlTypes] ?? 'neutral'
</script>

<template>
  <USlideover
    v-model:open="open"
    inset
    side="right"
    class="w-full max-w-lg"
    :title="`Logs for ${controlName}`"
    :ui="{ body: 'overflow-hidden' }"
  >
    <template #body>
      <div
        v-if="!logs.length"
        class="text-center text-muted"
      >
        No recorded activity for this control yet.
      </div>

      <!-- virtualize: up to LOG_LIMIT (200) rows. Row spacing comes from the item slot's own
      padding (`pb-2 last:pb-0`), not a viewport `gap` - virtualized rows are absolutely
      positioned, which a flex gap has no effect on. `overflow-hidden` on the slideover's own body
      (above) cancels its default `overflow-y-auto` so this is the only scroll region, not a
      redundant nested one. -->
      <UScrollArea
        v-else
        :items="logs"
        virtualize
        class="h-full"
        :ui="{ item: 'pb-2 last:pb-0' }"
      >
        <template #default="{ item: log }">
          <UCard :ui="{ body: 'flex items-center justify-between gap-2 p-2 sm:p-2' }">
            <div class="flex min-w-0 items-center gap-2">
              <UAvatar
                :src="clientFor(log)?.avatar"
                size="xs"
              />
              <div class="flex min-w-0 flex-col">
                <span class="truncate font-medium">{{ clientFor(log)?.displayName ?? 'Unknown' }}</span>
                <span class="text-xs text-muted">
                  {{ new Date(log.createdAt).toLocaleString() }}
                </span>
              </div>
            </div>

            <div class="flex shrink-0 items-center gap-2">
              <UBadge
                :color="typeColor(log.type)"
                variant="subtle"
              >
                {{ typeLabel(log.type) }}
              </UBadge>
              <span class="text-sm font-medium">{{ formatCommandValue(log.value) }}</span>
            </div>
          </UCard>
        </template>
      </UScrollArea>
    </template>
  </USlideover>
</template>

<style scoped></style>
