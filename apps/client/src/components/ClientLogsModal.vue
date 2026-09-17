<script setup lang="ts">
import type { LogClient } from '@renderer/composables/useClientLogsModal'
import type { Command } from '@renderer/db/commands.db'
import { db } from '@renderer/db/commands.db'
import { formatCommandValue, isOpenShockCommandValue } from '@renderer/utils/commandLog'
import type { ControlTypes, UiColor } from '@vrc-osc-toolkit/shared-ui'
import { CONTROL_TYPE_COLORS, CONTROL_TYPE_LABELS } from '@vrc-osc-toolkit/shared-ui'
import type { Subscription } from 'dexie'
import { liveQuery } from 'dexie'
import { onScopeDispose, ref, watch } from 'vue'

const props = defineProps<{ client: LogClient }>()
const open = defineModel<boolean>('open')

const LOG_LIMIT = 200

const logs = ref<Command[]>([])

// liveQuery only re-runs its callback when the Dexie tables it touched last time change - it has
// no idea `props.client` is reactive, so a single long-lived subscription created at setup time
// would never re-run on its own. `client` is fixed for this component's whole lifetime in
// practice (useOverlay mounts a fresh instance per open, see useClientLogsModal.ts), but this still
// watches it rather than assuming that - a plain immediate watch handles both cases identically.
let subscription: Subscription | undefined

watch(
  () => props.client,
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

// The control name only says which button was pressed, not the actually interesting part of an
// OpenShock command - which physical shocker(s) it fired. `shockers` is already the display names,
// resolved and written at command time (see useControls.ts's handleCommand) rather than resolved
// here from ids on every read, so this only has to fall back when none were known at write time
// (e.g. the shocker name cache hadn't warmed up yet) or the control didn't resolve at all.
const title = (log: Command): string => {
  if (isOpenShockCommandValue(log.value) && log.value.shockers.length) return log.value.shockers.join(', ')

  return log.controlName || log.controlId
}

const typeLabel = (type: string): string => CONTROL_TYPE_LABELS[type as ControlTypes] ?? type

const typeColor = (type: string): UiColor => CONTROL_TYPE_COLORS[type as ControlTypes] ?? 'neutral'
</script>

<template>
  <!-- Every Modal/Slideover in this app defaults to the same z-30 (set globally in
  vite.config.ts), which is normally fine since they don't overlap - but this one is opened from
  ClientDetails.vue's "See Logs" action while ClientsListSliderover stays open behind it (unlike
  e.g. the locked-group editor, which closes its own list first). Same reasoning and tier as
  AreYouSureModal.vue's override: two equal z-30 overlays would fall back to DOM/mount order, so
  this is deliberately bumped above that shared tier. -->
  <USlideover
    v-model:open="open"
    inset
    side="right"
    class="z-50 w-full max-w-lg"
    :ui="{ body: 'overflow-hidden' }"
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
              <span class="text-sm font-medium">{{ formatCommandValue(log.value) }}</span>
            </div>
          </UCard>
        </template>
      </UScrollArea>
    </template>
  </USlideover>
</template>

<style scoped></style>
