<script setup lang="ts">
import type { AutomationStep } from '@renderer/lib/tauri-bridge'
import { STEP_KINDS, stepBusyMs, stepStarts } from '@renderer/utils/automations'
import { seconds } from '@renderer/utils/automationSentence'
import { computed } from 'vue'

// When each step starts and how long it lasts, on one time axis: a bar for something that runs
// for a while, an open bar for holds and follows (they last until the trigger ends), a dot for
// something that happens at once.
const props = defineProps<{ steps: AutomationStep[] }>()

const timeline = computed(() => {
  const starts = stepStarts(props.steps)
  const end = Math.max(2000, ...props.steps.map((step, i) => starts[i]! + (stepBusyMs(step) ?? 1500)))
  const total = end * 1.15
  const at = (ms: number): number => (ms / total) * 100
  const tick = total > 20_000 ? 5000 : total > 6000 ? 1000 : 500
  return {
    rows: props.steps.map((step, index) => {
      const ms = stepBusyMs(step)
      const start = starts[index]!
      return { label: `${index + 1} · ${STEP_KINDS[step.kind].label}`, left: at(start), width: ms === null ? 100 - at(start) : at(ms), open: ms === null, dot: ms === 0 }
    }),
    ticks: Array.from({ length: Math.floor(total / tick) + 1 }, (_, i) => ({ left: at(i * tick), label: seconds(i * tick) }))
  }
})
</script>

<template>
  <div
    class="overflow-x-auto rounded-field bg-(--aurora-well) px-3.5 pt-3 pb-1"
    aria-label="When each step runs"
  >
    <div class="relative min-w-md">
      <div
        v-for="(row, index) in timeline.rows"
        :key="index"
        class="relative h-8"
      >
        <template v-if="row.dot">
          <span
            class="absolute top-2.5 size-3 -translate-x-1/2 rounded-full bg-secondary"
            :style="{ left: `${row.left}%` }"
          />
          <span
            class="absolute top-1.5 ml-3 font-mono text-xs whitespace-nowrap text-muted"
            :style="{ left: `${row.left}%` }"
          >{{ row.label }}</span>
        </template>
        <span
          v-else
          class="absolute top-1 flex h-6 min-w-2.5 items-center overflow-hidden rounded-md border border-secondary/60 px-2 font-mono text-xs whitespace-nowrap text-highlighted"
          :class="row.open ? 'border-r-0' : 'bg-secondary/30'"
          :style="{
            left: `${row.left}%`,
            width: `${row.width}%`,
            background: row.open ? 'repeating-linear-gradient(135deg, color-mix(in oklab, var(--ui-secondary) 30%, transparent) 0 8px, color-mix(in oklab, var(--ui-secondary) 14%, transparent) 8px 16px)' : undefined
          }"
        >{{ row.label }}{{ row.open ? ' · until it ends' : '' }}</span>
      </div>
      <div class="relative mt-1 h-5 border-t border-default">
        <span
          v-for="tick in timeline.ticks"
          :key="tick.left"
          class="absolute top-1 -translate-x-1/2 font-mono text-[10.5px] text-dimmed tabular-nums first:translate-x-0"
          :style="{ left: `${tick.left}%` }"
        >{{ tick.label }}</span>
      </div>
    </div>
  </div>
</template>
