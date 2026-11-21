<script setup lang="ts">
import OptionGrid from '@renderer/components/ui/OptionGrid.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import type { VoiceFxAction } from '@renderer/lib/tauri-bridge'
import type { VoiceFxEffect } from '@toolkitosc/shared-ui'
import { VOICE_FX_EFFECTS } from '@toolkitosc/shared-ui'
import { computed } from 'vue'

// The fields of the Voice effects step: hold an effect on your voice, or let go of it.
// Automations share one hold, so one automation can turn off what another turned on.
const step = defineModel<VoiceFxAction>({ required: true })

const RUNS = [
  { value: 'on' as const, label: 'Turn on' },
  { value: 'off' as const, label: 'Turn off' },
  { value: 'toggle' as const, label: 'Toggle' }
]

const DEFAULT = 'default'
const EFFECTS = [{ value: DEFAULT, label: 'Default effect', icon: 'i-lucide-star' }, ...VOICE_FX_EFFECTS.map((effect) => ({ value: effect.id as string, label: effect.label, icon: effect.icon }))]
const effect = computed({
  get: () => step.value.effect ?? DEFAULT,
  set: (value: string) => (step.value.effect = value === DEFAULT ? null : (value as VoiceFxEffect))
})

const LENGTHS = [
  { value: '0', label: 'Until turned off' },
  { value: '10', label: '10 s' },
  { value: '30', label: '30 s' },
  { value: '60', label: '1 min' },
  { value: '300', label: '5 min' }
]
const length = computed({
  // A length an assistant wrote that isn't in the list shows as the nearest longer one.
  get: () => LENGTHS.find((item) => Number(item.value) >= step.value.seconds && step.value.seconds > 0)?.value ?? (step.value.seconds ? '300' : '0'),
  set: (value: string) => (step.value.seconds = Number(value))
})

// USlider can hand back an array; only plain numbers are taken (see OpenShockFields.vue).
const fromSlider = (value: number | number[] | undefined, apply: (value: number) => void): void => {
  if (typeof value === 'number') apply(value)
}
</script>

<template>
  <div class="grid gap-4">
    <UFormField
      label="Voice effect"
      description="Automations share one hold: Turn off ends what any of them turned on. Other sources keep theirs."
    >
      <SegmentedControl
        v-model="step.run"
        class="w-fit"
        accent
        :items="RUNS"
        aria-label="Voice effect"
      />
    </UFormField>
    <template v-if="step.run !== 'off'">
      <UFormField label="Effect">
        <OptionGrid
          v-model="effect"
          min-width="9rem"
          :items="EFFECTS"
          aria-label="Effect"
        />
      </UFormField>
      <UFormField :label="`Strength: ${Math.round(step.level * 100)}%`">
        <USlider
          :model-value="step.level"
          :min="0.05"
          :max="1"
          :step="0.05"
          @update:model-value="fromSlider($event, (value) => (step.level = value))"
        />
      </UFormField>
      <UFormField label="How long">
        <SegmentedControl
          v-model="length"
          class="w-fit max-w-full flex-wrap"
          :items="LENGTHS"
          aria-label="How long"
        />
      </UFormField>
    </template>
  </div>
</template>
