<script setup lang="ts">
import type { SelectMenuItem } from '@nuxt/ui/components/SelectMenu.vue'
import type { OpenShockControl } from '@toolkitosc/shared-ui'

defineProps<{
  openShockMode: SelectMenuItem[]
  shockerList: SelectMenuItem[]
}>()

const mode = defineModel<OpenShockControl['mode']>('mode')
const shockers = defineModel<OpenShockControl['shockers']>('shockers')
const intensity = defineModel<OpenShockControl['intensity']>('intensity')
const duration = defineModel<OpenShockControl['duration']>('duration')
const cooldown = defineModel<OpenShockControl['cooldown']>('cooldown')

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
  <UFormField
    label="Shock Mode"
    required
  >
    <USelect
      v-model="mode"
      :items="openShockMode"
      placeholder="Select Shockers"
      value-key="value"
      class="w-full"
      virtualize
    />
  </UFormField>
  <UFormField
    label="Shockers"
    required
  >
    <USelect
      v-model="shockers"
      :items="shockerList"
      placeholder="Select Shockers"
      value-key="value"
      class="w-full"
      multiple
      virtualize
    />
  </UFormField>
  <UFormField
    v-if="intensity"
    label="Intensity"
    required
  >
    <div class="grid grid-cols-2 gap-4">
      <div class="flex flex-col gap-2">
        Min: {{ intensity.min }}
        <USlider
          :model-value="intensity.min"
          :min="0"
          :max="100"
          class="w-full"
          @update:model-value="onSliderUpdate($event, (val) => intensity && (intensity.min = val))"
        />
      </div>
      <div class="flex flex-col gap-2">
        Max: {{ intensity.max }}
        <USlider
          :model-value="intensity.max"
          :min="0"
          :max="100"
          class="w-full"
          @update:model-value="onSliderUpdate($event, (val) => intensity && (intensity.max = val))"
        />
      </div>
    </div>
  </UFormField>
  <UFormField
    v-if="duration"
    label="Duration"
    required
  >
    <div class="grid grid-cols-2 gap-4">
      <div class="flex flex-col gap-2">
        Min: {{ duration.min / 1000 }}s
        <USlider
          :model-value="duration.min"
          :min="300"
          :max="10000"
          :step="100"
          class="w-full"
          @update:model-value="onSliderUpdate($event, (val) => duration && (duration.min = val))"
        />
      </div>
      <div class="flex flex-col gap-2">
        Max: {{ duration.max / 1000 }}s
        <USlider
          :model-value="duration.max"
          :min="300"
          :max="10000"
          :step="100"
          class="w-full"
          @update:model-value="onSliderUpdate($event, (val) => duration && (duration.max = val))"
        />
      </div>
    </div>
  </UFormField>
  <UFormField
    v-if="typeof cooldown === 'number'"
    label="CoolDown"
    required
  >
    <div class="grid gap-4">
      <div class="flex flex-col gap-2">
        Min: {{ cooldown / 1000 }}s
        <USlider
          :model-value="cooldown"
          :min="0"
          :max="120000"
          :step="100"
          class="w-full"
          @update:model-value="onSliderUpdate($event, (val) => (cooldown = val))"
        />
      </div>
    </div>
  </UFormField>
</template>

<style scoped></style>
