<script setup lang="ts">
import { computed } from 'vue'

import { useOscMessages } from '../composables/useOscMessages'
import ControlBooleanBase from './ControlBooleanBase.vue'

const { get } = useOscMessages()

const props = defineProps<{
  title: string
  icon?: string
  inputs: { address: string }[]
}>()

const emit = defineEmits<{ (e: 'update:value', value: boolean): void }>()

const state = computed({
  get: () => {
    const values = props.inputs.map((input) => get<boolean>(input.address, false))

    return values.every((v) => v)
  },
  set: (newValue: boolean) => {
    emit('update:value', newValue)
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
