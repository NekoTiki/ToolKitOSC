<script setup lang="ts">
import { computed } from 'vue'

import { useOscMessages } from '../composables/useOscMessages'
import type { EnumControl } from '../types/controls'
import ControlBase from './ControlBase.vue'
import ControlOptions from './ControlOptions.vue'

const { get } = useOscMessages()

const props = defineProps<{
  title: string
  icon?: string
  // Option keys the active profile blocks for viewers (see ControlLimits).
  disabledOptions?: (string | number)[]
  address: string
  items: EnumControl['options']
}>()

const emit = defineEmits<{ (e: 'update:value', index: number): void }>()

const currentValue = computed(() => get<number>(props.address, 0))

const options = computed(() =>
  props.items.map((item) => ({ key: item.value, name: item.name, icon: item.icon, disabled: !!props.disabledOptions?.includes(item.value), selected: item.value === currentValue.value }))
)
</script>

<template>
  <control-base
    :title="title"
    :icon="icon"
    kind="value"
    fill
  >
    <ControlOptions
      :options="options"
      @select="emit('update:value', $event)"
    />
  </control-base>
</template>
