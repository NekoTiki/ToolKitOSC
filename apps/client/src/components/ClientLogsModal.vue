<script setup lang="ts">
import { useClientLogsModal } from '@renderer/composables/useClientLogsModal'
import type { Command } from '@renderer/db/commands.db'
import { db } from '@renderer/db/commands.db'
import type { ControlTypes, OpenShockCommandResult, UiColor } from '@vrc-osc-toolkit/shared-ui'
import { CONTROL_TYPE_COLORS, CONTROL_TYPE_LABELS } from '@vrc-osc-toolkit/shared-ui'
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

const isOpenShockValue = (value: Command['value']): value is OpenShockCommandResult =>
  typeof value === 'object' && value !== null && 'shockers' in value

// The control name only says which button was pressed, not the actually interesting part of an
// OpenShock command - which physical shocker(s) it fired. `shockers` is already the display names,
// resolved and written at command time (see useControls.ts's handleCommand) rather than resolved
// here from ids on every read, so this only has to fall back when none were known at write time
// (e.g. the shocker name cache hadn't warmed up yet) or the control didn't resolve at all.
const title = (log: Command): string => {
  if (isOpenShockValue(log.value) && log.value.shockers.length) return log.value.shockers.join(', ')

  return log.controlName || log.controlId
}

const formatValue = (value: Command['value']): string => {
  if (isOpenShockValue(value)) return `${value.intensity}% • ${(value.duration / 1000).toFixed(1)}s`
  if (typeof value === 'boolean') return value ? 'On' : 'Off'
  if (typeof value === 'number') return Number.isInteger(value) ? String(value) : value.toFixed(2)
  if (value === undefined) return '-'

  return String(value)
}

const typeLabel = (type: string): string => CONTROL_TYPE_LABELS[type as ControlTypes] ?? type

const typeColor = (type: string): UiColor => CONTROL_TYPE_COLORS[type as ControlTypes] ?? 'neutral'
</script>

<template>
  <!-- z-50: see the identical comment in ClientsListSliderover.vue - AppHeader.vue's sticky bar
  has an explicit z-10 that otherwise wins over this slideover's implicit one regardless of DOM
  order. -->
  <USlideover
    v-model:open="open"
    inset
    side="right"
    class="z-50 w-full max-w-lg"
    :ui="{ overlay: 'z-50' }"
  >
    <template #title>
      <div class="flex items-center gap-1.5">
        <UAvatar :src="client?.avatar" size="xs" />
        {{ client ? `Logs for ${client.displayName}` : 'Client Logs' }}
      </div>
    </template>

    <template #body>
      <div v-if="!logs.length" class="text-center text-muted">
        No recent activity for this client.
      </div>

      <div v-else class="flex flex-col gap-2 overflow-y-auto">
        <UCard
          v-for="log in logs"
          :key="log.id"
          class="shrink-0"
          :ui="{ body: 'flex items-center justify-between gap-2 p-2 sm:p-2' }"
        >
          <div class="flex flex-col">
            <span class="font-medium">{{ title(log) }}</span>
            <span class="text-xs text-muted">
              {{ new Date(log.createdAt).toLocaleString() }}
            </span>
          </div>

          <div class="flex items-center gap-2">
            <UBadge :color="typeColor(log.type)" variant="subtle">
              {{ typeLabel(log.type) }}
            </UBadge>
            <span class="text-sm font-medium">{{ formatValue(log.value) }}</span>
          </div>
        </UCard>
      </div>
    </template>
  </USlideover>
</template>

<style scoped></style>
