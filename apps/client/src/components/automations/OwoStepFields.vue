<script setup lang="ts">
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import type { OwoSensationAction } from '@renderer/lib/tauri-bridge'
import type { OwoMuscle } from '@toolkitosc/shared-ui'
import { OWO_MUSCLES, OWO_SENSATIONS } from '@toolkitosc/shared-ui'

// The fields of the OWO step: a built-in sensation on some muscles.
const step = defineModel<OwoSensationAction>({ required: true })

const MUSCLES = Object.keys(OWO_MUSCLES) as OwoMuscle[]

const toggle = (muscle: OwoMuscle): void => {
  const muscles = step.value.muscles

  step.value.muscles = muscles.includes(muscle) ? muscles.filter((m) => m !== muscle) : MUSCLES.filter((m) => m === muscle || muscles.includes(m))
}

// USlider can hand back an array; only plain numbers are taken (see OpenShockFields.vue).
const fromSlider = (value: number | number[] | undefined, apply: (value: number) => void): void => {
  if (typeof value === 'number') apply(value)
}
</script>

<template>
  <div class="grid gap-4">
    <UFormField label="Sensation">
      <SegmentedControl
        v-model="step.sensation"
        class="w-fit"
        aria-label="Sensation"
        :items="OWO_SENSATIONS.map((s) => ({ value: s.id, label: s.label }))"
      />
    </UFormField>
    <UFormField
      label="Where"
      :error="step.muscles.length ? false : 'Pick where it plays.'"
    >
      <div class="flex flex-wrap gap-1.5">
        <button
          v-for="muscle in MUSCLES"
          :key="muscle"
          type="button"
          class="rounded-full border px-3 py-1.5 text-[13px] transition-colors"
          :class="step.muscles.includes(muscle) ? 'border-primary/50 bg-primary/14 text-highlighted' : 'border-transparent bg-(--aurora-well) text-muted hover:bg-(--aurora-glass)'"
          :aria-pressed="step.muscles.includes(muscle)"
          @click="toggle(muscle)"
        >
          {{ OWO_MUSCLES[muscle].label }}
        </button>
      </div>
    </UFormField>
    <UFormField
      :label="`Intensity: ${Math.round(step.intensity * 100)}%`"
      description="Under each muscle's strength in Settings > OWO and the limits you calibrated in the OWO app."
    >
      <USlider
        :model-value="step.intensity"
        :min="0.05"
        :max="1"
        :step="0.05"
        class="max-w-sm"
        @update:model-value="fromSlider($event, (value) => (step.intensity = value))"
      />
    </UFormField>
  </div>
</template>
