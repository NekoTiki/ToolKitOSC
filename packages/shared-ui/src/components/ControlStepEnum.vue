<script setup lang="ts">
import { computed } from 'vue'

import { useOscMessages } from '../composables/useOscMessages'
import type { StepEnumControl } from '../types/controls'
import ControlBase from './ControlBase.vue'

// A one-cell alternative to the Enum picker: shows only the current option, with large previous /
// next buttons that wrap around. Same data and command as Enum (an option index).
const { get } = useOscMessages()

const props = defineProps<{
  title: string
  icon?: string
  address: string
  items: StepEnumControl['options']
}>()

const emit = defineEmits<{ (e: 'update:value', index: number): void }>()

const currentIndex = computed(() => {
  const value = get<number>(props.address, 0)
  const index = props.items.findIndex((item) => item.value === value)

  return index === -1 ? 0 : index
})

const current = computed(() => props.items[currentIndex.value])

const step = (delta: number): void => {
  if (!props.items.length) return

  emit('update:value', (currentIndex.value + delta + props.items.length) % props.items.length)
}
</script>

<template>
  <control-base
    :title="title"
    :icon="icon"
    kind="value"
  >
    <template #middle>
      <div class="grid min-w-0 max-w-full gap-1">
        <span
          class="truncate text-(length:--tile-value) leading-none font-semibold text-highlighted"
          :title="current?.name"
        >{{ current?.name ?? '—' }}</span>
        <span class="font-mono text-[11px] text-muted">{{ currentIndex + 1 }} of {{ items.length }}</span>
      </div>
    </template>

    <div class="grid grid-cols-2 gap-1.5">
      <button
        type="button"
        aria-label="Previous option"
        class="grid h-(--tile-option) cursor-pointer place-items-center rounded-field well text-highlighted transition-colors hover:bg-accented"
        @click.stop="step(-1)"
      >
        <UIcon
          name="i-lucide-chevron-left"
          class="size-5"
        />
      </button>
      <button
        type="button"
        aria-label="Next option"
        class="grid h-(--tile-option) cursor-pointer place-items-center rounded-field well text-highlighted transition-colors hover:bg-accented"
        @click.stop="step(1)"
      >
        <UIcon
          name="i-lucide-chevron-right"
          class="size-5"
        />
      </button>
    </div>
  </control-base>
</template>
