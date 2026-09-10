<script setup lang="ts">
import { computed } from 'vue'

import { useOscMessages } from '../composables/useOscMessages'
import type { EnumControl } from '../types/controls'
import ControlBase from './ControlBase.vue'

const { get } = useOscMessages()

const props = defineProps<{
  title: string
  icon?: string
  address: string
  items: EnumControl['options']
}>()

const emit = defineEmits<{ (e: 'update:value', index: number): void }>()

const currentValue = computed(() => get<number>(props.address, 0))
</script>

<template>
  <!-- Full custom, not a dropdown: every option is its own tappable chip, always visible, instead
  of hiding all but the current one behind a menu that has to be opened first - a dropdown is a
  particularly poor fit for a VR-overlay pointer anyway. Falls back to wrapping onto more rows
  rather than breaking if a control ever has a lot of options; real avatar controls tend to have a
  handful (outfits, hairstyles, ...), where this reads as a clear, glanceable picker instead of a
  plain closed box with the current value's name in it. -->
  <control-base
    :title="title"
    :icon="icon"
  >
    <div class="flex flex-wrap items-center justify-center gap-1.5">
      <button
        v-for="(item, index) in items"
        :key="item.value"
        type="button"
        class="flex items-center gap-1.5 rounded-full px-3 py-1.5 text-sm font-bold transition-colors"
        :class="
          item.value === currentValue
            ? 'bg-primary text-inverted'
            : 'bg-elevated text-muted hover:bg-accented'
        "
        @click.stop="emit('update:value', index)"
      >
        <UIcon
          v-if="item.icon"
          :name="item.icon"
          class="size-4"
        />
        {{ item.name }}
      </button>
    </div>
  </control-base>
</template>

<style scoped></style>
