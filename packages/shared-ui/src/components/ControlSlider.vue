<script setup lang="ts">
import { useOscMessages } from '../composables/useOscMessages'
import type { ControlLimits } from '../types/controls'
import { limitRange } from '../utils/controlLimits'
import ControlSliderBase from './ControlSliderBase.vue'

const { get } = useOscMessages()

defineProps<{
  title: string
  icon?: string
  address: string
  debounceMs?: number
  limits?: ControlLimits
}>()

const emit = defineEmits<{ (e: 'update:value', value: number): void }>()
</script>

<template>
  <ControlSliderBase
    :title="title"
    :icon="icon"
    :model-value="(get<number>(address, 0) || 0) * 100"
    :debounce-ms="debounceMs"
    :min="limits && limitRange(limits).min * 100"
    :max="limits && limitRange(limits).max * 100"
    @update:model-value="emit('update:value', $event / 100)"
  />
</template>

<style scoped></style>
