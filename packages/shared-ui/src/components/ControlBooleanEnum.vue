<script setup lang="ts">
import _ from 'lodash'
import { computed } from 'vue'

import { useOscMessages } from '../composables/useOscMessages'
import type { BooleanEnumControl } from '../types/controls'
import ControlBase from './ControlBase.vue'
import ControlOptions from './ControlOptions.vue'

const { get } = useOscMessages()

const props = defineProps<{
  title: string
  icon?: string
  items: BooleanEnumControl['inputs']
}>()

const emit = defineEmits<{ (e: 'update:value', index: number): void }>()

// The editor starts each list with one empty row, so an option left with no "true" address is
// stored as `true: ['']`. Blank rows aren't addresses; dropping them lets an all-false option match.
const filled = (list: string[]): string[] => list.filter((addr) => addr.trim())

const addresses = computed((): string[] => {
  return _.uniq(
    _.flattenDeep(props.items.map((item) => [filled(item.inputAddress.true), filled(item.inputAddress.false)]))
  )
})

const addressesValues = computed((): Map<string, boolean> => {
  return new Map(_.map(addresses.value, (address) => [address, get<boolean>(address, false)]))
})

const currentId = computed((): string | undefined => {
  const entries = Array.from(addressesValues.value.entries())

  const grouped = _.groupBy(entries, ([, val]) => (val ? 'true' : 'false'))
  const data = {
    true: _.map(grouped.true, ([addr]) => addr),
    false: _.map(grouped.false, ([addr]) => addr)
  }

  return _.find(
    props.items,
    ({ inputAddress }) =>
      _.every(filled(inputAddress.true), (addr) => _.includes(data.true, addr)) &&
      _.every(filled(inputAddress.false), (addr) => _.includes(data.false, addr))
  )?.id
})

const options = computed(() =>
  props.items.map((item) => ({ key: item.id, name: item.name, icon: item.icon, selected: item.id === currentId.value }))
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
