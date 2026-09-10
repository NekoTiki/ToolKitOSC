<script setup lang="ts">
import _ from 'lodash'
import { computed } from 'vue'

import { useOscMessages } from '../composables/useOscMessages'
import type { BooleanEnumControl } from '../types/controls'
import ControlBase from './ControlBase.vue'

const { get } = useOscMessages()

const props = defineProps<{
  title: string
  icon?: string
  items: BooleanEnumControl['inputs']
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
      _.every(inputAddress.true, (addr) => _.includes(data.true, addr)) &&
      _.every(inputAddress.false, (addr) => _.includes(data.false, addr))
  )?.id
})
</script>

<template>
  <!-- Full custom, not a dropdown - same chip-picker treatment as ControlEnum.vue, for the same
  reason: every option tappable and visible at once beats hiding all but the current one behind a
  menu, especially from a VR-overlay pointer. -->
  <control-base
    :title="title"
    :icon="icon"
  >
    <div class="flex flex-wrap items-center justify-center gap-1.5">
      <button
        v-for="(item, index) in items"
        :key="item.id"
        type="button"
        class="flex items-center gap-1.5 rounded-full px-3 py-1.5 text-sm font-bold transition-colors"
        :class="
          item.id === currentId
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
