<script setup lang="ts">
import CheckChip from '@renderer/components/ui/CheckChip.vue'
import type { OwoControl, OwoMuscle } from '@toolkitosc/shared-ui'
import { OWO_MIN_COOLDOWN_MS, OWO_MUSCLES, OWO_SENSATIONS } from '@toolkitosc/shared-ui'
import { computed } from 'vue'

// The OWO control's options: which muscles viewers reach, which sensations they get, and how hard,
// how long and how often. It's electrical, so the defaults are a shocker's: low, slow and signed in.
export type OwoOptions = Pick<OwoControl, 'muscles' | 'maxIntensity' | 'maxDurationMs' | 'sensations' | 'cooldown' | 'signedInOnly'>

const options = defineModel<OwoOptions>({ required: true })

const field = <K extends keyof OwoOptions>(key: K) =>
  computed<OwoOptions[K]>({
    get: () => options.value[key],
    set: (value) => (options.value = { ...options.value, [key]: value })
  })

const muscles = field('muscles')
const maxIntensity = field('maxIntensity')
const maxDurationMs = field('maxDurationMs')
const sensations = field('sensations')
const cooldown = field('cooldown')
const signedInOnly = field('signedInOnly')

const ALL = Object.keys(OWO_MUSCLES) as OwoMuscle[]

const toggle = <T,>(list: T[], item: T, on: boolean): T[] => (on ? [...list.filter((i) => i !== item), item] : list.filter((i) => i !== item))

// USlider can hand back an array; only plain numbers are taken (see OpenShockFields.vue).
const fromSlider = (value: number | number[] | undefined, apply: (value: number) => void): void => {
  if (typeof value === 'number') apply(value)
}
</script>

<template>
  <div class="grid gap-4">
    <UAlert
      color="error"
      variant="subtle"
      icon="i-lucide-zap"
      title="Electrical, like a shocker"
      description="The OWO Skin's limits you calibrated in the OWO app still apply under these. Keep the intensity low until you know how it feels."
    />
    <UFormField
      v-for="side in ['front', 'back'] as const"
      :key="side"
      :label="side === 'front' ? 'Front muscles viewers can reach' : 'Back muscles viewers can reach'"
    >
      <div class="flex flex-wrap gap-2">
        <CheckChip
          v-for="muscle in ALL.filter((m) => OWO_MUSCLES[m].side === side)"
          :id="`owo-muscle-${muscle}`"
          :key="muscle"
          :label="OWO_MUSCLES[muscle].label"
          :model-value="muscles.includes(muscle)"
          @update:model-value="muscles = toggle(muscles, muscle, $event)"
        />
      </div>
    </UFormField>
    <UFormField
      label="Sensations"
      description="Buttons on the tile. Viewers pick one, then tap a muscle."
    >
      <div class="flex flex-wrap gap-2">
        <CheckChip
          v-for="sensation in OWO_SENSATIONS"
          :id="`owo-sensation-${sensation.id}`"
          :key="sensation.id"
          :label="sensation.label"
          :model-value="sensations.includes(sensation.id)"
          @update:model-value="sensations = toggle(sensations, sensation.id, $event)"
        />
      </div>
    </UFormField>
    <div class="grid gap-x-7 gap-y-4 sm:grid-cols-2">
      <UFormField :label="`Highest intensity: ${Math.round(maxIntensity * 100)}%`">
        <USlider
          :model-value="maxIntensity"
          :min="0.05"
          :max="1"
          :step="0.05"
          @update:model-value="fromSlider($event, (value) => (maxIntensity = value))"
        />
      </UFormField>
      <UFormField
        :label="`Longest sensation: ${(maxDurationMs / 1000).toFixed(1)} s`"
        description="Longer ones are cut short."
      >
        <USlider
          :model-value="maxDurationMs"
          :min="100"
          :max="2000"
          :step="100"
          @update:model-value="fromSlider($event, (value) => (maxDurationMs = value))"
        />
      </UFormField>
      <UFormField
        :label="`Cooldown: ${cooldown / 1000} s`"
        description="Time between two taps, from anyone."
      >
        <USlider
          :model-value="cooldown / 1000"
          :min="OWO_MIN_COOLDOWN_MS / 1000"
          :max="30"
          :step="1"
          @update:model-value="fromSlider($event, (value) => (cooldown = value * 1000))"
        />
      </UFormField>
    </div>
    <USwitch
      :model-value="signedInOnly"
      label="Only viewers signed in with Discord"
      description="Off: anyone with your link can use it."
      @update:model-value="signedInOnly = !!$event"
    />
    <p class="text-sm text-dimmed">
      Each muscle's strength in Settings > OWO and the active profile's limits for this control still apply on top. While someone touches
      that side of your avatar, viewers' taps there wait.
    </p>
  </div>
</template>
