<script setup lang="ts" generic="T extends string">
// Mutually exclusive choices laid out as a grid of equal tiles, for when there are too many (or
// too long) to sit in one SegmentedControl row: wrapping a segmented row leaves a ragged second
// line, a grid keeps every row aligned. All choices stay visible, as for a VR pointer.
withDefaults(
  defineProps<{
    items: { value: T; label: string; icon?: string; title?: string }[]
    ariaLabel?: string
    // The narrowest a tile gets before the grid drops a column.
    minWidth?: string
  }>(),
  { ariaLabel: undefined, minWidth: '10rem' }
)

const model = defineModel<T>({ required: true })
</script>

<template>
  <div
    class="grid gap-1.5"
    :style="{ gridTemplateColumns: `repeat(auto-fill, minmax(min(${minWidth}, 100%), 1fr))` }"
    role="group"
    :aria-label="ariaLabel"
  >
    <button
      v-for="item in items"
      :key="item.value"
      type="button"
      class="flex h-11 min-w-0 cursor-pointer items-center gap-2 rounded-field border px-3.5 text-left text-sm font-medium transition-colors"
      :class="
        model === item.value
          ? 'border-primary bg-primary/14 text-highlighted shadow-[inset_0_0_0_1px_var(--ui-primary)]'
          : 'border-default bg-(--aurora-glass) text-muted hover:bg-elevated hover:text-default'
      "
      :aria-pressed="model === item.value"
      :title="item.title ?? item.label"
      @click="model = item.value"
    >
      <UIcon
        v-if="item.icon"
        :name="item.icon"
        class="size-4 shrink-0"
        :class="model === item.value ? 'text-primary' : ''"
      />
      <span class="min-w-0 truncate">{{ item.label }}</span>
    </button>
  </div>
</template>
