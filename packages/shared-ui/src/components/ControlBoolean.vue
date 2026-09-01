<script setup lang="ts">
import { computed } from 'vue'

import { useOscMessages } from '../composables/useOscMessages'
import ControlBooleanBase from './ControlBooleanBase.vue'

const { get } = useOscMessages()

const props = defineProps<{
  title: string
  icon?: string
  reverse?: boolean
  address: string
}>()

const emit = defineEmits<{ (e: 'update:value', value: boolean): void }>()

const state = computed({
  get: () => {
    if (props.reverse) return !get<boolean>(props.address, false)

    return get<boolean>(props.address, false)
  },
  set: (newValue: boolean) => {
    const valueToSet = props.reverse ? !newValue : newValue

    emit('update:value', valueToSet)
  }
})
</script>

<template>
  <control-boolean-base
    v-model="state"
    :title="title"
    :icon="icon"
  />
</template>

<style scoped></style>
