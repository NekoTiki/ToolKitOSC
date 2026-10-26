<script setup lang="ts">
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useAuth } from '@renderer/composables/useAuth'
import { serverWsUrl } from '@renderer/composables/useWebsocketSettings'
import { api } from '@renderer/lib/tauri-bridge'
import type { ControlTypes } from '@toolkitosc/shared-ui'
import { CONTROL_TYPE_LABELS } from '@toolkitosc/shared-ui'
import { computed, ref, watch } from 'vue'

// What viewers used, counted by the server: per day and by control type. Unlike the log, it covers
// every device you streamed from, and isn't cleared with the local activity.
interface Stats {
  activationByType: { type: string; count: number }[]
  activationDaily: { day: string; count: number }[]
  totalActivations: number
}

const DAY_RANGES = [
  { value: '7', label: '7 days' },
  { value: '30', label: '30 days' },
  { value: '90', label: '90 days' }
]

const { loggedIn } = useAuth()
const days = ref('30')
const stats = ref<Stats | null>(null)
const error = ref<string | null>(null)

watch(
  [days, loggedIn],
  async () => {
    if (!loggedIn.value) return

    try {
      const response = await api.serverRequest(serverWsUrl.value, 'GET', `/api/me/stats?days=${days.value}`)

      error.value = response.status < 400 ? null : `The server didn't send stats (${response.status}).`
      if (response.status < 400) stats.value = response.body as Stats
    } catch (err) {
      error.value = String(err)
    }
  },
  { immediate: true }
)

const byType = computed(() => [...(stats.value?.activationByType ?? [])].sort((a, b) => b.count - a.count))
const maxType = computed(() => Math.max(1, ...byType.value.map((row) => row.count)))

// Every day of the range, quiet days included, so the bars line up with the calendar.
const daily = computed(() => {
  const counts = new Map(stats.value?.activationDaily.map((row) => [row.day, row.count]))
  const length = Number(days.value)

  return Array.from({ length }, (_, i) => {
    const day = new Date(Date.now() - (length - 1 - i) * 86_400_000).toISOString().slice(0, 10)

    return { day, count: counts.get(day) ?? 0 }
  })
})
const maxDay = computed(() => Math.max(1, ...daily.value.map((row) => row.count)))
</script>

<template>
  <EmptyState
    v-if="!loggedIn"
    icon="i-lucide-chart-column"
    title="Sign in to see your stats"
    description="The server counts what viewers use on your share page. Sign in from Settings › Account."
  />
  <EmptyState
    v-else-if="error"
    icon="i-lucide-cloud-off"
    title="Stats aren't available"
    :description="error"
  />

  <div
    v-else
    class="grid content-start gap-4"
  >
    <div class="flex flex-wrap items-center gap-3">
      <p class="flex-1 text-sm text-muted">
        <b class="text-2xl font-semibold text-highlighted tabular-nums">{{ stats?.totalActivations ?? 0 }}</b>
        actions from viewers
      </p>
      <SegmentedControl
        v-model="days"
        :items="DAY_RANGES"
        aria-label="Date range"
      />
    </div>

    <section class="grid gap-2 rounded-field border border-default bg-(--aurora-glass) p-4">
      <h2 class="font-semibold text-highlighted">
        Per day
      </h2>
      <div
        class="flex h-36 items-end gap-0.5"
        role="img"
        :aria-label="`Actions per day over the last ${days} days`"
      >
        <span
          v-for="row in daily"
          :key="row.day"
          class="min-h-px flex-1 rounded-t bg-secondary"
          :class="{ 'opacity-25': !row.count }"
          :style="{ height: `${(row.count / maxDay) * 100}%` }"
          :title="`${row.day}: ${row.count}`"
        />
      </div>
    </section>

    <section class="grid gap-2 rounded-field border border-default bg-(--aurora-glass) p-4">
      <h2 class="font-semibold text-highlighted">
        By control type
      </h2>
      <p
        v-if="!byType.length"
        class="text-sm text-muted"
      >
        Nothing in this range.
      </p>
      <div
        v-for="row in byType"
        :key="row.type"
        class="grid grid-cols-[8rem_minmax(0,1fr)_3rem] items-center gap-2.5 text-sm"
      >
        <span class="truncate">{{ CONTROL_TYPE_LABELS[row.type as ControlTypes] ?? row.type }}</span>
        <span
          class="h-3.5 rounded-r bg-secondary"
          :style="{ width: `${(row.count / maxType) * 100}%` }"
        />
        <span class="text-right font-mono text-xs text-muted tabular-nums">{{ row.count }}</span>
      </div>
    </section>
  </div>
</template>
