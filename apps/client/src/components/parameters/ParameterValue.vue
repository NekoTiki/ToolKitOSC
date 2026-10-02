<script setup lang="ts">
import NumberBar from '@renderer/components/parameters/NumberBar.vue'
import { useOscMessages } from '@renderer/composables/useOscMessages'
import type { OSCArg } from '@renderer/env'
import { computed } from 'vue'

// One parameter's live value on the Parameters page. It reads the OSC state itself, so a value
// changing re-renders just this cell - not the page's whole row list, which face tracking would
// otherwise rebuild dozens of times a second.
const props = defineProps<{ address: string; kind?: 'Bool' | 'Float' | 'Int' }>()

const { args } = useOscMessages()

const value = computed<OSCArg | undefined>(() => args.value[props.address]?.[0])
const received = computed(() => props.address in args.value)
</script>

<template>
  <span
    v-if="!received"
    class="text-sm text-dimmed"
  >Not received</span>
  <span
    v-else-if="typeof value === 'boolean'"
    class="w-fit rounded-full px-2.5 py-0.5 text-xs font-medium"
    :class="value ? 'bg-primary/16 text-primary' : 'bg-(--aurora-well) text-muted'"
  >{{ value ? 'True' : 'False' }}</span>
  <NumberBar
    v-else-if="typeof value === 'number'"
    :value="value"
    :kind="kind"
  />
  <span
    v-else
    class="truncate font-mono text-sm"
  >{{ String(value) }}</span>
</template>
