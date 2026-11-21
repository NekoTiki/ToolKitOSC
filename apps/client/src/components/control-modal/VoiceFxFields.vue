<script setup lang="ts">
import CheckChip from '@renderer/components/ui/CheckChip.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import type { VoiceFxControl, VoiceFxEffect } from '@toolkitosc/shared-ui'
import { VOICE_FX_EFFECTS, VOICE_FX_MAX_SECONDS } from '@toolkitosc/shared-ui'
import { computed } from 'vue'

// The Voice effects control's options. Three kinds: a toggle and a timed button with their effect
// and strength set here, or all in one, where whoever uses it picks them on the tile.
export type VoiceFxOptions = Pick<VoiceFxControl, 'kind' | 'effect' | 'level' | 'seconds' | 'cooldown' | 'effects'>

const options = defineModel<VoiceFxOptions>({ required: true })

const field = <K extends keyof VoiceFxOptions>(key: K) =>
  computed<VoiceFxOptions[K]>({
    get: () => options.value[key],
    set: (value) => (options.value = { ...options.value, [key]: value })
  })

const kind = field('kind')
const effect = field('effect')
const level = field('level')
const seconds = field('seconds')
const cooldown = field('cooldown')
const effects = field('effects')

const KINDS = [
  { value: 'pick' as const, label: 'All in one', icon: 'i-lucide-sliders-horizontal' },
  { value: 'toggle' as const, label: 'Toggle', icon: 'i-lucide-toggle-right' },
  { value: 'timed' as const, label: 'Timed', icon: 'i-lucide-timer' }
]

const KIND_HELP: Record<VoiceFxOptions['kind'], string> = {
  pick: 'Whoever uses it picks the effect and the strength on the tile, then turns it on until turned off, or for a set time.',
  toggle: 'On until someone turns it off, with the effect and strength you set here.',
  timed: 'One press plays it for a set time, then it has to cool down.'
}

const EFFECT_ITEMS = VOICE_FX_EFFECTS.map((item) => ({ value: item.id, label: item.label, icon: item.icon }))

// In the order the effects are listed everywhere else.
const toggleEffect = (id: VoiceFxEffect, on: boolean): void => {
  effects.value = VOICE_FX_EFFECTS.map((item) => item.id).filter((item) => (item === id ? on : effects.value.includes(item)))
}

// USlider can hand back an array; only plain numbers are taken (see OpenShockFields.vue).
const fromSlider = (value: number | number[] | undefined, apply: (value: number) => void): void => {
  if (typeof value === 'number') apply(value)
}
</script>

<template>
  <div class="grid gap-4">
    <UFormField
      label="Kind"
      :description="KIND_HELP[kind]"
    >
      <SegmentedControl
        v-model="kind"
        :items="KINDS"
        accent
        aria-label="Kind"
      />
    </UFormField>

    <UFormField
      v-if="kind === 'pick'"
      label="Effects to pick from"
    >
      <div class="flex flex-wrap gap-2">
        <CheckChip
          v-for="item in VOICE_FX_EFFECTS"
          :id="`voicefx-effect-${item.id}`"
          :key="item.id"
          :label="item.label"
          :model-value="effects.includes(item.id)"
          @update:model-value="toggleEffect(item.id, $event)"
        />
      </div>
    </UFormField>
    <UFormField
      v-else
      label="Effect"
    >
      <SegmentedControl
        v-model="effect"
        :items="EFFECT_ITEMS"
        aria-label="Effect"
      />
    </UFormField>

    <div class="grid gap-x-7 gap-y-4 sm:grid-cols-2">
      <UFormField
        :label="kind === 'pick' ? `Strength to start from: ${Math.round(level * 100)}%` : `Strength: ${Math.round(level * 100)}%`"
        :description="kind === 'pick' ? 'The active profile can narrow the range viewers get.' : undefined"
      >
        <USlider
          :model-value="level"
          :min="0.05"
          :max="1"
          :step="0.05"
          @update:model-value="fromSlider($event, (value) => (level = value))"
        />
      </UFormField>
      <UFormField
        v-if="kind !== 'toggle'"
        :label="`Plays for: ${seconds} s`"
        :description="kind === 'pick' ? 'When they choose the timed button rather than the switch.' : undefined"
      >
        <USlider
          :model-value="seconds"
          :min="5"
          :max="Math.min(120, VOICE_FX_MAX_SECONDS)"
          :step="5"
          @update:model-value="fromSlider($event, (value) => (seconds = value))"
        />
      </UFormField>
      <UFormField
        v-if="kind !== 'toggle'"
        :label="`Cooldown: ${cooldown} s`"
        description="After it ends, before the timed button works again."
      >
        <USlider
          :model-value="cooldown"
          :min="0"
          :max="300"
          :step="5"
          @update:model-value="fromSlider($event, (value) => (cooldown = value))"
        />
      </UFormField>
    </div>
    <p class="text-sm text-dimmed">
      The tile's name is only a name: the tile always says which effect plays and how strong. One effect plays at a time, the newest one held.
    </p>
  </div>
</template>
