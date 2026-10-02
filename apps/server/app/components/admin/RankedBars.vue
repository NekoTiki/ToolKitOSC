<script setup lang="ts">
// Ranked horizontal bars drawn with plain elements: label, bar, value. Easier to compare than the
// old doughnuts, and every value is printed, so nothing depends on hovering a tooltip.
const props = withDefaults(
  defineProps<{ rows: { label: string; value: number }[]; format?: (value: number) => string; max?: number }>(),
  { format: (value: number) => value.toLocaleString('en-US'), max: undefined }
)

const scale = computed(() => props.max ?? Math.max(1, ...props.rows.map((row) => row.value)))
</script>

<template>
  <div
    v-if="rows.length"
    class="grid gap-1.5"
  >
    <div
      v-for="row in rows"
      :key="row.label"
      class="grid min-h-7.5 grid-cols-[6rem_minmax(0,1fr)_3rem] items-center gap-2.5 rounded-lg px-1 text-[13px] sm:grid-cols-[8.5rem_minmax(0,1fr)_4rem]"
      :title="`${row.label}: ${format(row.value)}`"
    >
      <span class="truncate">{{ row.label }}</span>
      <span class="relative h-3.5 border-l border-(--ui-text-dimmed)">
        <i
          class="absolute inset-y-0 left-0 rounded-r bg-secondary"
          :style="{ width: `${(row.value / scale) * 100}%` }"
        />
      </span>
      <span class="text-right font-mono text-xs text-muted tabular-nums">{{ format(row.value) }}</span>
    </div>
  </div>
  <p
    v-else
    class="py-6 text-center text-sm text-muted"
  >
    Nothing in this range.
  </p>
</template>
