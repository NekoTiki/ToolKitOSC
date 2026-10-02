<script setup lang="ts">
import type { ControlStatsResult, StatsResult } from '~/types/adminStats'

// KPI tiles plus the four usage charts, shared by the Overview (every host) and a user's page
// (just them): generations per day, AI providers, and controls by type.
const props = defineProps<{ ai: StatsResult | null | undefined; controls: ControlStatsResult | null | undefined; days: number; dailyTitle: string }>()

const { labelFor: controlTypeLabel } = useControlTypeChartColors()

const num = (value: number): string => value.toLocaleString('en-US')

const kpis = computed(() => {
  const success = props.ai?.totalSuccess ?? 0
  const failure = props.ai?.totalFailure ?? 0

  return [
    { icon: 'i-lucide-circle-check', label: 'Successful generations', value: success, note: `last ${props.days} days` },
    {
      icon: 'i-lucide-circle-x',
      label: 'Failed generations',
      value: failure,
      note: success + failure ? `${Math.round((failure / (success + failure)) * 100)}% failure rate` : 'no generations'
    },
    { icon: 'i-lucide-layout-grid', label: 'Controls configured', value: props.controls?.totalControls ?? 0, note: 'latest snapshot per avatar' },
    { icon: 'i-lucide-zap', label: 'Control activations', value: props.controls?.totalActivations ?? 0, note: `last ${props.days} days` }
  ]
})

// The provider card switches between volume, success rate and average time - all three read from
// the same byProvider rows.
const providerView = ref<'volume' | 'rate' | 'speed'>('volume')
const providerViews = [
  { value: 'volume', label: 'Volume' },
  { value: 'rate', label: 'Success rate' },
  { value: 'speed', label: 'Avg time' }
] as const

const providerBars = computed(() => {
  const rows = props.ai?.byProvider ?? []
  const label = (row: StatsResult['byProvider'][number]): string => row.providerLabel ?? row.provider ?? 'Unknown'

  if (providerView.value === 'rate') {
    return rows
      .filter((row) => row.success + row.failure)
      .map((row) => ({ label: label(row), value: Math.round((row.success / (row.success + row.failure)) * 100) }))
      .sort((a, b) => b.value - a.value)
  }

  if (providerView.value === 'speed') {
    return rows
      .filter((row) => row.avgDurationMs !== null)
      .map((row) => ({ label: label(row), value: row.avgDurationMs! / 1000 }))
      .sort((a, b) => a.value - b.value)
  }

  return rows.map((row) => ({ label: label(row), value: row.success + row.failure })).sort((a, b) => b.value - a.value)
})

const providerFormat = computed(() =>
  providerView.value === 'rate' ? (v: number): string => `${v}%` : providerView.value === 'speed' ? (v: number): string => `${v.toFixed(1)}s` : num
)

const providerHelp = computed(
  () =>
    ({
      volume: 'Generations in the selected range.',
      rate: 'Share of generations that succeeded.',
      speed: 'Successful generations only. Shorter is better.'
    })[providerView.value]
)

// Largest first, so the ranking reads top to bottom.
const typeBars = (rows: { type: string; count: number }[] | undefined): { label: string; value: number }[] =>
  [...(rows ?? [])].sort((a, b) => b.count - a.count).map((row) => ({ label: controlTypeLabel(row.type), value: row.count }))
</script>

<template>
  <div class="grid grid-cols-2 gap-3 sm:grid-cols-[repeat(auto-fill,minmax(190px,1fr))]">
    <div
      v-for="kpi in kpis"
      :key="kpi.label"
      class="grid gap-1 rounded-field glass p-4"
    >
      <small class="flex items-center gap-1.5 text-[12.5px] text-muted">
        <UIcon
          :name="kpi.icon"
          class="size-3.5"
        />{{ kpi.label }}
      </small>
      <b class="text-3xl leading-tight font-semibold text-highlighted tabular-nums">{{ num(kpi.value) }}</b>
      <span class="text-xs text-muted">{{ kpi.note }}</span>
    </div>
  </div>

  <div class="grid gap-3.5 lg:grid-cols-2">
    <AdminDailyGenerationsChart
      :title="dailyTitle"
      :daily="ai?.daily ?? []"
      :days="days"
    />

    <UCard :ui="{ body: 'grid content-start gap-3' }">
      <div class="flex flex-wrap items-center gap-2.5">
        <h2 class="flex-1 text-base font-semibold whitespace-nowrap text-highlighted">
          By provider
        </h2>
        <AdminSegmented
          v-model="providerView"
          :items="providerViews"
          label="Provider view"
          size="sm"
        />
      </div>
      <AdminRankedBars
        :rows="providerBars"
        :format="providerFormat"
        :max="providerView === 'rate' ? 100 : undefined"
      />
      <p class="text-[13px] text-muted">
        {{ providerHelp }}
      </p>
    </UCard>

    <UCard :ui="{ body: 'grid content-start gap-3' }">
      <h2 class="text-base font-semibold text-highlighted">
        Controls configured, by type
      </h2>
      <AdminRankedBars :rows="typeBars(controls?.inventory)" />
    </UCard>

    <UCard :ui="{ body: 'grid content-start gap-3' }">
      <h2 class="text-base font-semibold text-highlighted">
        Activations, by type
      </h2>
      <AdminRankedBars :rows="typeBars(controls?.activationByType)" />
    </UCard>
  </div>
</template>
