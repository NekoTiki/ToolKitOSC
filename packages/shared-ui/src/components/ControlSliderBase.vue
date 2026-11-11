<script setup lang="ts">
// Shared fader mechanics for both ControlSlider.vue (an OSC address) and ControlIntifaceToy.vue (an
// Intiface toy's actuator value) - same drag behavior, same optimistic display, same debounce; only
// where the value comes from/goes to differs, which each thin wrapper owns via this component's
// plain modelValue/update:modelValue v-model.
//
// The fader itself is Fader.vue; this adds the optimistic display and the debounced commits.
import _ from 'lodash'
import { computed, onBeforeUnmount, ref, watch } from 'vue'

import ControlBase from './ControlBase.vue'
import Fader from './Fader.vue'

const DEFAULT_DEBOUNCE_MS = 50

const props = defineProps<{
  title: string
  icon?: string
  // 0-100 (a percentage) - both wrappers convert to/from their own 0-1 value at their boundary.
  modelValue: number
  // How often, at most, a drag commits an update (used as both the debounce's `wait` and its
  // `maxWait`, so this reads as "at most once per this many ms").
  debounceMs?: number
  // The range the active profile allows viewers, on the same 0-100 scale. The track still reads
  // 0-100 so the number shown is the real value; the parts outside the range are shaded and the
  // knob can't enter them.
  min?: number
  max?: number
}>()

const emit = defineEmits<{ (e: 'update:modelValue', value: number): void }>()

// Optimistic UI: `props.modelValue` only updates once a full round trip confirms it (viewer ->
// relay -> host -> OSC -> VRChat -> OSC out -> host -> relay -> viewer for ControlSlider; a WS
// broadcast for ControlIntifaceToy) - waiting for that made dragging feel laggy. `localValue` shows
// where the user dragged to immediately; it's cleared once `modelValue` catches up (within
// rounding), or after a bounded timeout if that confirmation never arrives, so a dropped update
// can't leave the thumb stuck.
const RECONCILE_TIMEOUT_MS = 2000

const localValue = ref<number | null>(null)
let reconcileTimeout: ReturnType<typeof setTimeout> | undefined

const displayValue = computed(() => localValue.value ?? props.modelValue)

const clearReconcileTimeout = (): void => {
  if (reconcileTimeout) clearTimeout(reconcileTimeout)
  reconcileTimeout = undefined
}

watch(
  () => props.modelValue,
  (confirmed) => {
    if (localValue.value !== null && Math.round(confirmed) === Math.round(localValue.value)) {
      localValue.value = null
      clearReconcileTimeout()
    }
  }
)

onBeforeUnmount(clearReconcileTimeout)

const commitDelay = props.debounceMs ?? DEFAULT_DEBOUNCE_MS

const commit = _.debounce((value: number) => emit('update:modelValue', value), commitDelay, {
  leading: true,
  trailing: false,
  maxWait: commitDelay
})

// The fader already keeps the value within the range.
const setValue = (value: number): void => {
  localValue.value = value

  clearReconcileTimeout()
  reconcileTimeout = setTimeout(() => {
    localValue.value = null
    reconcileTimeout = undefined
  }, RECONCILE_TIMEOUT_MS)

  commit(value)
}

// The final value is always committed on release (the leading-edge debounce above may have skipped
// the last move).
const onChange = (value: number): void => {
  commit.cancel()
  emit('update:modelValue', value)
}
</script>

<template>
  <control-base
    :title="title"
    :icon="icon"
    kind="value"
  >
    <template #middle>
      <span class="text-(length:--tile-value) leading-none font-semibold text-secondary tabular-nums">
        {{ displayValue.toFixed() }}<span class="text-[0.55em] opacity-70">%</span>
      </span>
    </template>

    <fader
      :model-value="displayValue"
      :label="title"
      :range-min="min"
      :range-max="max"
      @update:model-value="setValue"
      @change="onChange"
    />
  </control-base>
</template>
