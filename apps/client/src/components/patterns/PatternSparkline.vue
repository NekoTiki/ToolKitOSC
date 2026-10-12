<script setup lang="ts">
import type { IntifacePatternDefinition } from '@renderer/composables/useIntifacePatterns'
import { computed } from 'vue'

// A small preview line of a pattern. A recording is drawn across one whole loop; a built-in pattern,
// which has no length of its own, across a few seconds.
const props = defineProps<{
  pattern: IntifacePatternDefinition
  mine?: boolean
}>()

const BUILTIN_WINDOW_MS = 4000
const POINTS = 48

const points = computed(() => {
  const span = props.pattern.durationMs ?? BUILTIN_WINDOW_MS

  return Array.from({ length: POINTS + 1 }, (_, i) => {
    const value = props.pattern.value(Math.min((i / POINTS) * span, span - 1))

    return `${((i / POINTS) * 64).toFixed(1)},${(22 - value * 20).toFixed(1)}`
  }).join(' ')
})
</script>

<template>
  <svg
    viewBox="0 0 64 24"
    aria-hidden="true"
    class="h-5.5 w-15 fill-none stroke-2"
    :class="mine ? 'stroke-secondary' : 'stroke-(--ui-text-dimmed)'"
    stroke-linejoin="round"
  >
    <polyline :points="points" />
  </svg>
</template>
