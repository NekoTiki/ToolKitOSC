<script setup lang="ts">
// Shared radial-dial mechanics for both ControlSlider.vue (an OSC address) and
// ControlIntifaceToy.vue (an Intiface toy's actuator value) - same drag geometry, same optimistic
// display, same debounce; only where the value actually comes from/goes to differs, which each of
// those two thin wrappers owns via this component's plain modelValue/update:modelValue v-model.
import _ from 'lodash'
import { computed, onBeforeUnmount, ref, watch } from 'vue'

const DEFAULT_DEBOUNCE_MS = 50

const props = defineProps<{
  title: string
  icon?: string
  // 0-100 range (a percentage) - both wrappers convert to/from their own 0-1 value at their own
  // boundary, kept as a percentage here purely because that's what this component's geometry and
  // displayed label already work in.
  modelValue: number
  // How often, at most, a drag actually commits an update (see `commit` below) - used as both the
  // debounce's `wait` and its `maxWait` (kept equal so this reads as "at most once per this many
  // ms", not a two-tier debounce with different leading/trailing timing).
  debounceMs?: number
}>()

const emit = defineEmits<{ (e: 'update:modelValue', value: number): void }>()

// Optimistic UI: `props.modelValue` only updates once a full round trip confirms it (viewer ->
// relay -> host -> OSC -> VRChat -> OSC out -> host -> relay -> viewer for ControlSlider; a WS
// broadcast for ControlIntifaceToy) - waiting for that before moving the knob made dragging feel
// laggy. `localValue` shows the position the user actually dragged to immediately; it's cleared
// once `modelValue` catches up and agrees with it (within rounding), or after a bounded timeout if
// that confirmation never arrives, so a dropped update can't leave the knob stuck forever.
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

const commit = _.debounce(
  (value: number) => emit('update:modelValue', value),
  commitDelay,
  { leading: true, trailing: false, maxWait: commitDelay }
)

const setValue = (value: number): void => {
  localValue.value = value

  clearReconcileTimeout()
  reconcileTimeout = setTimeout(() => {
    localValue.value = null
    reconcileTimeout = undefined
  }, RECONCILE_TIMEOUT_MS)

  commit(value)
}

const isDragging = ref(false)
const sliderRef = ref<HTMLElement>()

// A dead zone straddling the top of the dial: the 35deg either side of straight up (70deg total)
// don't move the thumb away from 0%/100% (see updateSliderValue below), so overshooting slightly
// while aiming for either end still lands exactly on it, instead of needing to hit a single-
// degree-wide point. The track itself is drawn with a matching gap (see the SVG below) so the dead
// zone is visible, not just a full circle that quietly stops responding near the top.
const DEAD_ZONE_DEG = 35
const ACTIVE_RANGE_DEG = 360 - DEAD_ZONE_DEG * 2

// Rotation that puts the SVG circle's dash-pattern start point (normally 3 o'clock) at the start
// of the active zone (DEAD_ZONE_DEG clockwise of straight up), and the arc length that covers just
// that active zone - drawing DEAD_ZONE_DEG*2 less than the full circumference leaves the gap.
const CIRCUMFERENCE = 2 * Math.PI * 35
const TRACK_ROTATION = DEAD_ZONE_DEG - 90
const ACTIVE_ARC_LENGTH = CIRCUMFERENCE * (ACTIVE_RANGE_DEG / 360)

const rotationAngle = computed(() => {
  return DEAD_ZONE_DEG + (displayValue.value / 100) * ACTIVE_RANGE_DEG - 90
})

const handleMouseDown = (event: MouseEvent): void => {
  // button !== 0: ignore right/middle-click - a right-click is meant to open this control's
  // context menu (see ControlGroup.vue), not jump the value to wherever it landed.
  if (event.button !== 0) return

  event.preventDefault()
  isDragging.value = true
  updateSliderValue(event)

  const handleMouseMove = (e: MouseEvent): void => {
    if (isDragging.value) {
      updateSliderValue(e)
    }
  }

  const handleMouseUp = (): void => {
    isDragging.value = false
    document.removeEventListener('mousemove', handleMouseMove)
    document.removeEventListener('mouseup', handleMouseUp)
  }

  document.addEventListener('mousemove', handleMouseMove)
  document.addEventListener('mouseup', handleMouseUp)
}

const updateSliderValue = (event: MouseEvent): void => {
  if (!sliderRef.value) return

  const rect = sliderRef.value.getBoundingClientRect()

  const centerX = rect.left + rect.width / 2
  const centerY = rect.top + rect.height / 2

  const deltaX = event.clientX - centerX
  const deltaY = event.clientY - centerY

  let angle = Math.atan2(deltaY, deltaX) * (180 / Math.PI)

  angle = angle + 90
  if (angle < 0) angle += 360
  if (angle >= 360) angle -= 360

  let percentage: number

  if (angle <= DEAD_ZONE_DEG) percentage = 0
  else if (angle >= 360 - DEAD_ZONE_DEG) percentage = 100
  else percentage = ((angle - DEAD_ZONE_DEG) / ACTIVE_RANGE_DEG) * 100

  setValue(Math.round(Math.max(0, Math.min(100, percentage))))
}
</script>

<template>
  <div
    ref="sliderRef"
    class="bg-default ring-default relative isolate aspect-square cursor-pointer overflow-hidden rounded-lg ring select-none"
    @mousedown="handleMouseDown"
  >
    <svg
      class="absolute inset-0 z-10 h-full w-full"
      viewBox="0 0 100 100"
    >
      <!-- Progress track background (gray, gapped at the dead zone) -->
      <circle
        cx="50"
        cy="50"
        r="35"
        fill="none"
        stroke="currentColor"
        stroke-width="10"
        class="text-primary/20"
        stroke-linecap="round"
        :stroke-dasharray="`${ACTIVE_ARC_LENGTH} ${CIRCUMFERENCE}`"
        stroke-dashoffset="0"
        :transform="`rotate(${TRACK_ROTATION} 50 50)`"
      />

      <!-- Progress track (colored, value) - a darker shade of the theme's primary color, not the
      plain 'text-primary' the knob and background track already use, so the filled arc reads as
      its own element rather than the same flat color repeated three times. -->
      <circle
        cx="50"
        cy="50"
        r="35"
        fill="none"
        stroke="currentColor"
        stroke-width="12"
        class="text-primary-300"
        stroke-linecap="round"
        :stroke-dasharray="`${(displayValue / 100) * ACTIVE_ARC_LENGTH} ${CIRCUMFERENCE}`"
        stroke-dashoffset="0"
        :transform="`rotate(${TRACK_ROTATION} 50 50)`"
      />
    </svg>

    <div class="pointer-events-none absolute inset-0 flex flex-col items-center justify-center">
      <!-- max-w-[55%]: the ring's clear inner circle (radius 35 minus half its 12-wide stroke,
      i.e. 29 units out of the SVG's 100-unit viewBox) only guarantees ~58% of the box's width at
      its exact vertical center, less again this far off-center - capping the title noticeably
      narrower than the box, rather than relying on a few pixels of padding, is what actually keeps
      a long one from visually running under the ring instead of just stopping short of the box's
      own edge. w-full alongside it: a flex-col child otherwise shrinks to its own content width,
      leaving `truncate` nothing narrower than the text itself to ever clip against. -->
      <div class="w-full max-w-[55%] truncate text-center text-2xl font-medium">
        {{ title }}
      </div>
      <div class="text-sm font-bold opacity-70">
        {{ displayValue.toFixed() }}%
      </div>
      <UIcon
        v-if="icon"
        :name="icon"
        class="absolute size-32 opacity-10"
      />
    </div>

    <!-- A light fill with a thick primary ring plus a smaller primary dot in the center, not a
    solid primary fill: matching the arc's own color made the knob blend straight into it wherever
    it sat on top of the colored portion. z-20 (above the SVG's z-10) so the whole knob - the knob
    sits exactly on the arc's own radius, overlapping its stroke band - always paints on top of the
    arc instead of mostly disappearing under it. -->
    <div
      class="bg-default ring-primary pointer-events-none absolute z-20 flex h-12 w-12 items-center justify-center rounded-full shadow-xl/50 ring-4"
      :style="{
        left: `${50 + 35 * Math.cos(((rotationAngle + 0) * Math.PI) / 180)}%`,
        top: `${50 + 35 * Math.sin(((rotationAngle + 0) * Math.PI) / 180)}%`,
        transform: 'translate(-50%, -50%)'
      }"
    >
      <div class="bg-primary h-5 w-5 rounded-full" />
    </div>
  </div>
</template>

<style scoped></style>
