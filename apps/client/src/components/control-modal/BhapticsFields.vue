<script setup lang="ts">
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import type { BhapticsControl, BhapticsPosition } from '@toolkitosc/shared-ui'
import { BHAPTICS_EFFECTS } from '@toolkitosc/shared-ui'
import { computed } from 'vue'

// The bHaptics control's options: which part of the body the tile shows, how fine its grid is, and
// how hard and how long viewers can make it play.
export type BhapticsOptions = Pick<BhapticsControl, 'positions' | 'grid' | 'spread' | 'maxIntensity' | 'pulseMs' | 'maxHoldMs' | 'effects'>

const options = defineModel<BhapticsOptions>({ required: true })

const field = <K extends keyof BhapticsOptions>(key: K) =>
  computed<BhapticsOptions[K]>({
    get: () => options.value[key],
    set: (value) => (options.value = { ...options.value, [key]: value })
  })

const positions = field('positions')
const grid = field('grid')
const spread = field('spread')
const maxIntensity = field('maxIntensity')
const pulseMs = field('pulseMs')
const maxHoldMs = field('maxHoldMs')
const effects = field('effects')

const toggleEffect = (id: string, on: boolean): void => {
  const current = effects.value ?? []

  effects.value = on ? [...current.filter((e) => e !== id), id] : current.filter((e) => e !== id)
}

type Device = 'vest' | 'head' | 'arms' | 'hands' | 'gloves' | 'feet'
type Sides = 'both' | 'first' | 'second'

// Each device's positions, in the order its tile switches between them.
const DEVICES: Record<Device, { label: string; positions: BhapticsPosition[]; sides?: [string, string] }> = {
  vest: { label: 'Vest', positions: ['VestFront', 'VestBack'], sides: ['Front', 'Back'] },
  head: { label: 'Head', positions: ['Head'] },
  arms: { label: 'Arms', positions: ['ForearmL', 'ForearmR'], sides: ['Left', 'Right'] },
  hands: { label: 'Hands', positions: ['HandL', 'HandR'], sides: ['Left', 'Right'] },
  gloves: { label: 'Gloves', positions: ['GloveL', 'GloveR'], sides: ['Left', 'Right'] },
  feet: { label: 'Feet', positions: ['FootL', 'FootR'], sides: ['Left', 'Right'] }
}

const device = computed<Device>({
  get: () => (Object.keys(DEVICES) as Device[]).find((key) => DEVICES[key].positions.includes(positions.value[0]!)) ?? 'vest',
  set: (value) => (positions.value = [...DEVICES[value].positions])
})

const sides = computed<Sides>({
  get: () => {
    const all = DEVICES[device.value].positions

    if (positions.value.length > 1 || all.length === 1) return 'both'

    return positions.value[0] === all[0] ? 'first' : 'second'
  },
  set: (value) => {
    const all = DEVICES[device.value].positions

    positions.value = value === 'both' ? [...all] : [value === 'first' ? all[0]! : all[1]!]
  }
})

const sideItems = computed(() => {
  const labels = DEVICES[device.value].sides

  return labels ? [{ value: 'both' as const, label: 'Both' }, { value: 'first' as const, label: labels[0] }, { value: 'second' as const, label: labels[1] }] : []
})

// USlider can hand back an array; only plain numbers are taken (see OpenShockFields.vue).
const fromSlider = (value: number | number[] | undefined, apply: (value: number) => void): void => {
  if (typeof value === 'number') apply(value)
}
</script>

<template>
  <div class="grid gap-4">
    <UFormField
      label="Device"
      description="The part of the body the tile shows. Viewers hold and drag across its motors, or tap for a short pulse."
    >
      <SegmentedControl
        v-model="device"
        accent
        aria-label="Device"
        class="flex-wrap"
        :items="(Object.keys(DEVICES) as Device[]).map((key) => ({ value: key, label: DEVICES[key].label }))"
      />
    </UFormField>
    <UFormField
      v-if="sideItems.length"
      label="Sides"
      description="With both, the tile has a switch between them."
    >
      <SegmentedControl
        v-model="sides"
        accent
        aria-label="Sides"
        :items="sideItems"
      />
    </UFormField>
    <div class="grid gap-x-7 gap-y-4 sm:grid-cols-2">
      <UFormField
        label="Targets"
        description="Bigger targets are easier to hit with a VR pointer."
      >
        <SegmentedControl
          v-model="grid"
          accent
          aria-label="Targets"
          :items="[
            { value: 'motors', label: 'Every motor' },
            { value: 'coarse', label: 'Blocks' },
            { value: 'side', label: 'Whole side' }
          ]"
        />
      </UFormField>
      <UFormField
        label="Spread"
        description="How much the motors around the touched one play too."
      >
        <SegmentedControl
          v-model="spread"
          accent
          aria-label="Spread"
          :items="[
            { value: 'off', label: 'Off' },
            { value: 'soft', label: 'Soft' },
            { value: 'wide', label: 'Wide' }
          ]"
        />
      </UFormField>
      <UFormField :label="`Highest intensity: ${Math.round(maxIntensity * 100)}%`">
        <USlider
          :model-value="maxIntensity"
          :min="0.05"
          :max="1"
          :step="0.05"
          @update:model-value="fromSlider($event, (value) => (maxIntensity = value))"
        />
      </UFormField>
      <UFormField :label="`Tap length: ${(pulseMs / 1000).toFixed(1)} s`">
        <USlider
          :model-value="pulseMs"
          :min="100"
          :max="2000"
          :step="100"
          @update:model-value="fromSlider($event, (value) => (pulseMs = value))"
        />
      </UFormField>
      <UFormField
        :label="`Longest hold: ${maxHoldMs / 1000} s`"
        description="A drag is cut off after this and needs a new press."
      >
        <USlider
          :model-value="maxHoldMs / 1000"
          :min="2"
          :max="30"
          :step="1"
          @update:model-value="fromSlider($event, (value) => (maxHoldMs = value * 1000))"
        />
      </UFormField>
    </div>
    <UFormField
      label="Effects"
      description="Buttons under the grid. Each plays across every side the tile has."
    >
      <div class="flex flex-wrap gap-2">
        <label
          v-for="effect in BHAPTICS_EFFECTS"
          :key="effect.id"
          :for="`bhaptics-effect-${effect.id}`"
          class="flex min-h-12 cursor-pointer items-center gap-3 rounded-2xl border border-default bg-(--aurora-well) px-3"
        >
          <UCheckbox
            :id="`bhaptics-effect-${effect.id}`"
            :model-value="effects?.includes(effect.id) ?? false"
            @update:model-value="toggleEffect(effect.id, !!$event)"
          />
          <span :class="effects?.includes(effect.id) ? 'text-highlighted' : 'text-muted'">{{ effect.label }}</span>
        </label>
      </div>
    </UFormField>
    <p class="text-sm text-dimmed">
      Each device's strength in Settings > bHaptics and the active profile's limits for this control still apply on top. While someone touches
      that part of your avatar, viewers' taps there wait.
    </p>
  </div>
</template>
