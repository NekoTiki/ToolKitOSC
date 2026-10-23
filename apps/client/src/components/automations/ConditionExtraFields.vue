<script setup lang="ts">
import type { ParameterCondition } from '@renderer/lib/tauri-bridge'
import { MAX_HELD_MS, MAX_TAP_WINDOW_MS, MAX_TAPS } from '@renderer/utils/automations'
import { seconds } from '@renderer/utils/automationSentence'

// The values of "is between", "taps" and "stops changing", which aren't a single number.
const condition = defineModel<ParameterCondition>({ required: true })

// USlider can hand back an array; only plain numbers are taken (see OpenShockFields.vue).
const fromSlider = (value: number | number[] | undefined, apply: (value: number) => void): void => {
  if (typeof value === 'number') apply(value)
}
const clamp = (value: number | string, min: number, max: number): number => Math.min(max, Math.max(min, Math.round(Number(value) || min)))
</script>

<template>
  <div
    v-if="condition.type === 'is-between'"
    class="grid gap-3 sm:grid-cols-2"
  >
    <UFormField :label="`From ${Math.round(condition.min * 100)}%`">
      <USlider
        :model-value="condition.min"
        :min="0"
        :max="1"
        :step="0.01"
        @update:model-value="fromSlider($event, (value) => condition.type === 'is-between' && (condition.min = value))"
      />
    </UFormField>
    <UFormField
      :label="`To ${Math.round(condition.max * 100)}%`"
      :error="condition.min >= condition.max ? 'Has to be above the start.' : false"
    >
      <USlider
        :model-value="condition.max"
        :min="0"
        :max="1"
        :step="0.01"
        @update:model-value="fromSlider($event, (value) => condition.type === 'is-between' && (condition.max = value))"
      />
    </UFormField>
  </div>

  <div
    v-else-if="condition.type === 'taps'"
    class="flex flex-wrap items-center gap-2 text-sm text-muted"
  >
    <UInput
      :model-value="condition.count"
      type="number"
      min="2"
      :max="MAX_TAPS"
      class="w-20"
      aria-label="How many taps"
      @update:model-value="condition.type === 'taps' && (condition.count = clamp($event, 2, MAX_TAPS))"
    />
    times within
    <UInput
      :model-value="condition.withinMs"
      type="number"
      min="100"
      :max="MAX_TAP_WINDOW_MS"
      step="100"
      class="w-28"
      aria-label="Within milliseconds"
      @update:model-value="condition.type === 'taps' && (condition.withinMs = clamp($event, 100, MAX_TAP_WINDOW_MS))"
    />
    ms ({{ seconds(condition.withinMs) }})
  </div>

  <div
    v-else-if="condition.type === 'stops-changing'"
    class="flex flex-wrap items-center gap-2 text-sm text-muted"
  >
    For
    <UInput
      :model-value="condition.ms"
      type="number"
      min="100"
      :max="MAX_HELD_MS"
      step="100"
      class="w-28"
      aria-label="Quiet milliseconds"
      @update:model-value="condition.type === 'stops-changing' && (condition.ms = clamp($event, 100, MAX_HELD_MS))"
    />
    ms ({{ seconds(condition.ms) }}) after its last change
  </div>
</template>
