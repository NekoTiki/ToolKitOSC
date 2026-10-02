<script setup lang="ts" generic="T extends string">
// A row of large, mutually exclusive buttons (tile size, Use/Edit...). All choices stay visible,
// which suits a VR pointer better than a dropdown.
defineProps<{
  items: { value: T; label?: string; icon?: string; title?: string }[]
  // Highlight the chosen button in the primary color instead of a neutral lift.
  accent?: boolean
  ariaLabel?: string
}>()

const model = defineModel<T>({ required: true })
</script>

<template>
  <div
    class="flex shrink-0 gap-0.5 rounded-[calc(var(--ui-radius)*4)] glass p-1"
    role="group"
    :aria-label="ariaLabel"
  >
    <button
      v-for="item in items"
      :key="item.value"
      type="button"
      class="flex h-10 min-w-11 cursor-pointer items-center justify-center gap-1.5 rounded-2xl px-3.5 text-sm font-medium transition-colors"
      :class="
        model === item.value
          ? accent
            ? 'bg-primary text-inverted'
            : 'bg-accented text-highlighted'
          : 'text-muted hover:text-default'
      "
      :aria-pressed="model === item.value"
      :title="item.title ?? item.label"
      @click="model = item.value"
    >
      <UIcon
        v-if="item.icon"
        :name="item.icon"
        class="size-4"
      />
      <span v-if="item.label">{{ item.label }}</span>
    </button>
  </div>
</template>
