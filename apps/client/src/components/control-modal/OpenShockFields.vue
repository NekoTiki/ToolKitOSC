<script setup lang="ts">
import CheckChip from '@renderer/components/ui/CheckChip.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import type { ControlTypes, OpenShockControl } from '@toolkitosc/shared-ui'
import { computed } from 'vue'

const props = defineProps<{
  type?: ControlTypes
  shockerList: { label: string; value: string }[]
}>()

const mode = defineModel<OpenShockControl['mode']>('mode')
const shockers = defineModel<OpenShockControl['shockers']>('shockers')
const intensity = defineModel<OpenShockControl['intensity']>('intensity')
const duration = defineModel<OpenShockControl['duration']>('duration')
const cooldown = defineModel<OpenShockControl['cooldown']>('cooldown')

// A Fixed Shocker has one value each, kept as equal min and max (the host sends the minimum).
const fixed = computed(() => props.type === 'open-shock-fixed')

const RANGE_HELP: Partial<Record<ControlTypes, string>> = {
  'open-shock-shocker': 'Each press rolls a random intensity and duration within these ranges.',
  'open-shock-choice':
    'Whoever presses chooses within these ranges. On a 2-cell tile they choose right on it; on a 1-cell tile a panel opens. Automations and the overlay roll within them.'
}

const MODES = [
  { value: 'Shock' as const, label: 'Shock', icon: 'material-symbols:electric-bolt' },
  { value: 'Vibrate' as const, label: 'Vibrate', icon: 'i-lucide-waves' }
]

const currentMode = computed({
  get: () => mode.value ?? 'Shock',
  set: (value) => (mode.value = value)
})

const toggleShocker = (id: string, on: boolean): void => {
  const rest = (shockers.value ?? []).filter((shocker) => shocker !== id)

  shockers.value = on ? [...rest, id] : rest
}

const seconds = (ms: number): string => `${(ms / 1000).toFixed(1)} s`

// USlider's own v-model sugar is unreliable for a plain single-value binding here: it always
// treats its value as a `number[]` internally and is only supposed to unwrap back to a bare
// number when there's exactly one thumb, but in practice it can hand back the raw array instead
// (same issue already worked around in OscArgsTable.vue). Since these fields are typed as `number`
// - `cooldown` is even prop-type-checked as one via defineModel - an array leaking through both
// throws a Vue prop-type warning and, for `cooldown`, makes its `v-if="typeof cooldown ===
// 'number'"` wrapper unmount the field entirely. So bind one-way and only accept plain numbers.
const onSliderUpdate = (value: number | number[] | undefined, apply: (value: number) => void): void => {
  if (typeof value === 'number') apply(value)
}
</script>

<template>
  <div class="grid gap-5">
    <div class="grid items-start gap-x-7 gap-y-5 sm:grid-cols-2">
      <UFormField label="Mode">
        <SegmentedControl
          v-model="currentMode"
          accent
          aria-label="Mode"
          class="w-fit"
          :items="MODES"
        />
      </UFormField>

      <UFormField
        label="Shockers"
        required
      >
        <div
          v-if="shockerList.length"
          class="flex flex-wrap gap-2"
        >
          <CheckChip
            v-for="shocker in shockerList"
            :id="`shocker-${shocker.value}`"
            :key="shocker.value"
            :label="shocker.label"
            :model-value="shockers?.includes(shocker.value) ?? false"
            @update:model-value="toggleShocker(shocker.value, $event)"
          />
        </div>
        <p
          v-else
          class="text-sm text-muted"
        >
          No shockers found on your OpenShock account.
        </p>
      </UFormField>
    </div>

    <div
      v-if="intensity && duration"
      class="grid gap-3"
    >
      <p
        v-if="type && RANGE_HELP[type]"
        class="text-sm text-muted"
      >
        {{ RANGE_HELP[type] }}
      </p>
      <div class="grid gap-x-7 gap-y-5 sm:grid-cols-2">
        <template v-if="fixed">
          <UFormField :label="`Intensity: ${intensity.min}%`">
            <USlider
              :model-value="intensity.min"
              :min="0"
              :max="100"
              @update:model-value="onSliderUpdate($event, (val) => ((intensity!.min = val), (intensity!.max = val)))"
            />
          </UFormField>
          <UFormField :label="`Duration: ${seconds(duration.min)}`">
            <USlider
              :model-value="duration.min"
              :min="300"
              :max="10000"
              :step="100"
              @update:model-value="onSliderUpdate($event, (val) => ((duration!.min = val), (duration!.max = val)))"
            />
          </UFormField>
        </template>
        <template v-else>
          <UFormField :label="`Lowest intensity: ${intensity.min}%`">
            <USlider
              :model-value="intensity.min"
              :min="0"
              :max="100"
              @update:model-value="onSliderUpdate($event, (val) => (intensity!.min = val))"
            />
          </UFormField>
          <UFormField :label="`Highest intensity: ${intensity.max}%`">
            <USlider
              :model-value="intensity.max"
              :min="0"
              :max="100"
              @update:model-value="onSliderUpdate($event, (val) => (intensity!.max = val))"
            />
          </UFormField>
          <UFormField :label="`Shortest: ${seconds(duration.min)}`">
            <USlider
              :model-value="duration.min"
              :min="300"
              :max="10000"
              :step="100"
              @update:model-value="onSliderUpdate($event, (val) => (duration!.min = val))"
            />
          </UFormField>
          <UFormField :label="`Longest: ${seconds(duration.max)}`">
            <USlider
              :model-value="duration.max"
              :min="300"
              :max="10000"
              :step="100"
              @update:model-value="onSliderUpdate($event, (val) => (duration!.max = val))"
            />
          </UFormField>
        </template>
        <UFormField
          v-if="typeof cooldown === 'number'"
          :label="`Cooldown: ${seconds(cooldown)}`"
          description="How long it waits after each shock ends before it can fire again."
        >
          <USlider
            :model-value="cooldown"
            :min="0"
            :max="120000"
            :step="100"
            @update:model-value="onSliderUpdate($event, (val) => (cooldown = val))"
          />
        </UFormField>
      </div>
    </div>
  </div>
</template>
