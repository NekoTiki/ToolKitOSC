<script setup lang="ts" generic="T extends string | number">
// A row of mutually exclusive buttons in a glass well: the dashboard's date range, the Users
// filter and the provider chart's view. Buttons instead of a dropdown, so every choice is one
// press away for a VR laser pointer.
withDefaults(defineProps<{ items: readonly { value: T; label: string }[]; label: string; size?: 'sm' | 'md' }>(), { size: 'md' })

const model = defineModel<T>({ required: true })
</script>

<template>
  <div
    class="flex w-fit max-w-full gap-0.5 overflow-x-auto rounded-[calc(var(--ui-radius)*4)] glass p-1"
    role="group"
    :aria-label="label"
  >
    <button
      v-for="item in items"
      :key="item.value"
      type="button"
      class="shrink-0 cursor-pointer rounded-2xl font-medium whitespace-nowrap transition-colors"
      :class="[
        size === 'sm' ? 'h-8.5 px-3 text-[12.5px]' : 'h-10 px-3.5 text-sm',
        model === item.value ? 'bg-accented text-highlighted' : 'text-muted hover:text-default'
      ]"
      :aria-pressed="model === item.value"
      @click="model = item.value"
    >
      {{ item.label }}
    </button>
  </div>
</template>
