<script setup lang="ts">
import { useClientLogsModal } from '@renderer/composables/useClientLogsModal'
import type { Command } from '@renderer/db/commands.db'
import { db } from '@renderer/db/commands.db'
import type { Subscription } from 'dexie'
import { liveQuery } from 'dexie'
import { onScopeDispose, ref, watch } from 'vue'

const { open, client } = useClientLogsModal()

const LOG_LIMIT = 200

const logs = ref<Command[]>([])

// liveQuery only re-runs its callback when the Dexie tables it touched last time change - it has
// no idea `client` is a Vue ref, so a single long-lived subscription created at setup time would
// never notice the selected client changing (and if it first ran while no client was selected,
// it never even touches `db.commands`, so it would never re-run again at all). Instead, tear down
// and recreate the subscription every time the selected client changes.
let subscription: Subscription | undefined

watch(
  client,
  (current) => {
    subscription?.unsubscribe()

    if (!current) {
      logs.value = []
      return
    }

    const observable = current.discordId
      ? liveQuery<Command[]>(() =>
          db.commands
            .where('[discordId+createdAt]')
            .between([current.discordId, 0], [current.discordId, Infinity])
            .reverse()
            .limit(LOG_LIMIT)
            .toArray()
        )
      : liveQuery<Command[]>(() =>
          db.commands
            .where('[ip+createdAt]')
            .between([current.ip, 0], [current.ip, Infinity])
            .reverse()
            .limit(LOG_LIMIT)
            .toArray()
        )

    subscription = observable.subscribe({
      next: (result) => (logs.value = result),
      error: (error) => console.error('Failed to load client logs:', error)
    })
  },
  { immediate: true }
)

onScopeDispose(() => subscription?.unsubscribe())

const formatValue = (value: Command['value']): string => {
  if (typeof value === 'boolean') return value ? 'On' : 'Off'
  if (typeof value === 'number') return Number.isInteger(value) ? String(value) : value.toFixed(2)
  if (value === undefined) return '-'

  return String(value)
}

const typeColor = (type: string): 'primary' | 'secondary' | 'neutral' => {
  if (type === 'slider') return 'primary'
  if (type === 'open-shock-shocker') return 'secondary'

  return 'neutral'
}
</script>

<template>
  <UModal
    v-model:open="open"
    :title="client ? `Logs for ${client.displayName}` : 'Client Logs'"
    class="z-50"
    :ui="{ overlay: 'z-50', body: 'max-w-lg' }"
  >
    <template #body>
      <div
        v-if="!logs.length"
        class="text-center text-muted"
      >
        No recent activity for this client.
      </div>

      <div
        v-else
        class="flex max-h-[60vh] flex-col gap-2 overflow-y-auto"
      >
        <UCard
          v-for="log in logs"
          :key="log.id"
          class="shrink-0"
          :ui="{ body: 'flex items-center justify-between gap-2 p-2 sm:p-2' }"
        >
          <div class="flex flex-col">
            <span class="font-medium">{{ log.controlName || log.controlId }}</span>
            <span class="text-xs text-muted">
              {{ new Date(log.createdAt).toLocaleString() }}
            </span>
          </div>

          <div class="flex items-center gap-2">
            <UBadge
              :color="typeColor(log.type)"
              variant="subtle"
            >
              {{ log.type }}
            </UBadge>
            <span class="text-sm font-medium">{{ formatValue(log.value) }}</span>
          </div>
        </UCard>
      </div>
    </template>
  </UModal>
</template>

<style scoped></style>
