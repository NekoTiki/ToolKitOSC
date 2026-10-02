<script setup lang="ts">
// Shared fader mechanics for both ControlSlider.vue (an OSC address) and ControlIntifaceToy.vue (an
// Intiface toy's actuator value) - same drag behavior, same optimistic display, same debounce; only
// where the value comes from/goes to differs, which each thin wrapper owns via this component's
// plain modelValue/update:modelValue v-model.
//
// A linear fader, not a dial: dragging along a straight track is far easier to control with a
// shaky VR pointer than tracing a circle. Arrow keys nudge it by 5% for keyboard users.
import _ from 'lodash'
import { computed, onBeforeUnmount, ref, watch } from 'vue'

import ControlBase from './ControlBase.vue'

const DEFAULT_DEBOUNCE_MS = 50
const KEY_STEP = 5

const props = defineProps<{
  title: string
  icon?: string
  // 0-100 (a percentage) - both wrappers convert to/from their own 0-1 value at their boundary.
  modelValue: number
  // How often, at most, a drag commits an update (used as both the debounce's `wait` and its
  // `maxWait`, so this reads as "at most once per this many ms").
  debounceMs?: number
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

const setValue = (value: number): void => {
  const clamped = Math.round(Math.max(0, Math.min(100, value)))

  localValue.value = clamped

  clearReconcileTimeout()
  reconcileTimeout = setTimeout(() => {
    localValue.value = null
    reconcileTimeout = undefined
  }, RECONCILE_TIMEOUT_MS)

  commit(clamped)
}

const trackRef = ref<HTMLElement>()

// The knob's center travels between half the track height from each end, so the knob never sticks
// out past the track - see the template's `--k`.
const valueAt = (event: PointerEvent): number => {
  const rect = trackRef.value!.getBoundingClientRect()
  const radius = rect.height / 2

  return ((event.clientX - rect.left - radius) / Math.max(rect.width - 2 * radius, 1)) * 100
}

// Pointer capture keeps the drag going when the pointer leaves the track, and the final value is
// always committed on release (the leading-edge debounce above may have skipped the last move).
const onPointerDown = (event: PointerEvent): void => {
  // Left button only - right-click is reserved for context menus.
  if (event.button !== 0 || !trackRef.value) return

  event.preventDefault()
  trackRef.value.setPointerCapture(event.pointerId)
  setValue(valueAt(event))

  const track = trackRef.value
  const onMove = (e: PointerEvent): void => setValue(valueAt(e))
  const onUp = (e: PointerEvent): void => {
    commit.cancel()
    emit('update:modelValue', Math.round(Math.max(0, Math.min(100, valueAt(e)))))
    track.removeEventListener('pointermove', onMove)
    track.removeEventListener('pointerup', onUp)
    track.removeEventListener('pointercancel', onUp)
  }

  track.addEventListener('pointermove', onMove)
  track.addEventListener('pointerup', onUp)
  track.addEventListener('pointercancel', onUp)
}

const onKeydown = (event: KeyboardEvent): void => {
  const delta = { ArrowRight: KEY_STEP, ArrowUp: KEY_STEP, ArrowLeft: -KEY_STEP, ArrowDown: -KEY_STEP }[event.key]

  if (delta === undefined) return

  event.preventDefault()
  setValue(displayValue.value + delta)
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

    <!-- Nested like the OFF/ON switch: the fill and the knob sit 4px inside the track, with corners
    4px tighter than the track's, so every edge runs parallel. --k is the knob's size; its left edge
    travels from 4px to (100% - 4px - --k), so its center stays half a track height from each end
    (see valueAt). The fill always ends at the knob's far edge - the knob caps it, no seam. -->
    <div
      ref="trackRef"
      role="slider"
      tabindex="0"
      :aria-label="title"
      aria-valuemin="0"
      aria-valuemax="100"
      :aria-valuenow="Math.round(displayValue)"
      class="relative h-(--tile-control) cursor-grab touch-none overflow-hidden rounded-field well [--k:calc(var(--tile-control)-0.5rem)] active:cursor-grabbing"
      @pointerdown="onPointerDown"
      @keydown="onKeydown"
      @click.stop
    >
      <div
        class="absolute inset-y-1 left-1 rounded-[calc(var(--radius-field)-0.25rem)] bg-linear-to-r from-secondary/30 to-secondary/85"
        :style="{ width: `calc(var(--k) + (100% - 0.5rem - var(--k)) * ${displayValue / 100})` }"
      />
      <div
        class="pointer-events-none absolute top-1 flex size-(--k) items-center justify-center gap-1 rounded-[calc(var(--radius-field)-0.25rem)] bg-white shadow-[0_2px_10px_rgb(0_0_0/0.35)]"
        :style="{ left: `calc(0.25rem + (100% - 0.5rem - var(--k)) * ${displayValue / 100})` }"
      >
        <!-- Grip lines: says "drag me" without relying on a hover state. -->
        <span class="h-[34%] w-0.5 rounded-full bg-black/20" />
        <span class="h-[34%] w-0.5 rounded-full bg-black/20" />
      </div>
    </div>
  </control-base>
</template>
