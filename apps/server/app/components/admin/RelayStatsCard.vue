<script setup lang="ts">
// How busy share pages were over the range: the most hosts and viewers online at once per day,
// viewers joining, and messages rate limited.
interface RelayDay {
  day: string
  peakHosts: number
  peakViewers: number
  joins: number
  rateLimited: number
}

const props = defineProps<{ days: number }>()

const { redirectIfUnauthorized } = useAdminApi()

const { data, error } = await useFetch<{ days: RelayDay[] }>('/api/admin/relay-stats', { query: { days: toRef(props, 'days') } })

if (error.value) await redirectIfUnauthorized(error.value)

const rows = computed(() => data.value?.days ?? [])
const maxViewers = computed(() => Math.max(1, ...rows.value.map((row) => row.peakViewers)))

const totals = computed(() => [
  { label: 'Most hosts at once', value: Math.max(0, ...rows.value.map((row) => row.peakHosts)) },
  { label: 'Most viewers at once', value: Math.max(0, ...rows.value.map((row) => row.peakViewers)) },
  { label: 'Viewers joined', value: rows.value.reduce((sum, row) => sum + row.joins, 0) },
  { label: 'Messages rate limited', value: rows.value.reduce((sum, row) => sum + row.rateLimited, 0) }
])
</script>

<template>
  <UCard :ui="{ body: 'grid gap-4' }">
    <h2 class="text-base font-semibold text-highlighted">
      Share pages
    </h2>
    <dl class="grid grid-cols-2 gap-3 md:grid-cols-4">
      <div
        v-for="total in totals"
        :key="total.label"
        class="grid gap-0.5"
      >
        <dt class="text-xs text-muted">
          {{ total.label }}
        </dt>
        <dd class="text-2xl font-semibold tabular-nums">
          {{ total.value.toLocaleString('en-US') }}
        </dd>
      </div>
    </dl>
    <div class="grid gap-1.5">
      <span class="text-xs text-muted">Most viewers at once, per day</span>
      <div
        v-if="rows.length"
        class="flex h-24 items-end gap-0.5"
        role="img"
        aria-label="Most viewers at once, per day"
      >
        <span
          v-for="row in rows"
          :key="row.day"
          class="min-h-px flex-1 rounded-t bg-secondary"
          :style="{ height: `${(row.peakViewers / maxViewers) * 100}%` }"
          :title="`${row.day}: ${row.peakViewers} viewers, ${row.peakHosts} hosts, ${row.joins} joins`"
        />
      </div>
      <p
        v-else
        class="py-4 text-center text-sm text-muted"
      >
        Nothing in this range.
      </p>
    </div>
  </UCard>
</template>
