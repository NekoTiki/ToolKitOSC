<script setup lang="ts">
import { useOscMessages } from '../composables/useOscMessages'
import ControlSliderBase from './ControlSliderBase.vue'

const { get } = useOscMessages()

defineProps<{
  title: string
  icon?: string
  address: string
  debounceMs?: number
}>()

const emit = defineEmits<{ (e: 'update:value', value: number): void }>()
</script>

<template>
  <ControlSliderBase
    :title="title"
    :icon="icon"
    :model-value="(get<number>(address, 0) || 0) * 100"
    :debounce-ms="debounceMs"
    @update:model-value="emit('update:value', $event / 100)"
  />
</template>

<style scoped></style>
