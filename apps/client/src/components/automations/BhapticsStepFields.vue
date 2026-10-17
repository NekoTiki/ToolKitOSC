<script setup lang="ts">
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import type { BhapticsEffectAction, BhapticsPulseAction } from '@renderer/lib/tauri-bridge'
import type { BhapticsPosition } from '@toolkitosc/shared-ui'
import { BHAPTICS_EFFECTS, BHAPTICS_LAYOUT } from '@toolkitosc/shared-ui'
import { computed } from 'vue'

// The fields of the two bHaptics steps: a pulse on some motors of one device, or a built-in effect
// across several.
const step = defineModel<BhapticsPulseAction | BhapticsEffectAction>({ required: true })

const POSITIONS = Object.keys(BHAPTICS_LAYOUT) as BhapticsPosition[]
// The vest's sides say which side they are; the layout's short labels are for the tile's switch.
const label = (position: BhapticsPosition): string =>
  position === 'VestFront' ? 'Vest front' : position === 'VestBack' ? 'Vest back' : BHAPTICS_LAYOUT[position].label

const pulse = computed(() => (step.value.kind === 'bhaptics-pulse' ? step.value : null))
const effect = computed(() => (step.value.kind === 'bhaptics-effect' ? step.value : null))

const layout = computed(() => (pulse.value ? BHAPTICS_LAYOUT[pulse.value.position] : null))

const setPosition = (position: BhapticsPosition): void => {
  if (!pulse.value) return
  pulse.value.position = position
  pulse.value.motors = []
}

const toggleMotor = (motor: number): void => {
  if (!pulse.value) return
  const motors = pulse.value.motors

  pulse.value.motors = motors.includes(motor) ? motors.filter((m) => m !== motor) : [...motors, motor].sort((a, b) => a - b)
}

const togglePosition = (position: BhapticsPosition): void => {
  if (!effect.value) return
  const positions = effect.value.positions

  effect.value.positions = positions.includes(position) ? positions.filter((p) => p !== position) : POSITIONS.filter((p) => p === position || positions.includes(p))
}

// USlider can hand back an array; only plain numbers are taken (see OpenShockFields.vue).
const fromSlider = (value: number | number[] | undefined, apply: (value: number) => void): void => {
  if (typeof value === 'number') apply(value)
}
</script>

<template>
  <div
    v-if="pulse && layout"
    class="grid gap-4"
  >
    <UFormField label="Device">
      <USelect
        :model-value="pulse.position"
        :items="POSITIONS.map((position) => ({ value: position, label: label(position) }))"
        class="w-56"
        @update:model-value="setPosition($event as BhapticsPosition)"
      />
    </UFormField>
    <UFormField
      label="Motors"
      :description="pulse.motors.length ? `${pulse.motors.length} picked` : 'None picked: every motor plays.'"
    >
      <div
        class="grid w-fit gap-1.5 rounded-2xl bg-(--aurora-well) p-2.5"
        :style="{ gridTemplateColumns: `repeat(${layout.columns}, 2rem)` }"
      >
        <button
          v-for="motor in layout.motors"
          :key="motor"
          type="button"
          class="size-8 cursor-pointer rounded-lg text-[11px] font-semibold tabular-nums transition-colors"
          :class="pulse.motors.includes(motor - 1) ? 'bg-warning text-inverted' : 'bg-(--aurora-glass) text-muted hover:text-highlighted'"
          :aria-pressed="pulse.motors.includes(motor - 1)"
          @click="toggleMotor(motor - 1)"
        >
          {{ motor - 1 }}
        </button>
      </div>
    </UFormField>
    <div class="grid gap-x-7 gap-y-4 sm:grid-cols-2">
      <UFormField :label="`Intensity: ${Math.round(pulse.intensity * 100)}%`">
        <USlider
          :model-value="pulse.intensity"
          :min="0.05"
          :max="1"
          :step="0.05"
          @update:model-value="fromSlider($event, (value) => (pulse!.intensity = value))"
        />
      </UFormField>
      <UFormField :label="`Length: ${(pulse.ms / 1000).toFixed(1)} s`">
        <USlider
          :model-value="pulse.ms"
          :min="100"
          :max="30_000"
          :step="100"
          @update:model-value="fromSlider($event, (value) => (pulse!.ms = value))"
        />
      </UFormField>
    </div>
  </div>

  <div
    v-else-if="effect"
    class="grid gap-4"
  >
    <UFormField label="Effect">
      <SegmentedControl
        v-model="effect.effect"
        class="w-fit"
        aria-label="Effect"
        :items="BHAPTICS_EFFECTS.map((e) => ({ value: e.id, label: e.label }))"
      />
    </UFormField>
    <UFormField
      label="Where"
      :error="effect.positions.length ? false : 'Pick where it plays.'"
    >
      <div class="flex flex-wrap gap-1.5">
        <button
          v-for="position in POSITIONS"
          :key="position"
          type="button"
          class="rounded-full border px-3 py-1.5 text-[13px] transition-colors"
          :class="effect.positions.includes(position) ? 'border-primary/50 bg-primary/14 text-highlighted' : 'border-transparent bg-(--aurora-well) text-muted hover:bg-(--aurora-glass)'"
          :aria-pressed="effect.positions.includes(position)"
          @click="togglePosition(position)"
        >
          {{ label(position) }}
        </button>
      </div>
    </UFormField>
    <UFormField :label="`Intensity: ${Math.round(effect.intensity * 100)}%`">
      <USlider
        :model-value="effect.intensity"
        :min="0.05"
        :max="1"
        :step="0.05"
        class="max-w-sm"
        @update:model-value="fromSlider($event, (value) => (effect!.intensity = value))"
      />
    </UFormField>
  </div>
</template>
