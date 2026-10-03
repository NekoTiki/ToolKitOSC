<script setup lang="ts">
import { useIntifaceControl } from '../composables/useIntifaceControl'
import type { ControlLimits } from '../types/controls'
import { limitRange } from '../utils/controlLimits'
import ControlSliderBase from './ControlSliderBase.vue'

// Same radial-dial widget as ControlSlider.vue, but its value comes from the shared Intiface
// control-value map (kept in sync across host/viewers via 'intiface-value-update') instead of the
// live OSC parameter cache - there's no avatar parameter backing an Intiface toy.
defineProps<{
  controlId: string
  title: string
  icon?: string
  debounceMs?: number
  limits?: ControlLimits
}>()

const emit = defineEmits<{ (e: 'update:value', value: number): void }>()

const { controlValue } = useIntifaceControl()
</script>

<template>
  <ControlSliderBase
    :title="title"
    :icon="icon"
    :model-value="(controlValue.get(controlId) ?? 0) * 100"
    :debounce-ms="debounceMs"
    :min="limits && limitRange(limits).min * 100"
    :max="limits && limitRange(limits).max * 100"
    @update:model-value="emit('update:value', $event / 100)"
  />
</template>

<style scoped></style>
