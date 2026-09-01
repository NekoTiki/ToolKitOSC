<script setup lang="ts">
import type { SelectItem } from '@nuxt/ui/components/Select.vue'
import _ from 'lodash'
import { computed, ref } from 'vue'

import { useOscMessages } from '../composables/useOscMessages'
import type { BooleanEnumControl } from '../types/controls'
import ControlBase from './ControlBase.vue'

const { get } = useOscMessages()

const props = defineProps<{
  title: string
  items: (SelectItem & BooleanEnumControl['inputs'][number])[]
}>()

const emit = defineEmits<{ (e: 'update:value', index: number): void }>()

const addresses = computed((): string[] => {
  return _.uniq(
    _.flattenDeep(props.items.map((item) => [item.inputAddress.true, item.inputAddress.false]))
  )
})

const addressesValues = computed((): Map<string, boolean> => {
  return new Map(_.map(addresses.value, (address) => [address, get<boolean>(address, false)]))
})

const state = computed({
  get: () => {
    const entries = Array.from(addressesValues.value.entries())

    const grouped = _.groupBy(entries, ([, val]) => (val ? 'true' : 'false'))
    const data = {
      true: _.map(grouped.true, ([addr]) => addr),
      false: _.map(grouped.false, ([addr]) => addr)
    }

    return _.get(
      _.find(
        props.items,
        ({ inputAddress }) =>
          _.every(inputAddress.true, (addr) => _.includes(data.true, addr)) &&
          _.every(inputAddress.false, (addr) => _.includes(data.false, addr))
      ),
      'id'
    )
  },

  set: (newValue) => {
    const itemIndex = _.findIndex(props.items, { id: newValue })

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
      value-key="id"
      size="xl"
      :items="items"
      class="w-full"
    />
  </control-base>
</template>

<style scoped></style>
