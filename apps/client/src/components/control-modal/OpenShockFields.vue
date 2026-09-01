<script setup lang="ts">
import type { SelectMenuItem } from '@nuxt/ui/components/SelectMenu.vue'
import type { OpenShockControl } from '@vrc-osc-toolkit/shared-ui'

defineProps<{
  openShockMode: SelectMenuItem[]
  shockerList: SelectMenuItem[]
}>()

const mode = defineModel<OpenShockControl['mode']>('mode')
const shockers = defineModel<OpenShockControl['shockers']>('shockers')
const intensity = defineModel<OpenShockControl['intensity']>('intensity')
const duration = defineModel<OpenShockControl['duration']>('duration')
const cooldown = defineModel<OpenShockControl['cooldown']>('cooldown')
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
          v-model="intensity.min"
          type="number"
          :min="0"
          :max="100"
          class="w-full"
        />
      </div>
      <div class="flex flex-col gap-2">
        Max: {{ intensity.max }}
        <USlider
          v-model="intensity.max"
          type="number"
          :min="0"
          :max="100"
          class="w-full"
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
          v-model="duration.min"
          type="number"
          :min="300"
          :max="10000"
          :step="100"
          class="w-full"
        />
      </div>
      <div class="flex flex-col gap-2">
        Max: {{ duration.max / 1000 }}s
        <USlider
          v-model="duration.max"
          type="number"
          :min="300"
          :max="10000"
          :step="100"
          class="w-full"
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
          v-model="cooldown"
          type="number"
          :min="0"
          :max="120000"
          :step="100"
          class="w-full"
        />
      </div>
    </div>
  </UFormField>
</template>

<style scoped></style>
