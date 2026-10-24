<script setup lang="ts">
import { useLiveQuery } from '@renderer/composables/useLiveQuery'
import { hostStatus, roomStats } from '@renderer/composables/useWebsocketHost'
import type { Client } from '@renderer/db/clients.db'
import { db as clientsDb } from '@renderer/db/clients.db'
import type { Command } from '@renderer/db/commands.db'
import { db } from '@renderer/db/commands.db'
import { viewerLookup } from '@renderer/utils/viewers'
import { useNow } from '@vueuse/core'
import { computed } from 'vue'

// This stream at a glance: how long you've been live, the most viewers at once (from the server),
// and who did the most and what got used most (from this device's log since the stream started).
const TOP_COUNT = 5

const now = useNow({ interval: 60_000 })

const live = computed(() => hostStatus.value === 'OPEN' && !!roomStats.value)

const commands = useLiveQuery(
  () => (live.value ? roomStats.value!.startedAt : null),
  (startedAt) => (startedAt === null ? Promise.resolve([]) : db.commands.where('createdAt').aboveOrEqual(startedAt).toArray()),
  [] as Command[]
)

const clients = useLiveQuery(
  () => null,
  () => clientsDb.clients.toArray(),
  [] as Client[]
)

const duration = computed(() => {
  const minutes = Math.max(0, Math.floor((now.value.getTime() - (roomStats.value?.startedAt ?? now.value.getTime())) / 60_000))

  return minutes < 60 ? `${minutes} min` : `${Math.floor(minutes / 60)} h ${minutes % 60} min`
})

const top = <T,>(items: T[], key: (item: T) => string | undefined): { label: string; count: number }[] => {
  const counts = new Map<string, number>()

  for (const item of items) {
    const label = key(item)

    if (label) counts.set(label, (counts.get(label) ?? 0) + 1)
  }

  return [...counts].map(([label, count]) => ({ label, count })).sort((a, b) => b.count - a.count).slice(0, TOP_COUNT)
}

const topViewers = computed(() => {
  const clientFor = viewerLookup(clients.value)

  return top(commands.value, (command) => clientFor(command)?.displayName)
})
const topControls = computed(() => top(commands.value, (command) => command.controlName))
</script>

<template>
  <section
    v-if="live"
    class="grid gap-3 rounded-field border border-primary/30 bg-primary/8 p-4"
  >
    <div class="flex flex-wrap items-baseline gap-x-6 gap-y-1">
      <h2 class="font-semibold text-highlighted">
        This stream
      </h2>
      <span class="text-sm text-muted">Live for <b class="text-default">{{ duration }}</b></span>
      <span class="text-sm text-muted">Most viewers at once <b class="text-default tabular-nums">{{ roomStats!.peakViewers }}</b></span>
      <span class="text-sm text-muted">Actions <b class="text-default tabular-nums">{{ commands.length }}</b></span>
    </div>
    <div class="grid gap-4 sm:grid-cols-2">
      <div
        v-for="list in [{ title: 'Top viewers', rows: topViewers }, { title: 'Most used', rows: topControls }]"
        :key="list.title"
        class="grid content-start gap-1.5"
      >
        <h3 class="font-mono text-[10.5px] tracking-widest text-muted uppercase">
          {{ list.title }}
        </h3>
        <p
          v-if="!list.rows.length"
          class="text-sm text-muted"
        >
          Nothing yet.
        </p>
        <div
          v-for="row in list.rows"
          :key="row.label"
          class="flex items-center gap-2 text-sm"
        >
          <span class="min-w-0 flex-1 truncate">{{ row.label }}</span>
          <span class="font-mono text-xs text-muted tabular-nums">{{ row.count }}</span>
        </div>
      </div>
    </div>
  </section>
</template>
