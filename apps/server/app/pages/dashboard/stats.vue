<script setup lang="ts">
import {
  ArcElement,
  BarElement,
  CategoryScale,
  Chart as ChartJS,
  Legend,
  LinearScale,
  Tooltip
} from 'chart.js'
import { Bar, Doughnut } from 'vue-chartjs'

import type { Attempt } from '~/types/aiAttempts'

ChartJS.register(CategoryScale, LinearScale, BarElement, ArcElement, Tooltip, Legend)

interface StatsResult {
  daily: { day: string; success: number; failure: number }[]
  byProvider: { provider: string | null; success: number; failure: number; avgDurationMs: number | null }[]
  byProfile: { profile: string | null; count: number }[]
  totalSuccess: number
  totalFailure: number
}

interface ControlStatsResult {
  inventory: { type: string; count: number }[]
  totalControls: number
  activationByType: { type: string; count: number }[]
  activationDaily: { day: string; count: number }[]
  totalActivations: number
}

const { redirectIfUnauthorized } = useAdminApi()

// useFetch (not adminFetch/onMounted) so these actually run during SSR - the middleware only
// checks "logged in" (see middleware/admin.ts), the real admin check is each request's own
// response, and awaiting these here means the initial HTML an admin gets already has real charts/
// rows in it instead of a guaranteed-empty shell that fills in after hydration. `query` takes refs
// directly - useFetch already re-fetches on its own whenever one of them changes, so there's no
// need for a manual watch(...) triggering the same reload by hand.
const days = ref(14)
const {
  data: stats,
  pending: statsLoading,
  error: statsError,
  refresh: refreshStats
} = await useFetch<StatsResult>('/api/admin/stats', { query: { days } })

if (statsError.value) await redirectIfUnauthorized(statsError.value)

// Same `days` range as the AI stats above - one date-range selector drives both sections.
const {
  data: controlStats,
  pending: controlStatsLoading,
  error: controlStatsError
} = await useFetch<ControlStatsResult>('/api/admin/control-stats', { query: { days } })

if (controlStatsError.value) await redirectIfUnauthorized(controlStatsError.value)

const { colorFor: controlTypeColor, labelFor: controlTypeLabel } = useControlTypeChartColors()
// Applied to every dataset below (and every other chart on this page) - see the composable's own
// comment for why bars get this too, not just doughnuts.
const chartBorder = useChartBorder()

const controlInventoryChartData = computed(() => ({
  labels: controlStats.value?.inventory.map((i) => controlTypeLabel(i.type)) ?? [],
  datasets: [
    {
      label: 'Controls configured',
      data: controlStats.value?.inventory.map((i) => i.count) ?? [],
      backgroundColor: controlStats.value?.inventory.map((i) => controlTypeColor(i.type)) ?? [],
      ...chartBorder
    }
  ]
}))

const controlActivationChartData = computed(() => ({
  labels: controlStats.value?.activationByType.map((a) => controlTypeLabel(a.type)) ?? [],
  datasets: [
    {
      label: 'Activations',
      data: controlStats.value?.activationByType.map((a) => a.count) ?? [],
      backgroundColor: controlStats.value?.activationByType.map((a) => controlTypeColor(a.type)) ?? [],
      ...chartBorder
    }
  ]
}))

// RecentGenerationsCard (see the template below) owns fetching/live-updating the attempts list
// itself - all this page adds on top is keeping its own chart aggregates (below) in sync with the
// exact same live events, via the card's 'generation-logged' emit.
function updateStatsFromAttempt(attempt: Attempt): void {
  if (!stats.value) return

  if (attempt.success) stats.value.totalSuccess++
  else stats.value.totalFailure++

  // attempt.createdAt is an ISO string (UTC) - its first 10 chars are exactly the 'YYYY-MM-DD' UTC
  // day queryStats() groups by server-side (date(created_at, 'unixepoch')).
  const day = attempt.createdAt.slice(0, 10)
  let dayEntry = stats.value.daily.find((d) => d.day === day)

  if (!dayEntry) {
    dayEntry = { day, success: 0, failure: 0 }
    stats.value.daily.push(dayEntry)
    stats.value.daily.sort((a, b) => a.day.localeCompare(b.day))
  }

  if (attempt.success) dayEntry.success++
  else dayEntry.failure++

  let providerEntry = stats.value.byProvider.find((p) => p.provider === attempt.provider)

  if (!providerEntry) {
    // avgDurationMs is deliberately left stale here (not recomputed) - an accurate running average
    // would need to know how many prior rows actually had a duration (not every failure does, see
    // queryStats' comment), which this incremental path doesn't track. It self-corrects on the
    // next reconnect-triggered reload (see the template's @reconnected="refreshStats").
    providerEntry = { provider: attempt.provider, success: 0, failure: 0, avgDurationMs: null }
    stats.value.byProvider.push(providerEntry)
  }

  if (attempt.success) providerEntry.success++
  else providerEntry.failure++

  let profileEntry = stats.value.byProfile.find((p) => p.profile === attempt.profile)

  if (!profileEntry) {
    profileEntry = { profile: attempt.profile, count: 0 }
    stats.value.byProfile.push(profileEntry)
  }

  profileEntry.count++
}

// tailwindColor() itself is an auto-imported composable (see app/composables/useTailwindColor.ts) -
// shared with the per-user stats page and this page's own Controls section below.
const CHART_COLORS = {
  success: tailwindColor('green-500'),
  failure: tailwindColor('red-500'),
  providerPalette: ['indigo-500', 'green-500', 'orange-500', 'cyan-500', 'rose-500', 'purple-500'].map(tailwindColor)
}

const dailyChartData = computed(() => ({
  labels: stats.value?.daily.map((d) => d.day) ?? [],
  datasets: [
    { label: 'Success', data: stats.value?.daily.map((d) => d.success) ?? [], backgroundColor: CHART_COLORS.success, ...chartBorder },
    { label: 'Failure', data: stats.value?.daily.map((d) => d.failure) ?? [], backgroundColor: CHART_COLORS.failure, ...chartBorder }
  ]
}))

const providerChartData = computed(() => ({
  labels: stats.value?.byProvider.map((p) => p.provider ?? 'unknown') ?? [],
  datasets: [
    {
      label: 'Generations',
      data: stats.value?.byProvider.map((p) => p.success + p.failure) ?? [],
      backgroundColor: CHART_COLORS.providerPalette,
      ...chartBorder
    }
  ]
}))

// Toggle on the "By provider" card (see template) between total volume (the doughnut above),
// success rate, and average generation time per provider - all three read from the same
// stats.byProvider rows already fetched/kept live for the volume view, no separate query.
const providerViewMode = ref<'volume' | 'rate' | 'duration'>('volume')
const providerViewItems = [
  { label: 'Volume', value: 'volume' as const },
  { label: 'Success rate', value: 'rate' as const },
  { label: 'Avg duration', value: 'duration' as const }
]

function successRate(row: { success: number; failure: number }): number {
  const total = row.success + row.failure

  return total ? Math.round((row.success / total) * 100) : 0
}

// A stacked success%/failure% pair per provider, always summing to 100 - fills the whole bar
// (90/10, 50/50, 100/0, ...) instead of a single segment whose height alone has to carry the
// meaning, matching the same success/failure split language as the daily chart above.
const providerRateChartData = computed(() => {
  const rows = stats.value?.byProvider ?? []
  const rates = rows.map(successRate)

  return {
    labels: rows.map((p) => p.provider ?? 'unknown'),
    datasets: [
      { label: 'Success %', data: rates, backgroundColor: CHART_COLORS.success, ...chartBorder },
      { label: 'Failure %', data: rates.map((rate) => 100 - rate), backgroundColor: CHART_COLORS.failure, ...chartBorder }
    ]
  }
})

// Same per-provider color identity as the volume doughnut, so a given provider reads as the same
// color across all three views instead of switching palettes when the toggle changes.
const providerDurationChartData = computed(() => {
  const rows = stats.value?.byProvider ?? []

  return {
    labels: rows.map((p) => p.provider ?? 'unknown'),
    datasets: [
      {
        label: 'Avg duration (s)',
        data: rows.map((p) => (p.avgDurationMs === null ? 0 : Math.round(p.avgDurationMs) / 1000)),
        backgroundColor: CHART_COLORS.providerPalette,
        ...chartBorder
      }
    ]
  }
})

const chartOptions = { responsive: true, maintainAspectRatio: false }
// Both custom bar views below hide the legend - each only ever shows one semantic per bar (a
// success/failure split, or a single duration value), so a legend just repeats the axis label with
// no extra information, unlike the doughnut's per-slice legend or the daily chart's two genuinely
// distinct series.
const rateChartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: { legend: { display: false } },
  scales: {
    x: { stacked: true },
    y: { stacked: true, min: 0, max: 100, ticks: { callback: (value: string | number) => `${value}%` } }
  }
}
const durationChartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: { legend: { display: false } },
  scales: { y: { min: 0, ticks: { callback: (value: string | number) => `${value}s` } } }
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="flex items-center justify-between gap-2">
      <p class="text-sm font-medium">
        Last
      </p>
      <USelect
        v-model="days"
        :items="[
          { label: '7 days', value: 7 },
          { label: '14 days', value: 14 },
          { label: '30 days', value: 30 },
          { label: '90 days', value: 90 }
        ]"
        class="w-32"
      />
    </div>

    <div
      v-if="stats"
      class="flex gap-4"
    >
      <UCard class="flex-1">
        <p class="text-xs text-muted">
          Successful generations
        </p>
        <p class="text-2xl font-semibold text-success">
          {{ stats.totalSuccess }}
        </p>
      </UCard>
      <UCard class="flex-1">
        <p class="text-xs text-muted">
          Failed generations
        </p>
        <p class="text-2xl font-semibold text-error">
          {{ stats.totalFailure }}
        </p>
      </UCard>
    </div>

    <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
      <UCard>
        <!-- Matches the "By provider" card's title+toggle row height below (an empty div holding
        the same place a UTabs would), so both charts sit at the same vertical offset and both
        cards end up the same overall height instead of this one looking shorter. -->
        <div class="mb-2 flex h-8 items-center justify-between gap-2">
          <p class="text-sm font-medium">
            Generations per day
          </p>
        </div>
        <ClientOnly>
          <div class="h-64">
            <Bar
              v-if="!statsLoading"
              :data="dailyChartData"
              :options="chartOptions"
            />
          </div>
        </ClientOnly>
      </UCard>

      <UCard>
        <div class="mb-2 flex h-8 items-center justify-between gap-2">
          <p class="text-sm font-medium">
            By provider
          </p>
          <UTabs
            v-model="providerViewMode"
            :items="providerViewItems"
            size="xs"
            class="w-72"
          />
        </div>
        <ClientOnly>
          <div class="h-64">
            <Doughnut
              v-if="!statsLoading && providerViewMode === 'volume'"
              :data="providerChartData"
              :options="chartOptions"
            />
            <Bar
              v-if="!statsLoading && providerViewMode === 'rate'"
              :data="providerRateChartData"
              :options="rateChartOptions"
            />
            <Bar
              v-if="!statsLoading && providerViewMode === 'duration'"
              :data="providerDurationChartData"
              :options="durationChartOptions"
            />
          </div>
        </ClientOnly>
      </UCard>
    </div>

    <div
      v-if="controlStats"
      class="flex gap-4"
    >
      <UCard class="flex-1">
        <p class="text-xs text-muted">
          Controls configured (all hosts)
        </p>
        <p class="text-2xl font-semibold">
          {{ controlStats.totalControls }}
        </p>
      </UCard>
      <UCard class="flex-1">
        <p class="text-xs text-muted">
          Control activations
        </p>
        <p class="text-2xl font-semibold">
          {{ controlStats.totalActivations }}
        </p>
      </UCard>
    </div>

    <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
      <UCard>
        <p class="mb-2 text-sm font-medium">
          Controls configured, by type
        </p>
        <ClientOnly>
          <div class="h-64">
            <Doughnut
              v-if="!controlStatsLoading"
              :data="controlInventoryChartData"
              :options="chartOptions"
            />
          </div>
        </ClientOnly>
      </UCard>

      <UCard>
        <p class="mb-2 text-sm font-medium">
          Activations, by type
        </p>
        <ClientOnly>
          <div class="h-64">
            <Doughnut
              v-if="!controlStatsLoading"
              :data="controlActivationChartData"
              :options="chartOptions"
            />
          </div>
        </ClientOnly>
      </UCard>
    </div>

    <RecentGenerationsCard
      @generation-logged="updateStatsFromAttempt"
      @reconnected="refreshStats"
    />
  </div>
</template>
