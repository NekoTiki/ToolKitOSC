<script setup lang="ts">
import type { SelectItem } from '@nuxt/ui/components/Select.vue'
import { computed, ref } from 'vue'

import { useOscMessages } from '../composables/useOscMessages'
import type { EnumControl } from '../types/controls'
import ControlBase from './ControlBase.vue'

const { get } = useOscMessages()

const props = defineProps<{
  title: string
  address: string
  items: (SelectItem & EnumControl['options'][number])[]
}>()

const emit = defineEmits<{ (e: 'update:value', index: number): void }>()

const state = computed({
  get: () => get<number>(props.address, 0),
  set: (newValue: number) => {
    const itemIndex = props.items.findIndex((item) => item.value === newValue)

    emit('update:value', itemIndex)
  }
})

const selectRef = ref<HTMLButtonElement | null>(null)

const open = ref(false)
</script>

<template>
  <control-base
    :title="title"
    @click="open = !open"
  >
    <USelect
      ref="selectRef"
      v-model:open="open"
      v-model="state"
      label-key="name"
      value-key="value"
      size="xl"
      :items="items"
      class="w-full"
    />
  </control-base>
</template>

<style scoped></style>
