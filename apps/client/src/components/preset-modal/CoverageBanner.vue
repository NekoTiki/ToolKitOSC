<script setup lang="ts">
import { computed } from 'vue'

// Mirrors the ratio thresholds `usePresets.ts`'s `coverage` is built from - shown wherever the
// user is about to snapshot the current state, so they know whether it's worth moving around in
// VRChat first to fill in more values before capturing.
const props = defineProps<{ captured: number; total: number }>()

const ratio = computed(() => (props.total === 0 ? 0 : props.captured / props.total))

const level = computed<'success' | 'warning' | 'error'>(() => {
  if (props.total === 0) return 'warning'
  if (ratio.value >= 0.75) return 'success'
  if (ratio.value >= 0.5) return 'warning'

  return 'error'
})

const icon = computed(() =>
  level.value === 'success' ? 'i-lucide-check-circle-2' : 'i-lucide-alert-triangle'
)

const title = computed(() => {
  const missing = props.total - props.captured

  if (props.total === 0) return 'No parameters seen yet'
  if (level.value === 'success') return `Looks complete (${props.captured}/${props.total} captured)`
  if (level.value === 'warning') {
    return `${missing} parameter${missing === 1 ? '' : 's'} not seen yet (${props.captured}/${props.total} captured)`
  }

  return `Too early - most parameters haven't been seen yet (${props.captured}/${props.total} captured)`
})
</script>

<template>
  <UAlert
    :color="level"
    variant="subtle"
    :icon="icon"
    :title="title"
    description="VRChat only reports a parameter's value once it changes in-game - it doesn't send the full list on load. Move around, toggle expressions, and open menus that touch the parameters you want, then capture or refresh the preset."
  />
</template>

<style scoped></style>
