<script setup lang="ts">
import { computed } from 'vue'

// A number with a small bar showing where it sits in its range: floats over 0 -> 1 (shown to two
// decimals), ints over VRChat's 0 -> 255. Values outside the range pin the bar to its end.
const props = defineProps<{
  value: number
  // The parameter's declared type. Without it (an address the avatar doesn't declare), a whole
  // number is treated as an int.
  kind?: 'Bool' | 'Float' | 'Int'
}>()

const INT_MAX = 255

const isFloat = computed(() => (props.kind ? props.kind === 'Float' : !Number.isInteger(props.value)))
const text = computed(() => (isFloat.value ? props.value.toFixed(2) : String(props.value)))
const fill = computed(() => Math.min(Math.max(isFloat.value ? props.value : props.value / INT_MAX, 0), 1))
</script>

<template>
  <span class="flex min-w-0 items-center gap-2">
    <span class="w-11 shrink-0 text-right font-mono text-sm tabular-nums">{{ text }}</span>
    <span
      class="h-1.5 w-12 shrink-0 overflow-hidden rounded-full bg-(--aurora-well)"
      role="meter"
      :aria-valuenow="value"
      :aria-valuemin="0"
      :aria-valuemax="isFloat ? 1 : INT_MAX"
    >
      <span
        class="block h-full rounded-full bg-secondary"
        :style="{ width: `${fill * 100}%` }"
      />
    </span>
  </span>
</template>
