<script setup lang="ts">
import _ from 'lodash'
import { computed, ref } from 'vue'

import { useIntifaceControl } from '../composables/useIntifaceControl'

// Same radial-dial widget as ControlSlider.vue, but its value comes from the shared Intiface
// control-value map (kept in sync across host/viewers via 'intiface-value-update') instead of the
// live OSC parameter cache - there's no avatar parameter backing an Intiface toy.
const props = defineProps<{
  controlId: string
  title: string
  icon?: string
}>()

const emit = defineEmits<{ (e: 'update:value', value: number): void }>()

const { controlValue } = useIntifaceControl()

const sliderValue = computed({
  get: () => (controlValue.value.get(props.controlId) ?? 0) * 100,
  set: _.debounce(
    (newValue: number) => {
      if (newValue === sliderValue.value) return

      emit('update:value', newValue / 100)
    },
    25,
    { leading: true, trailing: false, maxWait: 25 }
  )
})

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
  return DEAD_ZONE_DEG + (sliderValue.value / 100) * ACTIVE_RANGE_DEG - 90
})

const handleMouseDown = (event: MouseEvent): void => {
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

  sliderValue.value = Math.round(Math.max(0, Math.min(100, percentage)))
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
        class="text-primary-700"
        stroke-linecap="round"
        :stroke-dasharray="`${(sliderValue / 100) * ACTIVE_ARC_LENGTH} ${CIRCUMFERENCE}`"
        stroke-dashoffset="0"
        :transform="`rotate(${TRACK_ROTATION} 50 50)`"
      />
    </svg>

    <div class="pointer-events-none absolute inset-0 flex flex-col items-center justify-center">
      <div class="text-2xl font-medium">
        {{ title }}
      </div>
      <div class="text-sm font-bold opacity-70">
        {{ sliderValue.toFixed() }}%
      </div>
      <UIcon
        v-if="icon"
        :name="icon"
        class="absolute size-32 opacity-10"
      />
    </div>

    <!-- A light fill with a thick primary ring plus a smaller primary dot in the center, not a
    solid primary fill: matching the arc's own color made the knob blend straight into it wherever
    it sat on top of the colored portion. The ring currently paints under the SVG (which has its
    own stacking context above this) wherever the arc crosses it - only the inner dot is guaranteed
    to stay visible against the arc. -->
    <div
      class="bg-default ring-primary pointer-events-none absolute flex h-12 w-12 items-center justify-center rounded-full shadow-xl/50 ring-4"
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
